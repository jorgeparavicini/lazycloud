use std::sync::Arc;

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::Cell;
use tokio::sync::mpsc::UnboundedSender;

use crate::Theme;
use crate::cache::CacheKey;
use crate::commands::{Command, CommandCtx};
use crate::provider::gcp::gcs::Gcs;
use crate::provider::gcp::gcs::client::{ClientError, GcsClient};
use crate::provider::gcp::gcs::objects::ObjectsMsg;
use crate::provider::gcp::gcs::service::{GcsDomainMsg, GcsMsg};
use crate::provider::gcp::service::Lifecycle;
use crate::search::Matcher;
use crate::service::ServiceMsg;
use crate::ui::{
    ColumnDef,
    Component,
    EventResult,
    Keybinding,
    Result,
    Screen,
    Table,
    TableEvent,
    TableRow,
};

// === Cache ===

/// The buckets of the current project.
#[derive(Debug, Hash, PartialEq, Eq)]
struct BucketsKey;

impl CacheKey for BucketsKey {
    type Value = Vec<Bucket>;
}

// === Model ===

#[derive(Debug, Clone)]
pub struct Bucket {
    pub name: String,
    pub location: String,
    pub storage_class: String,
}

impl TableRow for Bucket {
    fn columns() -> &'static [ColumnDef] {
        static COLUMNS: &[ColumnDef] = &[
            ColumnDef::new("Name", Constraint::Min(20)),
            ColumnDef::new("Location", Constraint::Length(15)),
            ColumnDef::new("Storage Class", Constraint::Length(15)),
        ];
        COLUMNS
    }

    fn render_cells(&self, theme: &Theme) -> Vec<Cell<'static>> {
        self.render_cells_with_query(theme, "")
    }

    fn render_cells_with_query(&self, _theme: &Theme, _query: &str) -> Vec<Cell<'static>> {
        vec![
            Cell::from(self.name.clone()),
            Cell::from(self.location.clone()),
            Cell::from(self.storage_class.clone()),
        ]
    }

    fn matches(&self, query: &str) -> bool {
        let matcher = Matcher::new();
        matcher.matches(&self.name, query) || matcher.matches(&self.location, query)
    }
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum BucketsMsg {
    Load,
    Loaded(Vec<Bucket>),
}

impl From<BucketsMsg> for GcsMsg {
    fn from(msg: BucketsMsg) -> Self {
        Self::Domain(GcsDomainMsg::Bucket(msg))
    }
}

impl From<BucketsMsg> for EventResult<GcsMsg> {
    fn from(msg: BucketsMsg) -> Self {
        Self::Event(GcsMsg::from(msg))
    }
}

// === Screen ===

pub struct BucketListScreen {
    table: Table<Bucket>,
}

impl BucketListScreen {
    pub fn new(buckets: Vec<Bucket>) -> Self {
        Self {
            table: Table::new(buckets).with_title(" Buckets "),
        }
    }
}

impl Screen for BucketListScreen {
    type Output = GcsMsg;

    fn handle_key(&mut self, key: KeyEvent) -> Result<EventResult<Self::Output>> {
        let result = self.table.handle_key(key)?;

        if let EventResult::Event(TableEvent::Activated(bucket)) = result {
            return Ok(ObjectsMsg::Browse {
                bucket: bucket.name,
            }
            .into());
        }
        if result.is_consumed() {
            return Ok(EventResult::Consumed);
        }

        if key.code == KeyCode::Char('r') {
            return Ok(BucketsMsg::Load.into());
        }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.table.render(frame, area, theme);
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::primary("/", "Search"),
            Keybinding::secondary("r", "Reload"),
        ]
    }
}

// === Update ===

pub(super) fn update(state: &mut Gcs, msg: BucketsMsg) -> color_eyre::Result<ServiceMsg> {
    match msg {
        BucketsMsg::Load => {
            if let Some(buckets) = state.cache.get(&BucketsKey) {
                state.views.push(BucketListScreen::new(buckets.clone()));
                return Ok(ServiceMsg::Idle);
            }

            state.views.set_loading("Loading buckets...");

            Ok(FetchBucketsCmd {
                client: state.get_client()?,
                tx: state.clone_sender(),
            }
            .into())
        }

        BucketsMsg::Loaded(buckets) => {
            state.views.clear_loading();
            state.cache.insert(BucketsKey, buckets.clone());
            state.views.push(BucketListScreen::new(buckets));
            Ok(ServiceMsg::Idle)
        }
    }
}

// === Commands ===

struct FetchBucketsCmd {
    client: GcsClient,
    tx: UnboundedSender<GcsMsg>,
}

#[async_trait]
impl Command for FetchBucketsCmd {
    fn name(&self) -> String {
        "Loading buckets".to_string()
    }

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> Result<()> {
        match self.client.list_buckets().await {
            Ok(infos) => {
                let buckets = infos
                    .into_iter()
                    .map(|info| Bucket {
                        name: info.name,
                        location: info.location,
                        storage_class: info.storage_class,
                    })
                    .collect();
                self.tx.send(BucketsMsg::Loaded(buckets).into())?;
                Ok(())
            }
            Err(ClientError::ApiDisabled) => {
                self.tx.send(Lifecycle::ApiDisabled.into())?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
