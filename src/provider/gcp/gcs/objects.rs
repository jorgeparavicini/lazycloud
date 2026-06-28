use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{ListItem, Paragraph, Wrap};
use ratatui::Frame;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use std::sync::Arc;

use crate::Theme;
use crate::commands::{Command, CommandCtx};
use crate::provider::gcp::gcs::Gcs;
use crate::provider::gcp::gcs::client::{ClientError, GcsClient, ObjectInfo, ObjectList};
use crate::provider::gcp::gcs::service::{GcsDomainMsg, GcsMsg};
use crate::provider::gcp::service::Lifecycle;
use crate::service::ServiceMsg;
use crate::ui::{Component, EventResult, Keybinding, List, ListEvent, ListRow, Screen};

// === Model ===

#[derive(Debug, Clone)]
pub enum ObjectEntry {
    ParentDir,
    Folder { name: String, prefix: String },
    Object(ObjectInfo),
}

impl ListRow for ObjectEntry {
    fn render_row(&self, theme: &Theme) -> ListItem<'static> {
        match self {
            Self::ParentDir => {
                ListItem::new("..").style(Style::default().fg(theme.text_muted()))
            }
            Self::Folder { name, .. } => ListItem::new(format!("{name}/"))
                .style(Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
            Self::Object(info) => {
                let line = Line::from(vec![
                    Span::styled(info.name.clone(), Style::default().fg(theme.text())),
                    Span::raw("  "),
                    Span::styled(
                        format_size(info.size),
                        Style::default().fg(theme.text_muted()),
                    ),
                ]);
                ListItem::new(line)
            }
        }
    }
}

fn format_size(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    #[allow(clippy::cast_precision_loss)]
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.1} GB", b / GB)
    }
}

// === Messages ===

#[derive(Debug, Clone)]
pub enum ObjectsMsg {
    Browse { bucket: String },
    Navigate { bucket: String, prefix: String },
    ObjectsLoaded {
        bucket: String,
        prefix: String,
        list: ObjectList,
    },
    FetchPreview { bucket: String, object_name: String },
    PreviewLoaded { content: PreviewContent },
    Reload { bucket: String, prefix: String },
}

impl From<ObjectsMsg> for GcsMsg {
    fn from(msg: ObjectsMsg) -> Self {
        Self::Domain(GcsDomainMsg::Object(msg))
    }
}

impl From<ObjectsMsg> for EventResult<GcsMsg> {
    fn from(msg: ObjectsMsg) -> Self {
        Self::Event(GcsMsg::from(msg))
    }
}

// === Preview ===

#[derive(Debug, Clone)]
pub enum PreviewContent {
    Text {
        content: String,
        truncated: bool,
    },
    Binary {
        size: i64,
    },
    Error {
        message: String,
    },
    Loading {
        object_name: String,
    },
}

// === Screen ===

pub struct ObjectBrowserScreen {
    bucket: String,
    prefix: String,
    list: List<ObjectEntry>,
    preview_rx: UnboundedReceiver<PreviewContent>,
    current_preview: Option<PreviewContent>,
    right_scroll: u16,
}

impl ObjectBrowserScreen {
    pub fn new(
        bucket: String,
        prefix: String,
        entries: Vec<ObjectEntry>,
        preview_rx: UnboundedReceiver<PreviewContent>,
    ) -> Self {
        Self {
            bucket,
            prefix,
            list: List::new(entries),
            preview_rx,
            current_preview: None,
            right_scroll: 0,
        }
    }

