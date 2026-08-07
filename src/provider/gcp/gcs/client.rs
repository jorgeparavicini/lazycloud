use google_cloud_gax::client_builder;
use google_cloud_gax::error::rpc::Code;
use google_cloud_gax::paginator::{ItemPaginator, Paginator};
use google_cloud_storage::client::{Storage, StorageControl};
use tracing::{debug, info};

use crate::context::{CredentialError, GcpContext};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("GCS API is not enabled for this project")]
    ApiDisabled,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("{0}")]
    CredentialsError(#[from] CredentialError),

    #[error("Failed to create GCS client: {0}")]
    ClientCreationError(client_builder::Error),

    #[error("GCS API error: {code:?} - {message}")]
    ApiError { code: Code, message: String },

    #[error("{0}")]
    Other(String),
}

impl From<google_cloud_gax::error::Error> for ClientError {
    fn from(err: google_cloud_gax::error::Error) -> Self {
        if let Some(status) = err.status() {
            if status.code == Code::PermissionDenied {
                if status.message.contains("has not been used")
                    || status.message.contains("is disabled")
                {
                    return Self::ApiDisabled;
                }
                return Self::PermissionDenied(status.message.clone());
            }

            Self::ApiError {
                code: status.code,
                message: status.message.clone(),
            }
        } else {
            Self::Other(format!("Unexpected error: {err}"))
        }
    }
}

/// Page size requested for list calls. GCS caps both bucket and object listings
/// at 1000 entries per page, and defaults to a much smaller page when unset.
const LIST_PAGE_SIZE: i32 = 1000;

pub struct BucketInfo {
    pub name: String,
    pub location: String,
    pub storage_class: String,
}

#[derive(Debug, Clone)]
pub struct ObjectList {
    pub folders: Vec<String>,
    pub objects: Vec<ObjectInfo>,
}

#[derive(Debug, Clone)]
pub struct ObjectInfo {
    pub name: String,
    pub full_name: String,
    pub size: i64,
    pub content_type: String,
    pub updated: String,
    pub storage_class: String,
}

#[derive(Clone, Debug)]
pub struct GcsClient {
    control: StorageControl,
    storage: Storage,
    parent: String,
}

impl GcsClient {
    pub async fn new(context: &GcpContext) -> Result<Self, ClientError> {
        info!(
            project = %context.project_id,
            account = %context.account,
            "Creating GCS credentials"
        );
        // Build and validate the credentials once, then share the clone across
        // both clients (Credentials is Arc-backed) to avoid a second preflight.
        let credentials = context.create_credentials().await?;

        debug!("Credentials validated; building StorageControl client");
        let control = StorageControl::builder()
            .with_credentials(credentials.clone())
            .build()
            .await
            .map_err(ClientError::ClientCreationError)?;

        debug!("Building Storage client");
        let storage = Storage::builder()
            .with_credentials(credentials)
            .build()
            .await
            .map_err(|e| ClientError::Other(format!("Failed to create Storage client: {e}")))?;

        info!("GCS clients ready");
        Ok(Self {
            control,
            storage,
            parent: format!("projects/{}", context.project_id),
        })
    }

    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>, ClientError> {
        info!(parent = %self.parent, "Listing buckets");
        let mut items = self
            .control
            .list_buckets()
            .set_parent(&self.parent)
            .set_page_size(LIST_PAGE_SIZE)
            .by_item();

        let mut buckets = Vec::new();
        while let Some(bucket) = items.next().await {
            let bucket = bucket?;
            let name = bucket
                .name
                .rsplit_once('/')
                .map_or_else(|| bucket.name.clone(), |(_, n)| n.to_string());
            buckets.push(BucketInfo {
                name,
                location: bucket.location,
                storage_class: bucket.storage_class,
            });
        }
        debug!(count = buckets.len(), "Received buckets");
        Ok(buckets)
    }

    pub(super) async fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<ObjectList, ClientError> {
        let bucket_path = format!("projects/_/buckets/{bucket}");
        let mut builder = self
            .control
            .list_objects()
            .set_parent(&bucket_path)
            .set_delimiter("/")
            .set_page_size(LIST_PAGE_SIZE);

        if !prefix.is_empty() {
            builder = builder.set_prefix(prefix);
        }

        // Iterate by page rather than by item: prefixes (the synthetic folders)
        // live on the page, not on the item stream.
        let mut pages = builder.by_page();
        let mut folders = Vec::new();
        let mut objects = Vec::new();

        while let Some(page) = pages.next().await {
            let page = page?;
            folders.extend(page.prefixes);
            objects.extend(page.objects.into_iter().map(|obj| {
                let display_name = if let Some(stripped) = obj.name.strip_prefix(prefix) {
                    stripped.to_string()
                } else {
                    obj.name.rsplit('/').next().unwrap_or(&obj.name).to_string()
                };
                let updated = obj.update_time.map(String::from).unwrap_or_default();
                ObjectInfo {
                    name: display_name,
                    full_name: obj.name,
                    size: obj.size,
                    content_type: obj.content_type,
                    updated,
                    storage_class: obj.storage_class,
                }
            }));
        }

        debug!(
            folders = folders.len(),
            objects = objects.len(),
            "Received objects"
        );
        Ok(ObjectList { folders, objects })
    }

    pub(super) async fn read_object(
        &self,
        bucket: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, ClientError> {
        let bucket_path = format!("projects/_/buckets/{bucket}");
        let mut response = self
            .storage
            .read_object(&bucket_path, object_name)
            .send()
            .await
            .map_err(|e| ClientError::Other(format!("Failed to read object: {e}")))?;

        let max_bytes = 64 * 1024;
        let mut contents = Vec::new();
        while let Some(chunk) = response.next().await {
            match chunk {
                Ok(bytes) => {
                    contents.extend_from_slice(&bytes);
                    if contents.len() >= max_bytes {
                        contents.truncate(max_bytes);
                        break;
                    }
                }
                Err(e) => {
                    return Err(ClientError::Other(format!(
                        "Failed to read object stream: {e}"
                    )));
                }
            }
        }

        Ok(contents)
    }
}
