use crate::context::GcpContext;
use google_cloud_auth::build_errors;
use google_cloud_gax::client_builder;
use google_cloud_gax::error::rpc::Code;
use google_cloud_storage::client::StorageControl;

#[derive(Debug, thiserror::Error)]
pub(super) enum ClientError {
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

#[derive(Clone, Debug)]
pub(super) struct GcsClient {
    client: StorageControl,
    parent: String,
}

impl GcsClient {
    pub async fn new(context: &GcpContext) -> Result<Self, ClientError> {
        let credentials = context
            .create_credentials()
            .map_err(ClientError::CredentialsError)?;

        let client = StorageControl::builder()
            .with_credentials(credentials)
            .build()
            .await
            .map_err(ClientError::ClientCreationError)?;

        Ok(Self {
            client,
            parent: format!("projects/{}", context.project_id),
        })
    }

    pub async fn list_buckets(&self) -> Result<Vec<String>, ClientError> {
        let response = self
            .client
            .list_buckets()
            .set_parent(&self.parent)
            .send()
            .await?;

        let buckets = response
            .buckets
            .into_iter()
            .map(|bucket| bucket.name)
            .collect::<Vec<_>>();
        Ok(buckets)
    }
}
