//! Generic harness for GCP service screens.
//!
//! Every GCP service (Secret Manager, Cloud Storage, …) shares the exact same
//! lifecycle: connect a client, prompt to enable the API if it is disabled,
//! drive an internal message queue, and translate the result into [`ServiceMsg`]
//! for the App. That machinery lives here *once*. A concrete service only has to
//! implement [`GcpServiceLogic`] — its client, its domain messages, and what to
//! do with them — and gets the rest for free.
//!
//! This is the seam the rest of the provider code depends on: feature slices
//! talk to [`GcpService`] (via the per-service type alias) and never re-implement
//! connection/enablement plumbing.

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use crossterm::event::KeyEvent;
use google_cloud_api_serviceusage_v1::client::ServiceUsage;
use google_cloud_lro::Poller;
use ratatui::Frame;
use ratatui::layout::Rect;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};

use crate::Theme;
use crate::cache::Cache;
use crate::commands::{Command, CommandCtx};
use crate::context::{CloudContext, GcpContext};
use crate::event_queue::EventQueue;
use crate::provider::Provider;
use crate::registry::ServiceProvider;
use crate::service::{Service, ServiceMsg};
use crate::ui::{Component, ConfirmDialog, ConfirmEvent, EventResult, Keybinding, Modal};
use crate::view_stack::ViewStack;

// === Lifecycle messages ===

/// Provider-agnostic lifecycle messages handled entirely by [`GcpService`].
///
/// Concrete services never match on these — the harness owns them — but they do
/// emit them (`ApiDisabled` from a command that hit a disabled API, `Connected`
/// once a client is built, etc.) via the generic `From` conversions below.
pub enum Lifecycle<C> {
    /// Begin (or restart) client initialization.
    Initialize,
    /// A client was successfully built.
    Connected(C),
    /// The backing API is disabled; offer to enable it.
    ApiDisabled,
    /// User confirmed enabling the API.
    EnableApi,
    /// The API finished enabling; re-initialize.
    ApiEnabled,
    /// Pop a screen, or close the service if at the root.
    NavigateBack,
    /// A modal was dismissed without a choice.
    DialogCancelled,
}

impl<C: Clone> Clone for Lifecycle<C> {
    fn clone(&self) -> Self {
        match self {
            Self::Initialize => Self::Initialize,
            Self::Connected(c) => Self::Connected(c.clone()),
            Self::ApiDisabled => Self::ApiDisabled,
            Self::EnableApi => Self::EnableApi,
            Self::ApiEnabled => Self::ApiEnabled,
            Self::NavigateBack => Self::NavigateBack,
            Self::DialogCancelled => Self::DialogCancelled,
        }
    }
}

/// The message type that flows through a [`GcpService`]'s queue and view stack:
/// either a shared lifecycle message or a service-specific domain message.
///
/// Each service aliases this (e.g. `pub type GcsMsg = HostMsg<GcsLogic>`) and
/// provides `From<DomainLeaf>` conversions so feature slices keep writing
/// `SomeMsg::Variant.into()`.
pub enum HostMsg<L: GcpServiceLogic> {
    /// A lifecycle message handled by the harness.
    Lifecycle(Lifecycle<L::Client>),
    /// A service-specific message handled by [`GcpServiceLogic::update`].
    Domain(L::Domain),
}

impl<L: GcpServiceLogic> Clone for HostMsg<L>
where
    L::Client: Clone,
    L::Domain: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Lifecycle(l) => Self::Lifecycle(l.clone()),
            Self::Domain(d) => Self::Domain(d.clone()),
        }
    }
}

impl<L: GcpServiceLogic> From<Lifecycle<L::Client>> for HostMsg<L> {
    fn from(msg: Lifecycle<L::Client>) -> Self {
        Self::Lifecycle(msg)
    }
}

impl<L: GcpServiceLogic> From<Lifecycle<L::Client>> for EventResult<HostMsg<L>> {
    fn from(msg: Lifecycle<L::Client>) -> Self {
        Self::Event(HostMsg::Lifecycle(msg))
    }
}

// === Logic trait ===

/// Failure modes a [`GcpServiceLogic::connect`] implementation can report.
pub enum ConnectError {
    /// The backing API is not enabled for the project.
    ApiDisabled,
    /// Any other failure; surfaced to the user as an error.
    Other(color_eyre::Report),
}

