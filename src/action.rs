use crate::components::services::gcp::GcpAction;
use crate::components::services::Service;
use crate::context::Context;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    DisplayError(String),
    DisplayHelp,

    SelectContext(Context),
    SelectService(Service),

    Gcp(GcpAction),

    // Navigation
    Pop,
}
