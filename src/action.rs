use crate::components::services::Service;
use crate::context::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}
