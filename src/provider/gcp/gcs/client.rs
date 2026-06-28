use crate::context::GcpContext;
use google_cloud_auth::build_errors;
use google_cloud_gax::client_builder;
use google_cloud_gax::error::rpc::Code;
use google_cloud_storage::client::{Storage, StorageControl};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("GCS API is not enabled for this project")]
    ApiDisabled,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Failed to create credentials: {0}")]
    CredentialsError(build_errors::Error),

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
        let control_creds = context
            .create_credentials()
            .map_err(ClientError::CredentialsError)?;
        let storage_creds = context
            .create_credentials()
            .map_err(ClientError::CredentialsError)?;

        let control = StorageControl::builder()
            .with_credentials(control_creds)
            .build()
            .await
            .map_err(ClientError::ClientCreationError)?;

        let storage = Storage::builder()
            .with_credentials(storage_creds)
            .build()
            .await
            .map_err(|e| ClientError::Other(format!("Failed to create Storage client: {e}")))?;

        Ok(Self {
            control,
            storage,
            parent: format!("projects/{}", context.project_id),
        })
    }

    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>, ClientError> {
        let response = self
            .control
            .list_buckets()
            .set_parent(&self.parent)
            .send()
            .await?;

        let buckets = response
            .buckets
            .into_iter()
            .map(|bucket| {
                let name = bucket
                    .name
                    .rsplit_once('/')
                    .map_or_else(|| bucket.name.clone(), |(_, n)| n.to_string());
                BucketInfo {
                    name,
                    location: bucket.location,
                    storage_class: bucket.storage_class,
                }
            })
            .collect::<Vec<_>>();
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
            .set_delimiter("/");

        if !prefix.is_empty() {
            builder = builder.set_prefix(prefix);
        }

        let response = builder.send().await?;

        let folders = response.prefixes;

        let objects = response
            .objects
            .into_iter()
            .map(|obj| {
                let display_name = if let Some(stripped) = obj.name.strip_prefix(prefix) {
                    stripped.to_string()
                } else {
                    obj.name.rsplit('/').next().unwrap_or(&obj.name).to_string()
                };
                let updated = obj
                    .update_time
                    .map(String::from)
                    .unwrap_or_default();
                ObjectInfo {
                    name: display_name,
                    full_name: obj.name,
                    size: obj.size,
                    content_type: obj.content_type,
                    updated,
                    storage_class: obj.storage_class,
                }
            })
            .collect();

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
