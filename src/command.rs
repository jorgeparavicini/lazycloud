use serde::{Deserialize, Serialize};
use crate::components::services::Service;
use crate::context::Context;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
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
    SelectService(Service)
}
