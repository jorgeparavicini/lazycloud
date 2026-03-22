use crate::Theme;
use crate::app::AppMessage;
use crate::commands::Command;
use crate::context::{CloudContext, GcpContext};
use crate::provider::Provider;
use crate::provider::gcp::gcs::client::GcsClient;
use crate::registry::ServiceProvider;
use crate::service::{Service, ServiceMsg};
use crate::ui::EventResult;
use async_trait::async_trait;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use tokio::sync::mpsc::{UnboundedSender};
use tracing::error;
use crate::event_queue::EventQueue;
use crate::view_stack::ViewStack;

pub(super) enum GcsMsg {
    Initialize,
}

pub struct GcsProvider;

impl ServiceProvider for GcsProvider {
    fn provider(&self) -> Provider {
        Provider::Gcp
    }

    fn service_key(&self) -> &'static str {
        "gcs"
    }

    fn display_name(&self) -> &'static str {
        "Cloud Storage"
    }

    fn description(&self) -> &'static str {
        "Manage Google Cloud Storage buckets and objects."
    }

    fn create_service(&self, ctx: &CloudContext) -> Box<dyn Service> {
        let CloudContext::Gcp(gcp_ctx) = ctx;
        Box::new(Gcs::new(gcp_ctx.clone()))
    }
}

pub struct Gcs {
    context: GcpContext,
    client: Option<GcsClient>,
    event_queue: EventQueue<GcsMsg>,
    views: ViewStack<GcsMsg>,
}

impl Gcs {
    pub fn new(ctx: GcpContext) -> Self {
        Self {
            context: ctx,
            client: None,
            event_queue: EventQueue::new(),
            views: ViewStack::new(),
        }
    }

    fn process_message(&mut self, msg: GcsMsg) -> Result<ServiceMsg> {
        match msg {
            GcsMsg::Initialize => {
                let cmd = InitClientCmd {
                    context: self.context.clone(),
                    tx: self.event_queue.clone_sender(),
                };
                Ok(ServiceMsg::Run(vec![Box::new(cmd)]))
            }
        }
    }
}

impl Service for Gcs {
    fn init(&mut self) {
        self.event_queue.send(GcsMsg::Initialize);
    }

    fn handle_key(&mut self, key: KeyEvent) -> EventResult<()> {
        EventResult::Ignored
    }

    fn update(&mut self) -> Result<ServiceMsg> {
        let mut commands: Vec<Box<dyn crate::commands::Command>> = Vec::new();

        loop {
            let messages = self.event_queue.drain();
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

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.views.render(frame, area, theme);
    }

    fn breadcrumbs(&self) -> Vec<String> {
        vec!["GCS".to_string()]
    }
}

struct InitClientCmd {
    context: GcpContext,
    tx: UnboundedSender<GcsMsg>,
}

#[async_trait]
impl Command for InitClientCmd {
    fn name(&self) -> String {
        format!(
            "Initializing GCS client for project {}",
            self.context.project_id
        )
    }

    async fn execute(
        self: Box<Self>,
        action_tx: UnboundedSender<AppMessage>,
    ) -> crate::ui::Result<()> {
        match GcsClient::new(&self.context).await {
            Ok(client) => {
                self.tx.send(GcsMsg::Initialize)?;
                Ok(())
            }
            // TODO: Handle disabled api globally
            Err(err) => {
                error!("Failed to initialize GCS client: {err}");
                Err(err.into())
            }
        }
    }
}
