use crate::action::Action;
use crate::app::AppContext;
use crate::components::Component;
use crate::components::EventResult;
use crate::components::EventResult::{Consumed, Ignored};
use crate::components::services::gcp::GcpAction;
use crate::widgets::Loader;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use google_cloud_secretmanager_v1::client::SecretManagerService as GcpSecretManagerClient;
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, layout::Rect};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub enum SecretManagerAction {
    ServiceLoaded(SecretManagerService),
    SecretsLoaded(Vec<Secret>),
    VersionsLoaded(Secret, Vec<SecretVersion>),
    PayloadLoaded(Secret, SecretVersion, SecretPayload),
}

impl PartialEq for SecretManagerAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ServiceLoaded(_), Self::ServiceLoaded(_)) => true,
            (Self::SecretsLoaded(a), Self::SecretsLoaded(b)) => a == b,
            (Self::VersionsLoaded(a1, b1), Self::VersionsLoaded(a2, b2)) => a1 == a2 && b1 == b2,
            (Self::PayloadLoaded(s1, v1, p1), Self::PayloadLoaded(s2, v2, p2)) => {
                s1 == s2 && v1 == v2 && p1 == p2
            }
            _ => false,
        }
    }
}

impl Eq for SecretManagerAction {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub name: String,
    pub created_at: String,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretVersion {
    pub version_id: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPayload {
    pub data: String,
    pub is_binary: bool,
}

struct SecretList {
    secrets: Vec<Secret>,
    state: ListState,
}

struct SecretVersionList {
    secret: Secret,
    versions: Vec<SecretVersion>,
    state: ListState,
}

struct SecretDetails {
    secret: Secret,
    version: SecretVersion,
    payload: SecretPayload,
}

struct Loading {
    loader: Loader,
}

impl Loading {
    fn new(label: String) -> Self {
        Self {
            loader: Loader::new().with_label(label),
        }
    }
}

enum SecretManagerViewState {
    SecretList(SecretList),
    VersionList(SecretVersionList),
    SecretDetails(SecretDetails),
    Loading(Loading),
}

pub struct SecretManager {
    app_context: AppContext,
    view_state: SecretManagerViewState,
    service: Option<SecretManagerService>,
}

impl SecretManager {
    pub fn new(app_context: &AppContext) -> Self {
        let instance = Self {
            app_context: AppContext {
                active_context: app_context.active_context.clone(),
                action_tx: app_context.action_tx.clone(),
            },
            view_state: SecretManagerViewState::Loading(Loading::new(
                "Initializing secret manager...".to_string(),
            )),
            service: None,
        };

        // TODO: Replace below with actual project ID from context
        let project_id = "".to_string();
        let action_tx = app_context.action_tx.clone();

        // TODO: How to handle errors during async initialization?
        tokio::spawn(async move {
            match SecretManagerService::new(project_id).await {
                Ok(service) => {
                    let _ = action_tx.send(Action::Gcp(GcpAction::SecretManagerAction(
                        SecretManagerAction::ServiceLoaded(service),
                    )));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::DisplayError(format!(
                        "Failed to load Secret Manager service: {}",
                        e
                    )));
                }
            }
        });

        instance
    }

    /// Navigate one level back in the view state stack.
    ///
    /// Returns true if navigation was successful, false if already at the root.
    fn go_back(&mut self) -> bool {
        match &self.view_state {
            SecretManagerViewState::SecretList(_) => false, // Already at root
            SecretManagerViewState::VersionList(_) => {
                self.view_state =
                    SecretManagerViewState::Loading(Loading::new("Loading secrets...".to_string()));
                SecretManager::load_secrets(
                    self.service.clone().unwrap(),
                    self.app_context.action_tx.clone(),
                );
                true
            }
            SecretManagerViewState::SecretDetails(details) => {
                let secret_clone = details.secret.clone();
                self.view_state = SecretManagerViewState::Loading(Loading::new(
                    "Loading versions...".to_string(),
                ));
                SecretManager::load_versions(
                    self.service.clone().unwrap(),
                    secret_clone,
                    self.app_context.action_tx.clone(),
                );
                true
            }
            SecretManagerViewState::Loading(_) => false,
        }
    }

    /// Handle selection based on the current view state.
    /// Selecting a secret loads its versions, selecting a version loads its details.
    ///
    /// Returns true if selection was handled, false otherwise.
    /// For example, if in loading state or at details view.
    fn select(&mut self) -> bool {
        match &mut self.view_state {
            SecretManagerViewState::SecretList(secret_list) => {
                if let Some(selected) = secret_list.state.selected() {
                    let secret_clone = secret_list.secrets[selected].clone();
                    self.view_state = SecretManagerViewState::Loading(Loading::new(
                        "Loading versions...".to_string(),
                    ));
                    SecretManager::load_versions(
                        self.service.clone().unwrap(),
                        secret_clone,
                        self.app_context.action_tx.clone(),
                    );
                    return true;
                }
                false
            }
            SecretManagerViewState::VersionList(version_list) => {
                if let Some(selected) = version_list.state.selected() {
                    let version_clone = version_list.versions[selected].clone();
                    let secret_clone = version_list.secret.clone();
                    self.view_state = SecretManagerViewState::Loading(Loading::new(
                        "Loading payload...".to_string(),
                    ));
                    SecretManager::load_payload(
                        self.service.clone().unwrap(),
                        secret_clone,
                        version_clone,
                        self.app_context.action_tx.clone(),
                    );
                    return true;
                }
                false
            }
            SecretManagerViewState::SecretDetails(_) => false,
            SecretManagerViewState::Loading(_) => false,
        }
    }

    fn next(&mut self) -> bool {
        match &mut self.view_state {
            SecretManagerViewState::SecretList(secret_list) => {
                secret_list.state.select_next();
                true
            }
            SecretManagerViewState::VersionList(version_list) => {
                version_list.state.select_next();
                true
            }
            _ => false,
        }
    }

    fn previous(&mut self) -> bool {
        match &mut self.view_state {
            SecretManagerViewState::SecretList(secret_list) => {
                secret_list.state.select_previous();
                true
            }
            SecretManagerViewState::VersionList(version_list) => {
                version_list.state.select_previous();
                true
            }
            _ => false,
        }
    }

    fn load_secrets(service: SecretManagerService, action_tx: UnboundedSender<Action>) {
        tokio::spawn(async move {
            // sleep for demo purposes
            sleep(Duration::from_secs(3)).await;
            match service.list_secrets().await {
                Ok(secrets) => {
                    let _ = action_tx.send(Action::Gcp(GcpAction::SecretManagerAction(
                        SecretManagerAction::SecretsLoaded(secrets),
                    )));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::DisplayError(format!(
                        "Failed to load secrets: {}",
                        e
                    )));
                }
            }
        });
    }

