|Category |Suffix           |Responsibility                                                                                                  |State?   |Example                               |
|---------|-----------------|----------------------------------------------------------------------------------------------------------------|---------|--------------------------------------|
|Widget   |*Widget          |Pure Rendering. Stateless. Takes data and draws pixels. It implements ratatui::widgets::Widget.                 |No       |SpinnerWidget, StatusLineWidget       |
|Component|*Component       |Interactive UI Building Block. Reusable. Handles its own key events (Up/Down/Type). Generic (no business logic).|UI Only  |TextInputComponent, TableComponent    |
|Screen   |*Screen          |Full Page. Orchestrates Components. Connects UI events to Business Messages. Knows about "Secrets" or "VMs".    |Business |SecretListScreen, PayloadScreen       |
|Modal    |*Dialog / *Wizard|Ephemeral Overlay. Blocks the screen below it. Used for a specific task.                                        |Transient|DeleteSecretDialog, CreateSecretWizard|

```rust
pub trait Component {
type Output; // e.g., String, usize, etc.

    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Output>;
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
}
```

```rust
pub trait Screen {
// A Screen MUST produce App/Service messages
type Msg;

    // It handles keys and translates them to Business Intent
    fn handle_key(&mut self, key: KeyEvent) -> KeyResult<Self::Msg>;

    // It renders itself (and its child components)
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);

    // Feature: Screens usually need breadcrumbs
    fn breadcrumbs(&self) -> Vec<String>;
}
```

