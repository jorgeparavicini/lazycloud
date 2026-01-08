mod payload;
mod secrets;
mod versions;

pub use payload::PayloadView;
pub use secrets::SecretListView;
pub use versions::VersionListView;

use crate::provider::gcp::secret_manager::message::SecretManagerMsg;
use crate::view::View;

/// A view within the Secret Manager service that provides breadcrumb context.
pub trait ServiceView: View<Event = SecretManagerMsg> {
    fn breadcrumbs(&self) -> Vec<String>;

    /// Return the message needed to reload this view's data.
    fn reload(&self) -> SecretManagerMsg;
}