    fn display_path(&self) -> String {
        if self.prefix.is_empty() {
            self.bucket.clone()
        } else {
            format!("{}/{}", self.bucket, self.prefix)
        }
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let block = theme
            .block()
            .title(" Preview ".to_string())
            .title_style(theme.title_style());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let paragraph = match &self.current_preview {
            Some(PreviewContent::Text {
                content, truncated, ..
            }) => {
                let mut text = content.clone();
                if *truncated {
                    text.push_str("\n\n--- truncated ---");
                }
                Paragraph::new(text)
                    .style(Style::default().fg(theme.text()))
                    .wrap(Wrap { trim: false })
                    .scroll((self.right_scroll, 0))
            }
            Some(PreviewContent::Binary { size }) => Paragraph::new(format!(
                "Binary file ({})\n\nCannot display binary content.",
                format_size(*size)
            ))
            .style(Style::default().fg(theme.text_muted())),
            Some(PreviewContent::Error { message }) => {
                Paragraph::new(format!("Error: {message}"))
                    .style(Style::default().fg(theme.error()))
            }
            Some(PreviewContent::Loading { object_name }) => {
                Paragraph::new(format!("Loading {object_name}..."))
                    .style(Style::default().fg(theme.text_muted()))
            }
            None => self.list.selected().map_or_else(
                || {
                    Paragraph::new("No items")
                        .style(Style::default().fg(theme.text_muted()))
                },
                |selected| match selected {
                    ObjectEntry::ParentDir => Paragraph::new("Parent directory")
                        .style(Style::default().fg(theme.text_muted())),
                    ObjectEntry::Folder { name, .. } => {
                        Paragraph::new(format!("Directory: {name}/\n\nPress Enter to browse."))
                            .style(Style::default().fg(theme.text_muted()))
                    }
                    ObjectEntry::Object(info) => {
                        let details = format!(
                            "Name: {}\nSize: {}\nType: {}\nStorage: {}\nUpdated: {}",
                            info.full_name,
                            format_size(info.size),
                            info.content_type,
                            info.storage_class,
                            info.updated,
                        );
                        Paragraph::new(details)
                            .style(Style::default().fg(theme.text_dim()))
                            .wrap(Wrap { trim: false })
                    }
                },
            ),
        };

        frame.render_widget(paragraph, inner);
    }
}

impl Screen for ObjectBrowserScreen {
    type Output = GcsMsg;

    fn handle_key(&mut self, key: KeyEvent) -> crate::ui::Result<EventResult<Self::Output>> {
        let result = self.list.handle_key(key)?;

        match &result {
            EventResult::Event(ListEvent::Changed(entry)) => {
                self.right_scroll = 0;
                if let ObjectEntry::Object(info) = entry {
                    self.current_preview = Some(PreviewContent::Loading {
                        object_name: info.full_name.clone(),
                    });
                    return Ok(ObjectsMsg::FetchPreview {
                        bucket: self.bucket.clone(),
                        object_name: info.full_name.clone(),
                    }
                    .into());
                }
                self.current_preview = None;
                return Ok(EventResult::Consumed);
            }
            EventResult::Event(ListEvent::Activated(entry)) => match entry {
                ObjectEntry::ParentDir => {
                    let parent_prefix = parent_of(&self.prefix);
                    return Ok(ObjectsMsg::Navigate {
                        bucket: self.bucket.clone(),
                        prefix: parent_prefix,
                    }
                    .into());
                }
                ObjectEntry::Folder { prefix, .. } => {
                    return Ok(ObjectsMsg::Navigate {
                        bucket: self.bucket.clone(),
                        prefix: prefix.clone(),
                    }
                    .into());
                }
                ObjectEntry::Object(_) => {
                    return Ok(EventResult::Consumed);
                }
            },
            _ => {}
        }

        if result.is_consumed() {
            return Ok(EventResult::Consumed);
        }

        if key.code == KeyCode::Char('r') {
            return Ok(ObjectsMsg::Reload {
                bucket: self.bucket.clone(),
                prefix: self.prefix.clone(),
            }
            .into());
        }

        Ok(EventResult::Ignored)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(area);

        // Left pane: file list
        let left_block = theme
            .block()
            .title(format!(" {} ", self.display_path()))
            .title_style(theme.title_style());
        let left_inner = left_block.inner(chunks[0]);
        frame.render_widget(left_block, chunks[0]);
        self.list.render(frame, left_inner, theme);

        // Right pane: preview
        self.render_preview(frame, chunks[1], theme);
    }

    fn handle_tick(&mut self) {
        while let Ok(content) = self.preview_rx.try_recv() {
            self.current_preview = Some(content);
        }
    }

    fn breadcrumbs(&self) -> Vec<String> {
        if self.prefix.is_empty() {
            vec!["Objects".to_string()]
        } else {
            vec!["Objects".to_string(), self.prefix.clone()]
        }
    }

    fn keybindings(&self) -> Vec<Keybinding> {
        vec![
            Keybinding::primary("Enter", "Open"),
            // Esc/Back is a global binding, so it is not repeated here.
            Keybinding::secondary("r", "Reload"),
        ]
    }
}

fn parent_of(prefix: &str) -> String {
    let trimmed = prefix.trim_end_matches('/');
    match trimmed.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/"),
        None => String::new(),
    }
}

// === Update ===

