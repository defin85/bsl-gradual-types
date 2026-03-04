//! Runtime Execution Context для Phase 2 - Context-Aware Resolution
//!
//! Этот модуль предоставляет runtime tracking контекста выполнения,
//! включая директивы компиляции и текущую функцию.

use super::code_location::CompilerDirective;

pub use bsl_types::ContextRequirements;

/// Runtime контекст выполнения для tracking текущей функции/процедуры
///
/// Используется для:
/// - Отслеживания текущей директивы компиляции
/// - Проверки допустимости вызовов методов
/// - Context-aware type inference
///
/// # Примеры
/// ```
/// use bsl_shared::domain::{RuntimeExecutionContext, CompilerDirective, ContextRequirements};
///
/// let mut ctx = RuntimeExecutionContext::new();
/// ctx.current_directive = CompilerDirective::OnServer;
/// assert!(ctx.can_call_method(&ContextRequirements::ServerOnly));
///
/// ctx.current_directive = CompilerDirective::OnClient;
/// assert!(!ctx.can_call_method(&ContextRequirements::ServerOnly));
/// assert!(ctx.can_call_method(&ContextRequirements::Universal));
/// ```
#[derive(Debug, Clone)]
pub struct RuntimeExecutionContext {
    /// Текущая директива компиляции
    pub current_directive: CompilerDirective,
    /// Имя текущей функции/процедуры (если внутри функции)
    pub in_function: Option<String>,
}

impl RuntimeExecutionContext {
    /// Создать новый runtime контекст
    pub fn new() -> Self {
        Self {
            current_directive: CompilerDirective::Unknown,
            in_function: None,
        }
    }

    /// Проверить, можно ли вызвать метод с заданными требованиями
    ///
    /// # Правила:
    /// - OnServer/OnServerNoContext → можно вызывать любые методы
    /// - OnClient → можно вызывать ClientOnly и Universal
    /// - OnClientOnServerNoContext → только Universal
    /// - Unknown → разрешаем все (не блокируем)
    pub fn can_call_method(&self, requirements: &ContextRequirements) -> bool {
        match (&self.current_directive, requirements) {
            // Серверные директивы - можно вызывать всё
            (CompilerDirective::OnServer, _) => true,
            (CompilerDirective::OnServerNoContext, _) => true,

            // Клиентская директива
            (CompilerDirective::OnClient, ContextRequirements::ServerOnly) => false,
            (CompilerDirective::OnClient, ContextRequirements::ClientOnly) => true,
            (CompilerDirective::OnClient, ContextRequirements::Universal) => true,
            (CompilerDirective::OnClient, ContextRequirements::ServerPreferred) => true,

            // Универсальная директива - только Universal методы
            (CompilerDirective::OnClientOnServerNoContext, ContextRequirements::ServerOnly) => {
                false
            }
            (CompilerDirective::OnClientOnServerNoContext, ContextRequirements::ClientOnly) => {
                false
            }
            (CompilerDirective::OnClientOnServerNoContext, ContextRequirements::Universal) => true,
            (
                CompilerDirective::OnClientOnServerNoContext,
                ContextRequirements::ServerPreferred,
            ) => true,

            // Unknown - не блокируем
            (CompilerDirective::Unknown, _) => true,
        }
    }
}

impl Default for RuntimeExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "runtime_context/tests.rs"]
mod tests;