/// The per-service contract. Implement this and the harness supplies the rest.
///
/// `Domain` is the service's own message enum (e.g. dispatching to feature
/// slices); `Client` is the API client the harness builds and stores.
#[async_trait]
pub trait GcpServiceLogic: Sized + 'static {
    /// Service-specific messages handled by [`Self::update`].
    type Domain: Send + Sync + 'static;
    /// API client this service drives.
    type Client: Clone + Send + Sync + 'static;

    /// Stable key used in [`crate::registry::ServiceId`] (e.g. `"gcs"`).
    const SERVICE_KEY: &'static str;
    /// Human-readable name shown in the service list and breadcrumbs.
    const DISPLAY_NAME: &'static str;
    /// One-line description for the service list.
    const DESCRIPTION: &'static str;
    /// Fully-qualified service usage name, e.g. `"storage.googleapis.com"`.
    const API_SERVICE: &'static str;
    /// Optional icon/emoji shown next to the name.
    const ICON: Option<&'static str> = None;

    /// Create the per-service domain state (everything not owned by the host).
    fn new() -> Self;

    /// Build the API client, or report why it could not be built.
    async fn connect(context: &GcpContext) -> Result<Self::Client, ConnectError>;

    /// Queue the first message(s) once the client is ready (e.g. load a list).
    fn on_connected(host: &mut GcpService<Self>);

    /// Handle a service-specific message.
    ///
    /// # Errors
    /// Returns an error if processing fails; the App surfaces it.
    fn update(host: &mut GcpService<Self>, msg: Self::Domain) -> Result<ServiceMsg>;

    /// Called after an external editor session completes. Defaults to a no-op.
    fn handle_editor_result(host: &mut GcpService<Self>, new_content: Option<String>) {
        let _ = (host, new_content);
    }
}

// === Host ===

/// Owns the state and lifecycle shared by every GCP service.
///
/// Feature slices receive `&mut GcpService<L>` (through the per-service alias)
/// and use [`Self::views`], [`Self::cache`], [`Self::logic`] plus the helpers
/// below. They never touch connection or API-enablement concerns.
pub struct GcpService<L: GcpServiceLogic> {
    context: GcpContext,
    client: Option<L::Client>,
    events: EventQueue<HostMsg<L>>,
    /// Navigation stack of screens.
    pub(crate) views: ViewStack<HostMsg<L>>,
    /// Per-service response cache.
    pub(crate) cache: Cache,
    /// Per-service domain state.
    pub(crate) logic: L,
}

impl<L: GcpServiceLogic> GcpService<L> {
    /// Create a new service host for the given GCP context.
    pub fn new(context: GcpContext) -> Self {
        let mut views = ViewStack::new();
        views.set_loading("Initializing...");
        Self {
            context,
            client: None,
            events: EventQueue::new(),
            views,
            cache: Cache::new(),
            logic: L::new(),
        }
    }

    // === Helpers for feature slices ===

    /// Enqueue a message for processing on the next `update`.
    pub(crate) fn queue(&self, msg: HostMsg<L>) {
        self.events.send(msg);
    }

    /// Clone the message sender (for handing to async commands).
    pub(crate) fn clone_sender(&self) -> UnboundedSender<HostMsg<L>> {
        self.events.clone_sender()
    }

    /// Get a clone of the API client, erroring if not yet connected.
    ///
    /// # Errors
    /// Returns an error if the client has not been initialized.
    pub(crate) fn get_client(&self) -> Result<L::Client> {
        self.client
            .clone()
            .ok_or_else(|| eyre!("{} client not initialized", L::DISPLAY_NAME))
    }

    // === Lifecycle processing ===

    fn process_message(&mut self, msg: HostMsg<L>) -> Result<ServiceMsg> {
        match msg {
            HostMsg::Lifecycle(msg) => Ok(self.process_lifecycle(msg)),
            HostMsg::Domain(msg) => L::update(self, msg),
        }
    }

    fn process_lifecycle(&mut self, msg: Lifecycle<L::Client>) -> ServiceMsg {
        match msg {
            Lifecycle::Initialize => {
                self.views
                    .set_loading(format!("Initializing {}...", L::DISPLAY_NAME));
                InitClientCmd::<L> {
                    context: self.context.clone(),
                    tx: self.events.clone_sender(),
                }
                .into()
            }

            Lifecycle::Connected(client) => {
                self.client = Some(client);
                L::on_connected(self);
                ServiceMsg::Idle
            }

            Lifecycle::ApiDisabled => {
                self.views.clear_loading();
                self.views.show_modal(EnableApiDialog::<L>::new());
                ServiceMsg::Idle
            }

            Lifecycle::EnableApi => {
                self.views.close_modal();
                self.views
                    .set_loading(format!("Enabling {} API...", L::DISPLAY_NAME));
                EnableApiCmd::<L> {
                    context: self.context.clone(),
                    tx: self.events.clone_sender(),
                }
                .into()
            }

            Lifecycle::ApiEnabled => {
                self.events.send(Lifecycle::Initialize.into());
                ServiceMsg::Idle
            }

            Lifecycle::NavigateBack => {
                if self.views.pop() {
                    ServiceMsg::Idle
                } else {
                    ServiceMsg::Close
                }
            }

            Lifecycle::DialogCancelled => {
                self.views.close_modal();
                if self.views.has_screens() {
                    ServiceMsg::Idle
                } else {
                    ServiceMsg::Close
                }
            }
        }
    }
}

impl<L: GcpServiceLogic> Service for GcpService<L> {
    fn init(&mut self) {
        self.events.send(Lifecycle::Initialize.into());
    }

    fn handle_tick(&mut self) {
        self.views.handle_tick();
    }

    fn handle_key(&mut self, key: KeyEvent) -> EventResult<()> {
        self.views
            .handle_key(key, &self.events, Lifecycle::NavigateBack.into())
    }

