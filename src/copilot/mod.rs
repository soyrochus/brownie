use crate::event::AppEvent;
use copilot_sdk::{Client, ConnectionState, Session, SessionConfig, SessionEventData};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

#[derive(Clone)]
pub struct CopilotClient {
    workspace: PathBuf,
    tx: mpsc::Sender<AppEvent>,
    client: Arc<Client>,
    session: Arc<RwLock<Option<Arc<Session>>>>,
    runtime_handle: Handle,
    state_poller_started: Arc<AtomicBool>,
}

impl CopilotClient {
    pub fn new(workspace: PathBuf, tx: mpsc::Sender<AppEvent>) -> copilot_sdk::Result<Self> {
        let runtime_handle = Handle::try_current().map_err(|err| {
            copilot_sdk::CopilotError::InvalidConfig(format!("tokio runtime unavailable: {err}"))
        })?;

        let client = Client::builder()
            .use_stdio(true)
            .auto_restart(true)
            .cwd(workspace.clone())
            .deny_tools(vec!["*"])
            .build()?;

        Ok(Self {
            workspace,
            tx,
            client: Arc::new(client),
            session: Arc::new(RwLock::new(None)),
            runtime_handle,
            state_poller_started: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn start(&self) {
        let _ = self
            .tx
            .send(AppEvent::StatusChanged(ConnectionState::Connecting));
        self.spawn_state_poller();

        let client = Arc::clone(&self.client);
        let tx = self.tx.clone();
        let workspace = self.workspace.clone();
        let session_slot = Arc::clone(&self.session);
        let runtime_handle = self.runtime_handle.clone();

        self.runtime_handle.spawn(async move {
            if let Err(err) = client.start().await {
                let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                let _ = tx.send(AppEvent::SdkError(format!(
                    "failed to start Copilot client: {err}"
                )));
                return;
            }

            match client.get_auth_status().await {
                Ok(auth) if auth.is_authenticated => {
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Connected));
                }
                Ok(auth) => {
                    let message = auth
                        .status_message
                        .unwrap_or_else(|| "copilot CLI is not authenticated".to_string());
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                    let _ = tx.send(AppEvent::SdkError(message));
                    return;
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                    let _ = tx.send(AppEvent::SdkError(format!(
                        "failed to query auth status: {err}"
                    )));
                    return;
                }
            }

            let mut session_config = SessionConfig::default();
            session_config.working_directory = Some(workspace.to_string_lossy().to_string());

            match client.create_session(session_config).await {
                Ok(session) => {
                    let session_id = session.session_id().to_string();
                    {
                        let mut slot = session_slot.write().await;
                        *slot = Some(Arc::clone(&session));
                    }
                    let _ = tx.send(AppEvent::SessionCreated(session_id));
                    Self::spawn_event_listener(runtime_handle, session, tx);
                }
                Err(err) => {
                    let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Error));
                    let _ = tx.send(AppEvent::SdkError(format!(
                        "failed to create session: {err}"
                    )));
                }
            }
        });
    }

    pub fn send(&self, prompt: String) {
        let tx = self.tx.clone();
        let session_slot = Arc::clone(&self.session);

        self.runtime_handle.spawn(async move {
            let session = {
                let guard = session_slot.read().await;
                guard.clone()
            };

            let Some(session) = session else {
                let _ = tx.send(AppEvent::SdkError("No active session".to_string()));
                return;
            };

            if let Err(err) = session.send(prompt).await {
                let _ = tx.send(AppEvent::SdkError(format!("failed to send prompt: {err}")));
            }
        });
    }

    fn spawn_state_poller(&self) {
        if self
            .state_poller_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let tx = self.tx.clone();
        let client = Arc::clone(&self.client);
        self.runtime_handle.spawn(async move {
            let mut ticker = time::interval(Duration::from_millis(500));
            let mut last_state = client.state().await;

            loop {
                ticker.tick().await;
                let current_state = client.state().await;
                if current_state != last_state {
                    last_state = current_state;
                    let _ = tx.send(AppEvent::StatusChanged(current_state));
                }
            }
        });
    }

    fn spawn_event_listener(
        runtime_handle: Handle,
        session: Arc<Session>,
        tx: mpsc::Sender<AppEvent>,
    ) {
        runtime_handle.spawn(async move {
            let mut events = session.subscribe();
            loop {
                match events.recv().await {
                    Ok(event) => match event.data {
                        SessionEventData::AssistantMessageDelta(delta) => {
                            let _ = tx.send(AppEvent::StreamDelta(delta.delta_content));
                        }
                        SessionEventData::AssistantMessage(message) => {
                            let _ = tx.send(AppEvent::StreamDelta(message.content));
                            let _ = tx.send(AppEvent::StreamEnd);
                        }
                        SessionEventData::SessionIdle(_) => {
                            let _ = tx.send(AppEvent::StreamEnd);
                        }
                        SessionEventData::SessionError(err) => {
                            let _ = tx.send(AppEvent::SdkError(err.message));
                        }
                        SessionEventData::ToolUserRequested(data) => {
                            let _ = tx.send(AppEvent::ToolCallSuppressed(data.tool_name));
                        }
                        SessionEventData::ToolExecutionStart(data) => {
                            let _ = tx.send(AppEvent::ToolCallSuppressed(data.tool_name));
                        }
                        _ => {}
                    },
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        let _ = tx.send(AppEvent::StatusChanged(ConnectionState::Disconnected));
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                }
            }
        });
    }
}
