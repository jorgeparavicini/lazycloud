mod iam_policy;
mod labels;
mod overlay;
mod payload;
mod replication;
mod secrets;
mod versions;

pub use iam_policy::IamPolicyView;
pub use labels::LabelsView;
pub use overlay::{
    CreateSecretNameOverlay, CreateSecretPayloadOverlay, CreateVersionOverlay,
    DeleteSecretOverlay, DestroyVersionOverlay,
};
pub use payload::PayloadView;
pub use replication::ReplicationView;
pub use secrets::SecretListView;
pub use versions::VersionListView;

use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::view::View;

/// A view within the Secret Manager service that provides breadcrumb context.
pub trait SecretManagerView: View<Event = SecretManagerMsg> {
    fn breadcrumbs(&self) -> Vec<String>;

    /// Return the message needed to reload this view's data.
    fn reload(&self) -> SecretManagerMsg;
}