    fn update(&mut self) -> Result<ServiceMsg> {
        let mut commands: Vec<Box<dyn Command>> = Vec::new();

        loop {
            let messages = self.events.drain();
            if messages.is_empty() {
                break;
            }
            match EventQueue::process_events(messages, |msg| self.process_message(msg))? {
                ServiceMsg::Idle => {}
                ServiceMsg::Run(cmds) => commands.extend(cmds),
                ServiceMsg::Close => return Ok(ServiceMsg::Close),
                msg @ ServiceMsg::EditExternal { .. } => return Ok(msg),
            }
        }

        if commands.is_empty() {
            Ok(ServiceMsg::Idle)
        } else {
            Ok(ServiceMsg::Run(commands))
        }
    }

    fn handle_editor_result(&mut self, new_content: Option<String>) {
        L::handle_editor_result(self, new_content);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.views.render(frame, area, theme);
    }

    fn breadcrumbs(&self) -> Vec<String> {
        self.views.breadcrumbs(L::DISPLAY_NAME)
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        self.views.keybindings()
    }
}

// === Provider ===

/// Generic [`ServiceProvider`] for any [`GcpServiceLogic`]. Registering a new
/// GCP service is just `registry.register(GcpProvider::<MyLogic>::new())`.
pub struct GcpProvider<L: GcpServiceLogic>(PhantomData<fn() -> L>);

impl<L: GcpServiceLogic> GcpProvider<L> {
    /// Create the provider.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<L: GcpServiceLogic> ServiceProvider for GcpProvider<L> {
    fn provider(&self) -> Provider {
        Provider::Gcp
    }

    fn service_key(&self) -> &'static str {
        L::SERVICE_KEY
    }

    fn display_name(&self) -> &'static str {
        L::DISPLAY_NAME
    }

    fn description(&self) -> &'static str {
        L::DESCRIPTION
    }

    fn icon(&self) -> Option<&'static str> {
        L::ICON
    }

    fn create_service(&self, ctx: &CloudContext) -> Box<dyn Service> {
        let CloudContext::Gcp(gcp_ctx) = ctx;
        Box::new(GcpService::<L>::new(gcp_ctx.clone()))
    }
}

// === Dialog ===

struct EnableApiDialog<L: GcpServiceLogic> {
    dialog: ConfirmDialog,
    _marker: PhantomData<fn() -> L>,
}

impl<L: GcpServiceLogic> EnableApiDialog<L> {
    fn new() -> Self {
        let dialog = ConfirmDialog::new(format!(
            "The {} API is not enabled for this project. Would you like to enable it?",
            L::DISPLAY_NAME
        ))
        .with_title("API Not Enabled")
        .with_confirm_text("Enable");
        Self {
            dialog,
            _marker: PhantomData,
        }
    }
}

impl<L: GcpServiceLogic> Modal for EnableApiDialog<L> {
    type Output = HostMsg<L>;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        Ok(match self.dialog.handle_key(key)? {
            EventResult::Event(ConfirmEvent::Confirmed) => Lifecycle::EnableApi.into(),
            EventResult::Event(ConfirmEvent::Cancelled) => Lifecycle::DialogCancelled.into(),
            _ => EventResult::Consumed,
        })
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.dialog.render(frame, area, theme);
    }
}

// === Commands ===

struct InitClientCmd<L: GcpServiceLogic> {
    context: GcpContext,
    tx: UnboundedSender<HostMsg<L>>,
}

#[async_trait]
impl<L: GcpServiceLogic> Command for InitClientCmd<L> {
    fn name(&self) -> String {
        format!("Connecting to {}", self.context.display_name)
    }

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> Result<()> {
        match L::connect(&self.context).await {
            Ok(client) => {
                info!("Successfully initialized {} client", L::DISPLAY_NAME);
                self.tx.send(Lifecycle::Connected(client).into())?;
                Ok(())
            }
            Err(ConnectError::ApiDisabled) => {
                self.tx.send(Lifecycle::ApiDisabled.into())?;
                Ok(())
            }
            Err(ConnectError::Other(e)) => {
                error!("Failed to initialize {} client: {e}", L::DISPLAY_NAME);
                Err(e)
            }
        }
    }
}

struct EnableApiCmd<L: GcpServiceLogic> {
    context: GcpContext,
    tx: UnboundedSender<HostMsg<L>>,
}

#[async_trait]
impl<L: GcpServiceLogic> Command for EnableApiCmd<L> {
    fn name(&self) -> String {
        format!("Enabling {} API", L::DISPLAY_NAME)
    }

    /// Enabling an API is a long-running operation we poll to completion, so it
    /// gets a generous cap rather than the default short network timeout.
    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_mins(5))
    }

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> Result<()> {
        let credentials = self.context.create_credentials().await?;
        let client = ServiceUsage::builder()
            .with_credentials(credentials)
            .build()
            .await?;

        let service_name = format!(
            "projects/{}/services/{}",
            self.context.project_id,
            L::API_SERVICE
        );

        client
            .enable_service()
            .set_name(service_name)
            .poller()
            .until_done()
            .await?;

        self.tx.send(Lifecycle::ApiEnabled.into())?;
        Ok(())
    }
}