pub(super) fn update(state: &mut Gcs, msg: ObjectsMsg) -> color_eyre::Result<ServiceMsg> {
    match msg {
        ObjectsMsg::Browse { bucket } => {
            state.views.set_loading("Loading objects...");
            Ok(FetchObjectsCmd {
                client: state.get_client()?,
                bucket,
                prefix: String::new(),
                tx: state.clone_sender(),
            }
            .into())
        }

        ObjectsMsg::Navigate { bucket, prefix } => {
            state.views.set_loading("Loading objects...");
            Ok(FetchObjectsCmd {
                client: state.get_client()?,
                bucket,
                prefix,
                tx: state.clone_sender(),
            }
            .into())
        }

        ObjectsMsg::ObjectsLoaded {
            bucket,
            prefix,
            list,
        } => {
            state.views.clear_loading();

            let mut entries = Vec::new();
            if !prefix.is_empty() {
                entries.push(ObjectEntry::ParentDir);
            }
            for folder_prefix in &list.folders {
                let name = folder_prefix
                    .strip_prefix(&prefix)
                    .unwrap_or(folder_prefix)
                    .trim_end_matches('/')
                    .to_string();
                entries.push(ObjectEntry::Folder {
                    name,
                    prefix: folder_prefix.clone(),
                });
            }
            for obj in list.objects {
                entries.push(ObjectEntry::Object(obj));
            }

            let (tx, rx) = mpsc::unbounded_channel();
            state.logic.preview_tx = Some(tx);

            // Auto-trigger preview for first object
            let first_object = entries.iter().find_map(|e| {
                if let ObjectEntry::Object(info) = e {
                    Some(info.clone())
                } else {
                    None
                }
            });

            state.views.push(ObjectBrowserScreen::new(
                bucket.clone(),
                prefix,
                entries,
                rx,
            ));

            if let Some(info) = first_object {
                state.queue(
                    ObjectsMsg::FetchPreview {
                        bucket,
                        object_name: info.full_name,
                    }
                    .into(),
                );
            }

            Ok(ServiceMsg::Idle)
        }

        ObjectsMsg::FetchPreview {
            bucket,
            object_name,
        } => Ok(FetchPreviewCmd {
            client: state.get_client()?,
            bucket,
            object_name,
            tx: state.clone_sender(),
        }
        .into()),

        ObjectsMsg::PreviewLoaded { content } => {
            if let Some(tx) = &state.logic.preview_tx {
                let _ = tx.send(content);
            }
            Ok(ServiceMsg::Idle)
        }

        ObjectsMsg::Reload { bucket, prefix } => {
            state.views.pop();
            state.logic.preview_tx = None;
            state.views.set_loading("Reloading...");
            Ok(FetchObjectsCmd {
                client: state.get_client()?,
                bucket,
                prefix,
                tx: state.clone_sender(),
            }
            .into())
        }
    }
}

// === Commands ===

struct FetchObjectsCmd {
    client: GcsClient,
    bucket: String,
    prefix: String,
    tx: UnboundedSender<GcsMsg>,
}

#[async_trait]
impl Command for FetchObjectsCmd {
    fn name(&self) -> String {
        format!("Loading objects in {}", self.bucket)
    }

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> color_eyre::Result<()> {
        match self.client.list_objects(&self.bucket, &self.prefix).await {
            Ok(list) => {
                self.tx.send(
                    ObjectsMsg::ObjectsLoaded {
                        bucket: self.bucket,
                        prefix: self.prefix,
                        list,
                    }
                    .into(),
                )?;
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

struct FetchPreviewCmd {
    client: GcsClient,
    bucket: String,
    object_name: String,
    tx: UnboundedSender<GcsMsg>,
}

#[async_trait]
impl Command for FetchPreviewCmd {
    fn name(&self) -> String {
        format!("Loading preview for {}", self.object_name)
    }

    async fn execute(self: Box<Self>, _ctx: Arc<dyn CommandCtx>) -> color_eyre::Result<()> {
        let content = match self.client.read_object(&self.bucket, &self.object_name).await {
            Ok(data) => {
                let max_bytes = 64 * 1024;
                let truncated = data.len() >= max_bytes;
                match String::from_utf8(data) {
                    Ok(text) => PreviewContent::Text { content: text, truncated },
                    #[allow(clippy::cast_possible_wrap)]
                    Err(e) => PreviewContent::Binary {
                        size: e.into_bytes().len() as i64,
                    },
                }
            }
            Err(e) => PreviewContent::Error {
                message: e.to_string(),
            },
        };

        let _ = self.tx.send(ObjectsMsg::PreviewLoaded { content }.into());
        Ok(())
    }
}
