//! SymbolTable - таблица символов с иерархией scope-ов

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::types::TypeResolution;

use super::span::Span;
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
        resolution: TypeResolution,
        span: Span,
    ) {
        let function_scope = self.find_enclosing_function_scope(current_scope);
        self.register_variable(function_scope, name, resolution, span);
    }

    /// Зарегистрировать переменную без инициализации в function scope (Перем X;)
    ///
    /// Аналогично `register_variable_in_function_scope`, но для объявлений через `Перем`.
    pub fn register_variable_declared_in_function_scope(
        &mut self,
        current_scope: ScopeId,
        name: String,
        resolution: TypeResolution,
        span: Span,
    ) {
        let function_scope = self.find_enclosing_function_scope(current_scope);
        self.register_variable_declared(function_scope, name, resolution, span);
    }

    /// Зарегистрировать переменную в scope
    ///
    /// По умолчанию переменная считается инициализированной (например, присваивание X = 5;).
    /// Для объявления без инициализации (Перем X;) используйте `register_variable_declared`.
    pub fn register_variable(
        &mut self,
        scope_id: ScopeId,
        name: String,
        resolution: TypeResolution,
        span: Span,
    ) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope
                .variables
                .insert(name, VariableState::initialized(resolution, span));
        }
    }

    /// Зарегистрировать переменную без инициализации (Перем X;)
    ///
    /// Используется для объявлений переменных без начального значения.
    pub fn register_variable_declared(
        &mut self,
        scope_id: ScopeId,
        name: String,
        resolution: TypeResolution,
        span: Span,
    ) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope
                .variables
                .insert(name, VariableState::declared(resolution, span));
        }
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

    /// Инициализировать переменную как Generic тип с неизвестными параметрами
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// # use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
    /// let mut table = SymbolTable::new();
    /// table.initialize_as_generic(table.root_scope, "МассивСтрок".to_string(), "Массив".to_string(), 1);
    ///
    /// let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");
    /// assert!(resolution.is_some());
    /// let res = resolution.unwrap();
    /// assert_eq!(res.type_name(), "Массив<Неопределено>");
    /// assert!(matches!(res.certainty, Certainty::Inferred(c) if c == 0.0));
    /// ```
    pub fn initialize_as_generic(
        &mut self,
        scope_id: ScopeId,
        var_name: String,
        base_type: String,
        type_param_count: usize,
    ) {
        // Создаём пустые параметры (неизвестные типы = "?")
        let type_params: Vec<&str> = vec!["?"; type_param_count];

        // Используем TypeResolution::generic() с certainty = 0.0 (параметры неизвестны)
        let resolution = TypeResolution::generic(&base_type, &type_params, 0.0);

        // Регистрируем или обновляем переменную
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            // Если переменная уже зарегистрирована, получаем её span и initialized
            let (span, initialized) = scope
                .variables
                .get(&var_name)
                .map(|vs| (vs.declaration_span, vs.initialized))
                .unwrap_or_else(|| (Span::stub(), true));

            scope.variables.insert(
                var_name,
                VariableState::new(resolution, span, initialized),
            );
        }
    }

    /// Обновить Generic параметр переменной
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// # use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
    /// let mut table = SymbolTable::new();
    /// table.initialize_as_generic(table.root_scope, "МассивСтрок".to_string(), "Массив".to_string(), 1);
    /// table.update_generic_param(table.root_scope, "МассивСтрок", 0, "Строка".to_string());
    ///
    /// let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");
    /// assert!(resolution.is_some());
    /// let res = resolution.unwrap();
    /// assert_eq!(res.type_name(), "Массив<Строка>");
    /// assert!(matches!(res.certainty, Certainty::Known));
    /// ```
    pub fn update_generic_param(
        &mut self,
        scope_id: ScopeId,
        var_name: &str,
        param_index: usize,
        param_type: String,
    ) -> bool {
        use crate::domain::types::{Certainty, ConcreteType, ResolutionResult, SpecialType};

        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if let Some(var_state) = scope.variables.get_mut(var_name) {
                match &mut var_state.resolution.result {
                    ResolutionResult::Generic(gen) => {
                        // Получаем новый ConcreteType для параметра
                        let new_param = TypeResolution::string_to_concrete(&param_type);

                        // Обновляем конкретный параметр (расширяем вектор, если нужно)
                        if param_index < gen.type_params.len() {
                            gen.type_params[param_index] = new_param;
                        } else {
                            gen.type_params.push(new_param);
                        }

                        // Вычисляем новую уверенность
                        // Если ВСЕ параметры известны (не Undefined), то certainty = Known
                        let all_known = gen.type_params.iter().all(|p| {
                            !matches!(p, ConcreteType::Special(SpecialType::Undefined))
                        });
                        var_state.resolution.certainty = if all_known {
                            Certainty::Known
                        } else {
                            Certainty::Inferred(0.5)
                        };

                        return true;
                    }
                    _ => {
                        // Не Generic тип — попробуем конвертировать
                        // (например, если раньше был Inferred, теперь становится Generic)
                        tracing::warn!(
                            "update_generic_param: {} не Generic тип, пропускаем",
                            var_name
                        );
                    }
                }
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
