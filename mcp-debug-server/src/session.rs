use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::dap::{AdapterEndpoint, AdapterSettings, DapClient, DapEvent};

pub type SessionId = String;

#[derive(Default, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, Arc<Mutex<DebugSession>>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create_session(&self, params: CreateSessionRequest) -> Result<SessionId> {
        let adapter = params
            .adapter
            .clone()
            .unwrap_or_else(|| AdapterSettings {
                endpoint: AdapterEndpoint::Spawn {
                    command: "codelldb".to_string(),
                    args: vec![],
                    host: "127.0.0.1".to_string(),
                },
            });

        let dap = DapClient::connect(adapter.clone()).await?;
        let mut session = DebugSession::new(
            params.program.clone(),
            params.cwd.clone().map(PathBuf::from),
            adapter,
            dap,
        );
        session.initialize().await?;
        let id = session.id.clone();
        let mut guard = self.sessions.write().await;
        guard.insert(id.clone(), Arc::new(Mutex::new(session)));
        Ok(id)
    }

    pub async fn with_session<F, Fut, R>(&self, session_id: &str, f: F) -> Result<R>
    where
        F: FnOnce(&mut DebugSession) -> Fut,
        Fut: Future<Output = Result<R>>,
    {
        let session_arc = {
            let guard = self.sessions.read().await;
            guard
                .get(session_id)
                .cloned()
                .ok_or_else(|| anyhow!("Сессия {session_id} не найдена"))?
        };

        let mut session = session_arc.lock().await;
        f(&mut session).await
    }

    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut guard = self.sessions.write().await;
        guard
            .remove(session_id)
            .ok_or_else(|| anyhow!("Сессия {session_id} не найдена"))
            .map(|_| ())
    }

    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let guard = self.sessions.read().await;
        guard
            .values()
            .filter_map(|session| session.try_lock().ok())
            .map(|session| SessionSummary {
                id: session.id.clone(),
                program: session.program.clone(),
                state: session.state,
                current_thread_id: session.current_thread_id,
            })
            .collect()
    }
}

pub struct DebugSession {
    pub id: SessionId,
    pub program: String,
    pub cwd: Option<PathBuf>,
    pub adapter: AdapterSettings,
    dap: DapClient,
    state: SessionState,
    current_thread_id: Option<u32>,
    pending_events: VecDeque<DapEvent>,
}

