//! Reusable UI components for the TUI.
//!
//! Components are interactive UI building blocks that handle key events and emit
//! generic outputs. They know nothing about business logic.

mod command_status;
mod confirm_dialog;
mod context_selector;
mod error_dialog;
mod help;
mod list;
mod service_selector;
mod spinner;
mod status_bar;
mod table;
mod text_input;
mod theme_selector;

pub use command_status::{CommandId, CommandStatusView};
pub use confirm_dialog::{ConfirmDialogComponent, ConfirmEvent, ConfirmStyle};
pub use error_dialog::{ErrorDialog, ErrorDialogEvent};
pub use context_selector::ContextSelectorView;
pub use help::{HelpEvent, HelpView, Keybinding, KeybindingSection};
pub use list::{ListComponent, ListEvent, ListRow};
pub use service_selector::ServiceSelectorView;
pub use spinner::SpinnerWidget;
pub use status_bar::StatusBarView;
pub use table::{ColumnDef, TableComponent, TableEvent, TableRow};
pub use text_input::{TextInputComponent, TextInputEvent};
pub use theme_selector::{ThemeEvent, ThemeSelectorView};
