//! Методы поиска и обновления переменных в SymbolTable

use super::{ScopeId, SymbolTable};
use crate::domain::types::TypeResolution;

impl SymbolTable {
    /// Получить тип переменной из текущего или родительского scope
    pub fn get_variable_type(&self, scope_id: ScopeId, name: &str) -> Option<TypeResolution> {
        let mut current_scope_id = Some(scope_id);

        while let Some(sid) = current_scope_id {
            if let Some(scope) = self.scopes.get(&sid) {
                if let Some(var_state) = scope.variables.get(name) {
                    return Some(var_state.resolution.clone());
                }
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Обновить тип переменной в указанном scope
    ///
    /// При обновлении типа также помечает переменную как инициализированную.
    pub fn update_variable_type(
        &mut self,
        scope_id: ScopeId,
        name: String,
        new_resolution: TypeResolution,
    ) -> bool {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if let Some(var_state) = scope.variables.get_mut(&name) {
                var_state.resolution = new_resolution;
                var_state.mark_initialized();
                return true;
            }
        }
        false
    }

    /// Поиск переменной в заданной области видимости
    ///
    /// Возвращает TypeResolution переменной, если она существует в указанном scope.
    /// Не выполняет поиск в родительских scope (для этого используйте `lookup_variable_in_hierarchy`).
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "x".to_string(),
    ///     TypeResolution::explicit("Число"),
    ///     Span::stub(),
    /// );
    ///
    /// let resolution = table.lookup_variable(table.root_scope, "x");
    /// assert!(resolution.is_some());
    /// ```
    pub fn lookup_variable(&self, scope_id: ScopeId, name: &str) -> Option<&TypeResolution> {
        self.scopes
            .get(&scope_id)?
            .variables
            .get(name)
            .map(|var_state| &var_state.resolution)
    }

    /// Поиск переменной с подъёмом по цепочке родительских scope
    ///
    /// Ищет переменную начиная с указанного scope и поднимаясь вверх по иерархии
    /// до root scope. Возвращает scope_id где была найдена переменная и её TypeResolution.
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "globalVar".to_string(),
    ///     TypeResolution::explicit("Число"),
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
    ) -> Option<(ScopeId, &TypeResolution)> {
        let mut current = Some(scope_id);
        while let Some(sid) = current {
            if let Some(scope) = self.scopes.get(&sid) {
                if let Some(var_state) = scope.variables.get(name) {
                    return Some((sid, &var_state.resolution));
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        None
    }

    /// Обновление типа переменной в заданной области видимости с обработкой ошибок
    ///
    /// Альтернатива `update_variable_type()`, возвращающая `Result` для более явной обработки ошибок.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если:
    /// - Scope с указанным ID не существует
    /// - Переменная с указанным именем не найдена в scope
    pub fn update_variable_type_checked(
        &mut self,
        scope_id: ScopeId,
        name: &str,
        resolution: TypeResolution,
    ) -> Result<(), String> {
        let scope = self
            .scopes
            .get_mut(&scope_id)
            .ok_or_else(|| format!("Scope {:?} not found", scope_id))?;

        let var_state = scope
            .variables
            .get_mut(name)
            .ok_or_else(|| format!("Variable '{}' not found in scope {:?}", name, scope_id))?;

        var_state.resolution = resolution;
        var_state.mark_initialized();
        Ok(())
    }

    /// Проверка существования переменной в scope
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "x".to_string(),
    ///     TypeResolution::explicit("Число"),
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
