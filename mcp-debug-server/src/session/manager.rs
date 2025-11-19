use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

use crate::dap::DapClient;
use crate::types::SessionId;
use super::state::SessionState;

/// Информация о debug сессии
pub struct DebugSession {
    /// Уникальный идентификатор сессии
    pub id: SessionId,

    /// DAP клиент для взаимодействия с debugger
    pub dap_client: DapClient,

    /// Путь к бинарному файлу под отладкой
    pub binary_path: String,

    /// Текущий thread ID (если известен)
    pub current_thread_id: Option<u32>,

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
    ) -> Self {
        Self {
            id,
            dap_client,
            binary_path,
            current_thread_id: None,
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
}

/// Менеджер debug сессий (thread-safe)
pub struct SessionManager {
    /// Хранилище сессий: SessionId -> DebugSession
    /// Arc<RwLock<...>> для concurrent access
    sessions: Arc<RwLock<HashMap<String, DebugSession>>>,
}

impl SessionManager {
    /// Создать новый SessionManager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Создать новую debug сессию
    pub async fn create_session(
        &self,
        binary_path: String,
        adapter_command: String,
    ) -> Result<SessionId> {
        // Создать уникальный session ID
        let session_id = SessionId::new();

        // Запустить DAP client
        let mut dap_client = DapClient::spawn(&adapter_command).await?;

        // Инициализировать DAP сессию
        dap_client.initialize().await?;

        // Создать DebugSession
        let session = DebugSession::new(
            session_id.clone(),
            dap_client,
            binary_path,
        );

        // Сохранить в HashMap (с write lock)
        self.sessions.write().await.insert(
            session_id.as_str().to_string(),
            session,
        );

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
    /// ```ignore
    /// manager.with_session(&session_id, |session| async {
    ///     session.dap_client.next(thread_id).await
    /// }).await?;
    /// ```
    pub async fn with_session<'a, F, R>(
        &'a self,
        session_id: &SessionId,
        f: F,
    ) -> Result<R>
    where
        F: for<'b> FnOnce(&'b mut DebugSession) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R>> + Send + 'b>>,
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
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = SessionManager::new();
        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn test_session_state_validation() {
        // Тестируем валидацию state transitions через SessionState API
        // (без создания DebugSession, так как требуется реальный DapClient)
        
        use SessionState::*;
        
        // Тест 1: Valid transition Initialized → Running
        let current_state = Initialized;
        assert!(current_state.can_transition_to(Running));
        
        // Тест 2: Invalid transition Running → Initialized
        let current_state = Running;
        assert!(!current_state.can_transition_to(Initialized));
        
        // Тест 3: Valid transition to Terminated
        assert!(Running.can_transition_to(Terminated));
        assert!(Stopped.can_transition_to(Terminated));
    }
}