impl DebugSession {
    pub fn new(
        program: String,
        cwd: Option<PathBuf>,
        adapter: AdapterSettings,
        dap: DapClient,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            program,
            cwd,
            adapter,
            dap,
            state: SessionState::Initializing,
            current_thread_id: None,
            pending_events: VecDeque::new(),
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn adapter(&self) -> &AdapterSettings {
        &self.adapter
    }

    pub fn current_thread_id(&self) -> Option<u32> {
        self.current_thread_id
    }

    pub fn consume_events(&mut self) -> Vec<DapEvent> {
        let mut consumed = Vec::new();
        while let Some(event) = self.pending_events.pop_front() {
            self.apply_event(&event);
            consumed.push(event);
        }
        consumed
    }

    pub async fn initialize(&mut self) -> Result<()> {
        let initialize_args = json!({
            "clientID": "mcp-debug-server",
            "clientName": "MCP Debug Server",
            "adapterID": "lldb",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "pathFormat": "path",
            "supportsRunInTerminalRequest": false,
        });
        self.dap
            .send_request("initialize", initialize_args, &mut self.pending_events)
            .await?;

        // CodeLLDB обычно отправляет initialized-событие сразу; фиксируем его.
        self.consume_events();

        self.dap
            .send_request("configurationDone", json!({}), &mut self.pending_events)
            .await?;
        self.state = SessionState::Ready;
        Ok(())
    }

    pub async fn set_breakpoints(
        &mut self,
        file: &str,
        lines: Vec<u32>,
    ) -> Result<Vec<Breakpoint>> {
        let body = self
            .dap
            .send_request(
                "setBreakpoints",
                json!({
                    "source": { "path": file },
                    "breakpoints": lines.iter().map(|line| json!({ "line": line })).collect::<Vec<_>>(),
                }),
                &mut self.pending_events,
            )
            .await?;
        let breakpoints = body
            .get("breakpoints")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let result = breakpoints
            .into_iter()
            .map(|bp| Breakpoint {
                line: bp
                    .get("line")
                    .and_then(Value::as_u64)
                    .map(|v| v as u32)
                    .or_else(|| lines.first().copied())
                    .unwrap_or_default(),
                verified: bp
                    .get("verified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                message: bp.get("message").and_then(Value::as_str).map(|s| s.to_string()),
            })
            .collect();
        self.consume_events();
        Ok(result)
    }

    pub async fn launch(&mut self, params: LaunchParameters) -> Result<()> {
        let cwd = params
            .cwd
            .clone()
            .or_else(|| self.cwd.as_ref().map(|p| p.to_string_lossy().into_owned()));

        let program = params.program.unwrap_or_else(|| self.program.clone());
        let mut launch_args = json!({
            "program": program,
            "args": params.args.unwrap_or_default(),
            "cwd": cwd,
            "stopOnEntry": params.stop_on_entry.unwrap_or(false),
        });

        if let Some(env) = params.env {
            launch_args["env"] = json!(env);
        }

        self.dap
            .send_request("launch", launch_args, &mut self.pending_events)
            .await?;
        self.state = SessionState::Running;
        self.consume_events();
        Ok(())
    }

    pub async fn continue_execution(&mut self) -> Result<()> {
        let args = match self.current_thread_id {
            Some(thread_id) => json!({ "threadId": thread_id }),
            None => json!({}),
        };
        self.dap
            .send_request("continue", args, &mut self.pending_events)
            .await?;
        self.state = SessionState::Running;
        self.consume_events();
        Ok(())
    }

    pub async fn step_in(&mut self) -> Result<()> {
        let thread_id = self
            .current_thread_id
            .ok_or_else(|| anyhow!("Нет активного потока — выполните wait_for_stop"))?;
        self.dap
            .send_request("stepIn", json!({ "threadId": thread_id }), &mut self.pending_events)
            .await?;
        self.consume_events();
        Ok(())
    }

    pub async fn step_over(&mut self) -> Result<()> {
        let thread_id = self
            .current_thread_id
            .ok_or_else(|| anyhow!("Нет активного потока — выполните wait_for_stop"))?;
        self.dap
            .send_request("next", json!({ "threadId": thread_id }), &mut self.pending_events)
            .await?;
        self.consume_events();
        Ok(())
    }

    pub async fn step_out(&mut self) -> Result<()> {
        let thread_id = self
            .current_thread_id
            .ok_or_else(|| anyhow!("Нет активного потока — выполните wait_for_stop"))?;
        self.dap
            .send_request("stepOut", json!({ "threadId": thread_id }), &mut self.pending_events)
            .await?;
        self.consume_events();
        Ok(())
    }

    pub async fn stack_trace(&mut self, depth: Option<u32>) -> Result<Value> {
        let thread_id = self
            .current_thread_id
            .ok_or_else(|| anyhow!("Нет активного потока — выполните wait_for_stop"))?;
        let mut args = json!({ "threadId": thread_id });
        if let Some(levels) = depth {
            args["levels"] = json!(levels);
        }
        let body = self
            .dap
            .send_request("stackTrace", args, &mut self.pending_events)
            .await?;
        self.consume_events();
        Ok(body)
    }

    pub async fn evaluate(&mut self, expression: &str, context: Option<&str>) -> Result<Value> {
        let thread_id = self
            .current_thread_id
            .ok_or_else(|| anyhow!("Нет активного потока — выполните wait_for_stop"))?;
        let stack = self.stack_trace(Some(1)).await?;
        let frame_id = stack
            .get("stackFrames")
            .and_then(Value::as_array)
            .and_then(|frames| frames.first())
            .and_then(|frame| frame.get("id"))
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("Не удалось определить текущий stack frame"))?;

        let mut args = json!({
            "expression": expression,
            "frameId": frame_id as u32,
        });
        if let Some(ctx) = context {
            args["context"] = json!(ctx);
        }

        let body = self
            .dap
            .send_request("evaluate", args, &mut self.pending_events)
            .await?;
        self.consume_events();
        Ok(body)
    }

    pub async fn wait_for_stop(&mut self) -> Result<StopReason> {
        let event = self
            .dap
            .wait_for_event(
                &mut self.pending_events,
                |event| event.get("event").and_then(Value::as_str) == Some("stopped"),
            )
            .await?;
        self.apply_event(&event);

        let reason = event
            .get("body")
            .and_then(|body| body.get("reason"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let description = event
            .get("body")
            .and_then(|body| body.get("description"))
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        let thread_id = event
            .get("body")
            .and_then(|body| body.get("threadId"))
            .and_then(Value::as_u64)
            .map(|v| v as u32);

        if thread_id.is_some() {
            self.current_thread_id = thread_id;
        }
        self.state = SessionState::Stopped;

        Ok(StopReason {
            reason,
            description,
            thread_id,
        })
    }

    fn apply_event(&mut self, event: &Value) {
        match event.get("event").and_then(Value::as_str) {
            Some("stopped") => {
                if let Some(thread_id) = event
                    .get("body")
                    .and_then(|body| body.get("threadId"))
                    .and_then(Value::as_u64)
                {
                    self.current_thread_id = Some(thread_id as u32);
                }
                self.state = SessionState::Stopped;
            }
            Some("continued") => {
                self.state = SessionState::Running;
            }
            Some("terminated") => {
                self.state = SessionState::Terminated;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SessionSummary {
    pub id: SessionId,
    pub program: String,
    pub state: SessionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_thread_id: Option<u32>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Initializing,
    Ready,
    Running,
    Stopped,
    Terminated,
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Initializing
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Breakpoint {
    pub line: u32,
    pub verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct StopReason {
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CreateSessionRequest {
    pub program: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub adapter: Option<AdapterSettings>,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct LaunchParameters {
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub stop_on_entry: Option<bool>,
}
