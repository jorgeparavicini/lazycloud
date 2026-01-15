
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
# Build the project
cargo build

# Build release
cargo build --release

# Run the application
cargo run

# Run tests
cargo test

# Run a specific test
cargo test <test_name>

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Architecture Overview

Lazycloud is a Terminal User Interface (TUI) for managing cloud resources across multiple providers (GCP, AWS, Azure). It uses Ratatui for rendering and Tokio for async operations.

### Core Design: Services as Mini-Applications

Each cloud service (e.g., GCP Secret Manager) is a **self-contained mini-application** that owns its internal state, navigation, views, and async operations. The app only manages phase transitions between:

1. **Context Selection** - Choose a cloud context (GCP project, AWS account)
2. **Service Selection** - Choose a service within that context
3. **Active Service** - The service runs as a self-contained mini-app

```rust
enum AppPhase {
    SelectingContext(ContextSelector),   // Simple widget
    SelectingService(ServiceSelector),   // Simple widget
    ActiveService(Box<dyn Service>),     // Self-contained mini-app
}
```

### Key Traits

**`Service` trait** (`src/core/service.rs`) - The only trait for cloud services:
- `on_mount()` / `on_unmount()` - Lifecycle hooks
- `handle_event()` → `ServiceResult` - Event handling
- `render()` - UI rendering
- Services return `ServiceResult::CloseService` to exit back to service selector

**`ServiceProvider` trait** (`src/registry/service_provider.rs`) - Factory for services:
- `create_service(ctx: &CloudContext) -> Box<dyn Service>`
- Registered in `ServiceRegistry` for discovery

### Directory Structure

```
src/
├── main.rs              # Entry point, registry initialization
├── app.rs               # AppPhase state machine, event routing
├── core/                # Framework (Event, AppMessage, Service, Tui)
├── model/               # Domain models (Provider, CloudContext)
├── registry/            # Service discovery (ServiceId, ServiceProvider, ServiceRegistry)
├── widget/              # Reusable UI components (SelectList, Spinner, StatusBar)
├── screen/              # Simple selection widgets (ContextSelector, ServiceSelector)
└── provider/            # Cloud service implementations
    └── gcp/
        └── secret_manager/  # Self-contained service mini-app
            ├── screen.rs    # Implements Service trait
            ├── message.rs   # Local message enum (SecretManagerMsg)
            ├── view/        # Internal views (SecretListView, etc.)
            └── command.rs   # Async commands
```

### Event Flow

```
TUI Event
  → App::handle_events()
    → Match on AppPhase
      → SelectingContext/Service: widget.handle_key_event() → Option<Selection>
      → ActiveService: service.handle_event() → ServiceResult
    → Process result (phase transition, send message, etc.)
```

### Local Message Channels

Each service has its own internal message channel for async operations:
```rust
struct SecretManagerScreen {
    msg_tx: UnboundedSender<SecretManagerMsg>,  // Commands send results here
    msg_rx: UnboundedReceiver<SecretManagerMsg>, // Polled in handle_tick()
}
```

This keeps service-specific messages out of `AppMessage`, which only handles phase transitions.

### Adding a New Cloud Service

1. Create directory: `src/provider/<provider>/<service>/`
2. Implement `Service` trait in `screen.rs`
3. Define local message enum in `message.rs`
4. Create views implementing your view trait
5. Implement `ServiceProvider` in parent `mod.rs`
6. Register in `src/provider/<provider>/mod.rs`

### Key Terminology

- **Event**: Input from terminal (keyboard, mouse, tick)
- **AppMessage**: App-level messages for phase transitions only
- **ServiceResult**: Result of service handling an event (Handled, CloseService, Error, Ignored)
- **Service**: Self-contained cloud service mini-application
- **View**: UI component within a service's internal state machine

## Rust Edition

This project uses Rust Edition 2024 with resolver version 3.
