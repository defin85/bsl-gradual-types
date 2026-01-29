//! Методы регистрации переменных в SymbolTable

use super::{ScopeId, SymbolTable};
use crate::ir::span::Span;
use crate::ir::types::VariableState;

impl SymbolTable {
    /// Зарегистрировать переменную в function scope (не в текущем block scope)
    ///
    /// В BSL переменные объявленные внутри if/while/for видны во всей функции.
    /// Этот метод автоматически находит правильный scope для регистрации.
    ///
    /// # Параметры
    ///
    /// - `current_scope` - текущий scope (может быть Block внутри функции)
    /// - `name` - имя переменной
    /// - `resolution` - тип переменной
    /// - `span` - позиция в коде
    ///
    /// # Примеры
    ///
    /// ```text
    /// Процедура Тест()
    ///     Если Истина Тогда
    ///         Х = 1;  // Х регистрируется в scope Тест(), не в scope Если
    ///     КонецЕсли;
    ///     // Х доступен здесь!
    /// КонецПроцедуры
    /// ```
    pub fn register_variable_in_function_scope(
        &mut self,
        current_scope: ScopeId,
        name: String,
        span: Span,
    ) {
        let function_scope = self.find_enclosing_function_scope(current_scope);
        self.register_variable(function_scope, name, span);
    }

    /// Зарегистрировать переменную без инициализации в function scope (Перем X;)
    ///
    /// Аналогично `register_variable_in_function_scope`, но для объявлений через `Перем`.
    pub fn register_variable_declared_in_function_scope(
        &mut self,
        current_scope: ScopeId,
        name: String,
        span: Span,
    ) {
        let function_scope = self.find_enclosing_function_scope(current_scope);
        self.register_variable_declared(function_scope, name, span);
    }

    /// Зарегистрировать переменную в scope
    ///
    /// По умолчанию переменная считается инициализированной (например, присваивание X = 5;).
    /// Для объявления без инициализации (Перем X;) используйте `register_variable_declared`.
    pub fn register_variable(&mut self, scope_id: ScopeId, name: String, span: Span) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope
                .variables
                .insert(name, VariableState::initialized(span));
        }
    }

    /// Зарегистрировать переменную без инициализации (Перем X;)
    ///
    /// Используется для объявлений переменных без начального значения.
    pub fn register_variable_declared(&mut self, scope_id: ScopeId, name: String, span: Span) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.variables.insert(name, VariableState::declared(span));
        }
    }
}
