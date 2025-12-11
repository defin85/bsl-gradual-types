//! SymbolTable - таблица символов с иерархией scope-ов

mod generics;
mod lookup;
mod registration;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::types::TypeResolution;

use super::types::{FunctionSignature, VariableState};

/// Идентификатор scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeId(pub usize);

/// Тип области видимости (scope)
///
/// Определяет семантику scope для корректной регистрации переменных:
/// - `Global` - корневой scope (root)
/// - `Function` - тело функции/процедуры (переменные видны во всём теле)
/// - `Block` - блоки if/while/for (НЕ создают отдельную область видимости для переменных в BSL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScopeKind {
    /// Корневой scope (root)
    #[default]
    Global,
    /// Тело функции/процедуры — переменные регистрируются здесь
    Function,
    /// Блоки if/while/for — НЕ создают отдельную область видимости для переменных
    Block,
}

/// Область видимости (scope)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    /// ID scope
    pub id: ScopeId,

    /// Родительский scope (None для root)
    pub parent: Option<ScopeId>,

    /// Переменные в этом scope (имя -> состояние переменной)
    pub variables: HashMap<String, VariableState>,

    /// Дочерние scope-ы
    pub children: Vec<ScopeId>,

    /// Тип scope (для определения куда регистрировать переменные)
    #[serde(default)]
    pub kind: ScopeKind,
}

/// Таблица символов с иерархией scope-ов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTable {
    /// Все scope-ы программы
    pub scopes: HashMap<ScopeId, Scope>,

    /// Корневой scope (глобальная область видимости)
    pub root_scope: ScopeId,

    /// Глобальные функции
    pub global_functions: HashMap<String, FunctionSignature>,

    /// Глобальные процедуры
    pub global_procedures: HashMap<String, FunctionSignature>,
}

impl SymbolTable {
    /// Создать новую таблицу символов с root scope
    pub fn new() -> Self {
        let root_scope = ScopeId(0);
        let mut scopes = HashMap::new();
        scopes.insert(
            root_scope,
            Scope {
                id: root_scope,
                parent: None,
                variables: HashMap::new(),
                children: Vec::new(),
                kind: ScopeKind::Global,
            },
        );

        Self {
            scopes,
            root_scope,
            global_functions: HashMap::new(),
            global_procedures: HashMap::new(),
        }
    }

