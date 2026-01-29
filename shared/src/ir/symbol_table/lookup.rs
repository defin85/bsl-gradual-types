//! Методы поиска и обновления переменных в SymbolTable

use super::{ScopeId, SymbolTable};

impl SymbolTable {
    /// Пометить переменную как инициализированную в указанном scope.
    pub fn mark_variable_initialized(&mut self, scope_id: ScopeId, name: &str) -> bool {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if let Some(var_state) = scope.variables.get_mut(name) {
                var_state.mark_initialized();
                return true;
            }
        }
        false
    }

    /// Поиск переменной в заданной области видимости
    ///
    /// Возвращает VariableState переменной, если она существует в указанном scope.
    /// Не выполняет поиск в родительских scope (для этого используйте `lookup_variable_in_hierarchy`).
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "x".to_string(),
    ///     Span::stub(),
    /// );
    ///
    /// let state = table.lookup_variable(table.root_scope, "x");
    /// assert!(state.is_some());
    /// ```
    pub fn lookup_variable(&self, scope_id: ScopeId, name: &str) -> Option<&crate::ir::types::VariableState> {
        self.scopes
            .get(&scope_id)?
            .variables
            .get(name)
    }

    /// Поиск переменной с подъёмом по цепочке родительских scope
    ///
    /// Ищет переменную начиная с указанного scope и поднимаясь вверх по иерархии
    /// до root scope. Возвращает scope_id где была найдена переменная и её VariableState.
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "globalVar".to_string(),
    ///     Span::stub(),
    /// );
    ///
    /// let child = table.create_scope(table.root_scope);
    /// let result = table.lookup_variable_in_hierarchy(child, "globalVar");
    /// assert!(result.is_some());
    /// ```
    pub fn lookup_variable_in_hierarchy(
        &self,
        scope_id: ScopeId,
        name: &str,
    ) -> Option<(ScopeId, &crate::ir::types::VariableState)> {
        let mut current = Some(scope_id);
        while let Some(sid) = current {
            if let Some(scope) = self.scopes.get(&sid) {
                if let Some(var_state) = scope.variables.get(name) {
                    return Some((sid, var_state));
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        None
    }

    /// Обновление состояния переменной в заданной области видимости с обработкой ошибок
    ///
    /// Альтернатива `mark_variable_initialized()`, возвращающая `Result` для более явной обработки ошибок.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если:
    /// - Scope с указанным ID не существует
    /// - Переменная с указанным именем не найдена в scope
    pub fn mark_variable_initialized_checked(&mut self, scope_id: ScopeId, name: &str) -> Result<(), String> {
        let scope = self
            .scopes
            .get_mut(&scope_id)
            .ok_or_else(|| format!("Scope {:?} not found", scope_id))?;

        let var_state = scope
            .variables
            .get_mut(name)
            .ok_or_else(|| format!("Variable '{}' not found in scope {:?}", name, scope_id))?;

        var_state.mark_initialized();
        Ok(())
    }

    /// Проверка существования переменной в scope
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "x".to_string(),
    ///     Span::stub(),
    /// );
    ///
    /// assert!(table.has_variable(table.root_scope, "x"));
    /// assert!(!table.has_variable(table.root_scope, "y"));
    /// ```
    pub fn has_variable(&self, scope_id: ScopeId, name: &str) -> bool {
        self.scopes
            .get(&scope_id)
            .map(|s| s.variables.contains_key(name))
            .unwrap_or(false)
    }
}
