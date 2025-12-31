use crate::components::services::gcp::secret_manager::SecretManagerAction;

pub mod secret_manager;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcpAction {
    SecretManagerAction(SecretManagerAction),
}