    fn load_versions(
        service: SecretManagerService,
        secret: Secret,
        action_tx: UnboundedSender<Action>,
    ) {
        tokio::spawn(async move {
            match service.list_versions(&secret.name).await {
                Ok(versions) => {
                    let _ = action_tx.send(Action::Gcp(GcpAction::SecretManagerAction(
                        SecretManagerAction::VersionsLoaded(secret, versions),
                    )));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::DisplayError(format!(
                        "Failed to load versions for secret {}: {}",
                        secret.name, e
                    )));
                }
            }
        });
    }

    fn load_payload(
        service: SecretManagerService,
        secret: Secret,
        version: SecretVersion,
        action_tx: UnboundedSender<Action>,
    ) {
        tokio::spawn(async move {
            match service
                .access_version(&secret.name, &version.version_id)
                .await
            {
                Ok(data) => {
                    let payload = SecretPayload {
                        data,
                        is_binary: false,
                    };
                    let _ = action_tx.send(Action::Gcp(GcpAction::SecretManagerAction(
                        SecretManagerAction::PayloadLoaded(secret, version, payload),
                    )));
                }
                Err(e) => {
                    let _ = action_tx.send(Action::DisplayError(format!(
                        "Failed to load payload for secret {} version {}: {}",
                        secret.name, version.version_id, e
                    )));
                }
            }
        });
    }

    fn render_secret_list(frame: &mut Frame, area: Rect, secret_list: &mut SecretList) {
        let items = secret_list
            .secrets
            .iter()
            .map(|i| ListItem::new(i.name.clone()))
            .collect::<Vec<ListItem>>();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut secret_list.state)
    }

    fn render_version_list(frame: &mut Frame, area: Rect, version_list: &mut SecretVersionList) {
        let items = version_list
            .versions
            .iter()
            .map(|i| ListItem::new(i.version_id.clone()))
            .collect::<Vec<ListItem>>();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut version_list.state)
    }

    fn render_secret_details(frame: &mut Frame, area: Rect, details: &SecretDetails) {
        let text = format!(
            "Secret: {}\nVersion: {}\n\nPayload:\n{}",
            details.secret.name, details.version.version_id, details.payload.data
        );
        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Secret Details"),
        );
        frame.render_widget(paragraph, area)
    }
}