    /// Создать новый дочерний scope с типом Block (по умолчанию)
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, ScopeId};
    /// let mut table = SymbolTable::new();
    /// let child_scope = table.create_scope(table.root_scope);
    /// ```
    pub fn create_scope(&mut self, parent: ScopeId) -> ScopeId {
        self.create_scope_with_kind(parent, ScopeKind::Block)
    }

    /// Создать новый дочерний scope с указанным типом
    ///
    /// # Параметры
    ///
    /// - `parent` - родительский scope
    /// - `kind` - тип scope (Global, Function, Block)
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, ScopeId, ScopeKind};
    /// let mut table = SymbolTable::new();
    /// let func_scope = table.create_scope_with_kind(table.root_scope, ScopeKind::Function);
    /// ```
    pub fn create_scope_with_kind(&mut self, parent: ScopeId, kind: ScopeKind) -> ScopeId {
        let id = ScopeId(self.scopes.len());
        let scope = Scope {
            id,
            parent: Some(parent),
            variables: HashMap::new(),
            children: Vec::new(),
            kind,
        };
        self.scopes.insert(id, scope);

        // Добавляем в дочерние для родителя
        if let Some(parent_scope) = self.scopes.get_mut(&parent) {
            parent_scope.children.push(id);
        }

        id
    }

    /// Найти ближайший function scope (или global) для данного scope
    ///
    /// В BSL переменные видны во всём теле функции, а не только в локальном блоке.
    /// Этот метод находит правильный scope для регистрации переменных.
    ///
    /// # Возвращает
    ///
    /// - `ScopeId` ближайшего Function или Global scope
    /// - Если не найден — возвращает переданный scope_id
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, ScopeKind};
    /// let mut table = SymbolTable::new();
    /// let func_scope = table.create_scope_with_kind(table.root_scope, ScopeKind::Function);
    /// let block_scope = table.create_scope_with_kind(func_scope, ScopeKind::Block);
    /// // Из Block scope найдём Function scope
    /// assert_eq!(table.find_enclosing_function_scope(block_scope), func_scope);
    /// ```
    pub fn find_enclosing_function_scope(&self, scope_id: ScopeId) -> ScopeId {
        let mut current = scope_id;

        loop {
            if let Some(scope) = self.scopes.get(&current) {
                // Function или Global — это "владеющие" scopes для переменных
                if matches!(scope.kind, ScopeKind::Function | ScopeKind::Global) {
                    return current;
                }
                // Иначе поднимаемся к родителю
                if let Some(parent) = scope.parent {
                    current = parent;
                } else {
                    // Достигли root без Function — вернуть текущий
                    return current;
                }
            } else {
                // Scope не найден — вернуть исходный
                return scope_id;
            }
        }
    }

    /// Получить родительский scope
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// let mut table = SymbolTable::new();
    /// let child = table.create_scope(table.root_scope);
    ///
    /// assert_eq!(table.get_parent_scope(child), Some(table.root_scope));
    /// assert_eq!(table.get_parent_scope(table.root_scope), None);
    /// ```
    pub fn get_parent_scope(&self, scope_id: ScopeId) -> Option<ScopeId> {
        self.scopes.get(&scope_id)?.parent
    }

    /// Зарегистрировать глобальную функцию
    pub fn register_function(&mut self, signature: FunctionSignature) {
        self.global_functions
            .insert(signature.name.clone(), signature);
    }

    /// Зарегистрировать глобальную процедуру
    pub fn register_procedure(&mut self, signature: FunctionSignature) {
        self.global_procedures
            .insert(signature.name.clone(), signature);
    }

    /// Поиск функции по имени
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, FunctionSignature, Parameter};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// // Phase 3: return_type теперь Option<TypeResolution>
    /// table.register_function(FunctionSignature {
    ///     name: "МояФункция".to_string(),
    ///     params: vec![],
    ///     return_type: Some(TypeResolution::explicit("Число")),
    ///     is_export: false,
    /// });
    ///
    /// assert!(table.find_function("МояФункция").is_some());
    /// ```
    pub fn find_function(&self, name: &str) -> Option<&FunctionSignature> {
        self.global_functions.get(name)
    }

    /// Поиск процедуры по имени
    pub fn find_procedure(&self, name: &str) -> Option<&FunctionSignature> {
        self.global_procedures.get(name)
    }

    /// Получить количество глобальных функций
    pub fn functions_count(&self) -> usize {
        self.global_functions.len()
    }

    /// Получить количество глобальных процедур
    pub fn procedures_count(&self) -> usize {
        self.global_procedures.len()
    }

    /// Итератор по всем глобальным функциям
    ///
    /// Возвращает итератор по парам (имя функции, FunctionSignature)
    pub fn iter_functions(&self) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        self.global_functions.iter()
    }

    /// Итератор по всем глобальным процедурам
    ///
    /// Возвращает итератор по парам (имя процедуры, FunctionSignature)
    pub fn iter_procedures(&self) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        self.global_procedures.iter()
    }

    /// Итератор по всем переменным в scope
    ///
    /// Возвращает итератор по парам (имя переменной, TypeResolution) для указанного scope.
    /// Не включает переменные из родительских scope.
    pub fn variables_in_scope(
        &self,
        scope_id: ScopeId,
    ) -> Option<impl Iterator<Item = (&String, &TypeResolution)>> {
        self.scopes.get(&scope_id).map(|scope| {
            scope
                .variables
                .iter()
                .map(|(name, var_state)| (name, &var_state.resolution))
        })
    }

    /// Итератор по всем scopes в таблице символов
    ///
    /// Используется для конвертации в DTO и других операций, требующих доступа ко всем scopes.
    pub fn iter_all_scopes(&self) -> impl Iterator<Item = (&ScopeId, &Scope)> {
        self.scopes.iter()
    }

    /// Количество scopes в таблице символов
    pub fn scopes_count(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
