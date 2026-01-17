# CLAUDE.md

## Build Commands

```bash
cargo build            # Build debug
cargo build --release  # Build release
cargo run              # Run the application
cargo test             # Run tests
cargo fmt              # Format code
cargo clippy           # Run linter
```

## Architecture

Lazycloud is a TUI for managing cloud resources. It uses Ratatui for rendering and Tokio for async.

### App States

The app transitions through three phases:

1. **Context Selection** - Pick a cloud context (GCP project, AWS account)
2. **Service Selection** - Pick a service within that context
3. **Active Service** - Service runs as self-contained mini-app

### Directory Structure

```
src/
├── app.rs                      # App state machine
├── main.rs                     # Entry point
├── ui/                         # UI trait definitions
│   ├── component.rs            # Component trait
│   ├── screen.rs               # Screen trait
│   └── modal.rs                # Modal trait
├── component/                  # Reusable UI components
│   ├── table.rs                # TableComponent
│   ├── list.rs                 # ListComponent
│   ├── text_input.rs           # TextInputComponent
│   └── confirm_dialog.rs       # ConfirmDialogComponent
├── core/                       # Framework
│   ├── service.rs              # Service trait
│   ├── command.rs              # Async command trait
│   ├── event.rs                # Input events
│   └── tui.rs                  # Terminal handling
├── registry/                   # Service discovery
└── provider/                   # Cloud implementations
    └── gcp/
        └── secret_manager/     # Example service
            ├── service.rs      # SecretManager (implements Service)
            ├── secrets.rs      # Screens and modals for secrets
            ├── versions.rs     # Screens and modals for versions
            ├── payload.rs      # PayloadScreen
            └── client.rs       # GCP API client
```

## UI Trait Hierarchy

Three traits define UI behavior. Choose based on what you're building:

### Component

Reusable, interactive building blocks. Know nothing about business logic.

```rust
pub trait Component {
    type Output;  // What this component emits (e.g., TableEvent<T>)

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Output>>;
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn on_tick(&mut self) {}
}
```

**Use for:** Generic widgets reusable across services.

**Examples:** `TableComponent<T>`, `ListComponent<T>`, `TextInputComponent`, `ConfirmDialogComponent`

### Screen

Full-page views that orchestrate components. Know about the domain.

```rust
pub trait Screen {
    type Msg;  // Service message type (e.g., SecretManagerMsg)

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>>;
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn on_tick(&mut self) {}
    fn breadcrumbs(&self) -> Vec<String> { vec![] }
}
```

**Use for:** Main views within a service that show data and handle navigation.

**Examples:** `SecretListScreen`, `VersionListScreen`, `PayloadScreen`, `LabelsScreen`

### Modal

Ephemeral overlays that capture all input until dismissed.

```rust
pub trait Modal {
    type Msg;  // Service message type

    fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>>;
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme);
    fn title(&self) -> Option<&str> { None }
}
```

**Use for:** Confirmations, input dialogs, wizards.

**Examples:** `DeleteSecretDialog`, `CreateSecretWizard`, `CreateVersionDialog`

### Handled Enum

All `handle_key` methods return `Result<Handled<T>>`:

```rust
pub enum Handled<E> {
    Ignored,   // Didn't handle this key
    Consumed,  // Handled but no message
    Event(E),  // Handled and produced a message
}
```

Use `HandledResultExt::process()` to simplify handling:

```rust
let (consumed, msg) = screen.handle_key(key).process();
if let Some(msg) = msg {
    self.queue(msg);
}
if consumed {
    return true;
}
```

## Service Structure

Each cloud service implements the `Service` trait and manages its own:
- Screen stack for navigation
- Modal overlay
- Message queue for async results
- Caching

### Core Pattern

```rust
pub struct MyService {
    screen_stack: Vec<Box<dyn Screen<Msg = MyMsg>>>,
    modal: Option<Box<dyn Modal<Msg = MyMsg>>>,
    loading: Option<&'static str>,
    msg_tx: UnboundedSender<MyMsg>,
    msg_rx: UnboundedReceiver<MyMsg>,
    // ... caches, client, etc.
}
```

### Message Flow

1. User input → `handle_input()` → delegates to modal/screen
2. Screen returns `Handled::Event(msg)` → queued via `msg_tx`
3. `update()` drains `msg_rx` → calls `process_message()`
4. `process_message()` dispatches to feature modules → returns `UpdateResult`
5. `UpdateResult::Commands(...)` → spawned as async tasks
6. Async task completes → sends result message back to `msg_tx`

### Feature Modules

Split large services into feature modules (secrets.rs, versions.rs, payload.rs):

```rust
// In secrets.rs
pub fn update(state: &mut SecretManager, msg: SecretsMsg) -> Result<UpdateResult> {
    match msg {
        SecretsMsg::Load => { /* ... */ }
        SecretsMsg::Delete(secret) => { /* ... */ }
    }
}
```

Dispatch from main service:

```rust
fn process_message(&mut self, msg: MyMsg) -> Result<UpdateResult> {
    match msg {
        MyMsg::Secret(m) => secrets::update(self, m),
        MyMsg::Version(m) => versions::update(self, m),
    }
}
```

## Code Style

### No Useless Comments

Don't write comments that just repeat the code:

```rust
// BAD
/// Delete a secret
Delete(Secret),

/// Load secrets
Load,

// GOOD - no comment needed, the code is self-documenting
Delete(Secret),
Load,

// GOOD - comment adds useful context
/// Destroys version permanently. Cannot be undone.
Destroy { secret: Secret, version: SecretVersion },
```

### Doc Comments

Only add doc comments when they provide information not obvious from the name:
- Complex invariants
- Non-obvious behavior
- Important warnings
- Usage examples for public APIs

### Error Handling

Use `?` for error propagation. The `Result<Handled<T>>` return type enables this:

```rust
fn handle_key(&mut self, key: KeyEvent) -> Result<Handled<Self::Msg>> {
    let result = self.table.handle_key(key)?;  // Propagate errors
    // ...
}
```

## Adding a New Service

1. Create `src/provider/<provider>/<service>/`
2. Define message enum with `From` impls for `Handled<ServiceMsg>`
3. Create screens implementing `Screen<Msg = ServiceMsg>`
4. Create modals implementing `Modal<Msg = ServiceMsg>`
5. Implement `Service` trait in service.rs
6. Implement `ServiceProvider` for the factory
7. Register in the provider's mod.rs