impl Component for SecretManager {
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<EventResult> {
        match key.code {
            KeyCode::Esc => {
                if self.go_back() {
                    return Ok(Consumed(None));
                }
            }
            KeyCode::Enter => {
                if self.select() {
                    return Ok(Consumed(None));
                }
            }
            KeyCode::Down => {
                if self.next() {
                    return Ok(Consumed(None));
                }
            }
            KeyCode::Up => {
                if self.previous() {
                    return Ok(Consumed(None));
                }
            }
            _ => {}
        }
        Ok(Ignored)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if action == Action::Tick {
            if let SecretManagerViewState::Loading(loading) = &mut self.view_state {
                loading.loader.on_tick();
            }
            return Ok(None);
        }

        let Action::Gcp(GcpAction::SecretManagerAction(action)) = action else {
            return Ok(None);
        };

        let action_tx = self.app_context.action_tx.clone();

        match action {
            SecretManagerAction::ServiceLoaded(service) => {
                self.service = Some(service.clone());

                SecretManager::load_secrets(service, action_tx);
                Ok(None)
            }
            SecretManagerAction::SecretsLoaded(secrets) => {
                let secret_list = SecretList {
                    secrets,
                    state: ListState::default(),
                };
                self.view_state = SecretManagerViewState::SecretList(secret_list);
                Ok(None)
            }
            SecretManagerAction::VersionsLoaded(secret, versions) => {
                let version_list = SecretVersionList {
                    secret,
                    versions,
                    state: ListState::default(),
                };
                self.view_state = SecretManagerViewState::VersionList(version_list);
                Ok(None)
            }
            SecretManagerAction::PayloadLoaded(secret, version, payload) => {
                let details = SecretDetails {
                    secret,
                    version,
                    payload,
                };
                self.view_state = SecretManagerViewState::SecretDetails(details);
                Ok(None)
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.view_state {
            SecretManagerViewState::SecretList(secret_list) => {
                SecretManager::render_secret_list(frame, area, secret_list)
            }
            SecretManagerViewState::VersionList(version_list) => {
                SecretManager::render_version_list(frame, area, version_list)
            }
            SecretManagerViewState::SecretDetails(details) => {
                SecretManager::render_secret_details(frame, area, details)
            }
            SecretManagerViewState::Loading(loading) => loading.loader.render(frame, area),
        }
    }

    fn breadcrumbs(&self) -> Vec<String> {
        let b = vec!["Secret Manager".to_string()];
        b
    }
}

#[derive(Clone, Debug)]
pub struct SecretManagerService {
    client: GcpSecretManagerClient,
    project_id: String,
}

impl SecretManagerService {
    pub async fn new(project_id: String) -> Result<Self> {
        let client = GcpSecretManagerClient::builder().build().await?;
        Ok(Self { client, project_id })
    }

    pub async fn list_secrets(&self) -> Result<Vec<Secret>> {
        let parent = format!("projects/{}", self.project_id);

        let response = self.client.list_secrets().set_parent(parent).send().await?;

        let mut secrets = Vec::new();
        for secret in response.secrets {
            if let Some(name) = secret.name.split('/').last() {
                secrets.push(Secret {
                    name: name.to_string(),
                    created_at: secret
                        .create_time
                        .as_ref()
                        .map_or("Unknown".to_string(), |t| t.seconds().to_string()),
                    labels: secret.labels.clone(),
                });
            }
        }
        Ok(secrets)
    }

    pub async fn list_versions(&self, secret_id: &str) -> Result<Vec<SecretVersion>> {
        let parent = format!("projects/{}/secrets/{}", self.project_id, secret_id);

        let response = self
            .client
            .list_secret_versions()
            .set_parent(parent)
            .send()
            .await?;

        let mut versions = Vec::new();
        for version in response.versions {
            if let Some(name) = version.name.split('/').last() {
                versions.push(SecretVersion {
                    version_id: name.to_string(),
                    state: format!("{:?}", version.state),
                    created_at: version
                        .create_time
                        .as_ref()
                        .map_or("Unknown".to_string(), |t| t.seconds().to_string()),
                });
            }
        }
        Ok(versions)
    }

    pub async fn access_version(&self, secret_id: &str, version_id: &str) -> Result<String> {
        let name = format!(
            "projects/{}/secrets/{}/versions/{}",
            self.project_id, secret_id, version_id
        );

        let response = self
            .client
            .access_secret_version()
            .set_name(name)
            .send()
            .await?;

        if let Some(payload) = response.payload {
            let data = String::from_utf8_lossy(&payload.data).to_string();
            Ok(data)
        } else {
            Ok("No payload".to_string())
        }
    }
}
