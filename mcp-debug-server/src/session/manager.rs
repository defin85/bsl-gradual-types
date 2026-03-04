use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::SessionState;
use crate::dap::DapClient;
use crate::types::SessionId;

/// Информация о debug сессии
pub struct DebugSession {
    /// Уникальный идентификатор сессии
    pub id: SessionId,

    /// DAP клиент для взаимодействия с debugger
    pub dap_client: DapClient,

    /// Путь к бинарному файлу под отладкой
    pub binary_path: String,

    /// Текущий thread ID (shared ownership с EventProcessor)
    pub current_thread_id: Arc<tokio::sync::Mutex<Option<u32>>>,

    /// Установленные breakpoints: file_path -> [line_numbers]
    pub breakpoints: HashMap<String, Vec<u32>>,

    /// Текущее состояние сессии
    pub state: SessionState,
}

impl DebugSession {
    /// Создать новую debug сессию
    pub fn new(
        id: SessionId,
        dap_client: DapClient,
        binary_path: String,
        current_thread_id: Arc<tokio::sync::Mutex<Option<u32>>>,
    ) -> Self {
        Self {
            id,
            dap_client,
            binary_path,
            current_thread_id,
            breakpoints: HashMap::new(),
            state: SessionState::Initialized,
        }
    }

    /// Установить новое состояние (с проверкой валидности перехода)
    pub fn set_state(&mut self, new_state: SessionState) -> Result<()> {
        if !self.state.can_transition_to(new_state) {
            anyhow::bail!(
                "Invalid state transition: {} -> {}",
                self.state.description(),
                new_state.description()
            );
        }
        self.state = new_state;
        Ok(())
    }

    /// Получить frameId топового stack frame для текущего потока
    ///
    /// Возвращает реальный frameId из текущего приостановленного состояния.
    /// Требуется для DAP операций: evaluate, setVariable, etc.
    ///
    /// # Errors
    ///
    /// - Если нет активного потока (процесс не остановлен)
    /// - Если stackTrace пустой или недоступен
    ///
    /// # Note
    ///
    /// Проверка stopped состояния основана на current_thread_id (обновляется EventProcessor),
    /// а НЕ на session.state (может быть устаревшим).
    pub async fn get_current_frame_id(&mut self) -> anyhow::Result<u32> {
        // Получить thread_id из shared state (обновляется EventProcessor при stopped event)
        let thread_id = {
            let guard = self.current_thread_id.lock().await;
            *guard
        }
        .ok_or_else(|| {
            anyhow::anyhow!("No active thread. Process is not stopped or session not started.")
        })?;

        // Шаг 3: Вызвать DAP stackTrace для получения frames
        let stack_result = self
            .dap_client
            .stack_trace(thread_id)
            .await
            .map_err(|e| anyhow::anyhow!("DAP stackTrace error: {}", e))?;

        // Шаг 4: Извлечь frameId из stackFrames[0] (topmost frame)
        let frame_id = stack_result
            .get("stackFrames")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|frame| frame.get("id"))
            .and_then(|id| id.as_u64())
            .map(|id| id as u32)
            .ok_or_else(|| {
                anyhow::anyhow!("No stack frames available. Process may have terminated.")
            })?;

        tracing::debug!(
            session_id = %self.id,
            thread_id = %thread_id,
            frame_id = %frame_id,
            "Retrieved current frame ID"
        );

        Ok(frame_id)
    }
}

/// Менеджер debug сессий (thread-safe)
pub struct SessionManager {
    /// Хранилище сессий: SessionId -> DebugSession
    /// Arc<RwLock<...>> для concurrent access
    sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
    /// Shared буфер для polling событий
    event_buffer: crate::dap::EventBuffer,
}

impl SessionManager {
    /// Создать новый SessionManager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_buffer: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Создать новую debug сессию
    pub async fn create_session(
        &self,
        binary_path: String,
        adapter_command: String,
    ) -> Result<SessionId> {
        // Резолвить adapter command через auto-discovery
        let resolved_adapter = crate::config::resolve_adapter(&adapter_command);

        // Создать уникальный session ID
        let session_id = SessionId::new();

        // Создать shared current_thread_id для EventProcessor
        let current_thread_id = Arc::new(tokio::sync::Mutex::new(None));

        // Запустить DAP client с EventProcessor
        let mut dap_client = DapClient::spawn(
            &resolved_adapter,
            self.event_buffer.clone(),
            session_id.as_str().to_string(),
            current_thread_id.clone(),
        )
        .await?;

        // Инициализировать DAP сессию
        dap_client.initialize().await?;

        // Создать DebugSession
        let session = DebugSession::new(
            session_id.clone(),
            dap_client,
            binary_path,
            current_thread_id,
        );

        // Сохранить в HashMap (с write lock)
        self.sessions
            .write()
            .await
            .insert(session_id.as_str().to_string(), session);

        Ok(session_id)
    }

    /// Получить список всех активных сессий
    pub async fn list_sessions(&self) -> Vec<(SessionId, SessionState, String)> {
        let sessions = self.sessions.read().await;

        sessions
            .values()
            .map(|s| (s.id.clone(), s.state, s.binary_path.clone()))
            .collect()
    }

    /// Завершить debug сессию
    pub async fn terminate_session(&self, session_id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(mut session) = sessions.remove(session_id.as_str()) {
            // Установить состояние Terminated
            let _ = session.set_state(SessionState::Terminated);
            // DapClient будет автоматически закрыт через Drop
            Ok(())
        } else {
            anyhow::bail!("Session not found: {}", session_id)
        }
    }

    /// Выполнить операцию с сессией (с async closure)
    ///
    /// Пример использования:
    /// ```rust,no_run
    /// # use mcp_debug_server::session::SessionManager;
    /// # async fn example() -> anyhow::Result<()> {
    /// # let manager = SessionManager::new();
    /// # let session_id = manager
    /// #     .create_session("./target/debug/my_app".to_string(), "codelldb".to_string())
    /// #     .await?;
    /// manager
    ///     .with_session(&session_id, |session| {
    ///         Box::pin(async move {
    ///             session.dap_client.next(1).await.map_err(anyhow::Error::from)
    ///         })
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_session<'a, F, R>(&'a self, session_id: &SessionId, f: F) -> Result<R>
    where
        F: for<'b> FnOnce(
            &'b mut DebugSession,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<R>> + Send + 'b>,
        >,
    {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .get_mut(session_id.as_str())
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        f(session).await
    }

    /// Проверить существование сессии
    pub async fn session_exists(&self, session_id: &SessionId) -> bool {
        self.sessions.read().await.contains_key(session_id.as_str())
    }

    /// Получить и очистить все события для сессии (для polling через MCP tools)
    pub async fn poll_events(&self, session_id: &SessionId) -> Vec<serde_json::Value> {
        let mut buffer = self.event_buffer.lock().await;

        buffer
            .get_mut(session_id.as_str())
            .map(|queue| queue.drain(..).collect())
            .unwrap_or_default()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "manager/tests.rs"]
mod tests;
