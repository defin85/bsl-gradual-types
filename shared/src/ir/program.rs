//! SemanticProgram - корневая структура IR
//!
//! Содержит структуру программы и методы для работы с семантическим представлением.

use serde::{Deserialize, Serialize};

use super::cfg::ControlFlowGraph;
use super::span::SourceInfo;
use super::symbol_table::{ScopeId, SymbolTable};
use super::types::{SemanticNode, SemanticNodeKind};

/// Семантическое представление программы
///
/// Не зависит от конкретного парсера и представляет программу
/// в терминах, необходимых для типового анализа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticProgram {
    /// Таблица символов (переменные, функции)
    pub symbols: SymbolTable,

    /// Семантические узлы программы
    pub nodes: Vec<SemanticNode>,

    /// Информация об исходном файле
    pub source_info: SourceInfo,

    /// Граф потока управления (для flow-sensitive анализа)
    pub cfg: Option<ControlFlowGraph>,
}

impl SemanticProgram {
    /// Создать новую пустую программу
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            nodes: Vec::new(),
            source_info: SourceInfo {
                path: String::new(),
                content_hash: 0,
            },
            cfg: None,
        }
    }

    /// Найти семантический узел по позиции в исходном коде
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SemanticProgram, Span};
    /// # let program = SemanticProgram::new();
    /// if let Some(node) = program.find_node_at_position(10, 5) {
    ///     println!("Нашли узел на строке 10, колонке 5");
    /// }
    /// ```
    pub fn find_node_at_position(&self, line: u32, column: u32) -> Option<&SemanticNode> {
        use tracing::debug;

        // Возвращаем САМЫЙ МАЛЕНЬКИЙ узел, содержащий позицию
        // (наиболее специфичный, самый вложенный)
        // Это важно для hover: если есть Assignment и вложенный FunctionCall,
        // курсор на вызове метода должен показывать FunctionCall, а не Assignment

        debug!("find_node_at_position: line={}, col={}", line, column);

        let candidates: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| node.span.contains(line, column))
            .collect();

        debug!(
            "Found {} candidates for position {}:{}",
            candidates.len(),
            line,
            column
        );
        for (i, node) in candidates.iter().enumerate() {
            let type_name = match &node.kind {
                SemanticNodeKind::Assignment { variable, .. } => {
                    format!("Assignment({})", variable)
                }
                SemanticNodeKind::VariableAccess { name } => format!("VariableAccess({})", name),
                SemanticNodeKind::FunctionCall {
                    function_name,
                    object_name,
                    ..
                } => format!(
                    "FunctionCall({}.{})",
                    object_name.as_deref().unwrap_or("?"),
                    function_name
                ),
                SemanticNodeKind::MemberAccess {
                    object_name,
                    member_name,
                    ..
                } => format!(
                    "MemberAccess({}.{})",
                    object_name.as_deref().unwrap_or("?"),
                    member_name
                ),
                SemanticNodeKind::VariableDeclaration { name, .. } => format!("VarDecl({})", name),
                _ => format!("{:?}", node.kind),
            };
            debug!("  [{}] {} span={:?}", i, type_name, node.span);
        }

        let result = self
            .nodes
            .iter()
            .filter(|node| node.span.contains(line, column))
            .min_by_key(|node| {
                // Сортируем по размеру span (площадь покрытия)
                let lines = node.span.end_line.saturating_sub(node.span.start_line);
                let cols = if lines == 0 {
                    node.span.end_column.saturating_sub(node.span.start_column)
                } else {
                    // Для многострочных span используем количество строк
                    lines * 1000
                };

                // MILESTONE 2.11: Приоритизация по типу узла
                // Когда несколько узлов имеют одинаковый размер span,
                // выбираем более специфичный тип (меньшее число = выше приоритет)
                let type_priority = match &node.kind {
                    SemanticNodeKind::FunctionCall { .. } => 0, // Высший приоритет
                    SemanticNodeKind::MemberAccess { .. } => 1, // Высокий приоритет
                    SemanticNodeKind::VariableAccess { .. } => 2, // Высокий приоритет
                    SemanticNodeKind::VariableDeclaration { .. } => 3, // Средний приоритет
                    SemanticNodeKind::Assignment { .. } => 10,  // Низкий приоритет
                    _ => 5,                                     // Остальные - средний приоритет
                };

                // Сортировка: сначала по размеру span, затем по приоритету типа
                (lines * 1000 + cols, type_priority)
            });

        if let Some(node) = result {
            let type_name = match &node.kind {
                SemanticNodeKind::Assignment { variable, .. } => {
                    format!("Assignment({})", variable)
                }
                SemanticNodeKind::VariableAccess { name } => format!("VariableAccess({})", name),
                SemanticNodeKind::FunctionCall {
                    function_name,
                    object_name,
                    ..
                } => format!(
                    "FunctionCall({}.{})",
                    object_name.as_deref().unwrap_or("?"),
                    function_name
                ),
                _ => format!("{:?}", node.kind),
            };
            debug!("Selected node: {} span={:?}", type_name, node.span);
        } else {
            debug!("No node found for position {}:{}", line, column);
        }

        result
    }

    /// Получить scope по ID
    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&super::symbol_table::Scope> {
        self.symbols.scopes.get(&scope_id)
    }

    /// Получить переменную в scope (с поиском в родительских scope)
    ///
    /// Поиск идёт от текущего scope вверх по иерархии до root
    pub fn resolve_variable(&self, name: &str, scope_id: ScopeId) -> Option<&super::types::VariableState> {
        let mut current_scope_id = Some(scope_id);

        while let Some(sid) = current_scope_id {
            if let Some(scope) = self.get_scope(sid) {
                if let Some(var_state) = scope.variables.get(name) {
                    return Some(var_state);
                }
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Извлечь имя переменной из узла IR (если узел содержит переменную)
    ///
    /// # Returns
    /// - `Some(name)` если узел содержит переменную с именем
    /// - `None` если узел не содержит переменную или это сложное выражение
    ///
    /// # Phase 4: DRY refactoring
    /// Устраняет дублирование между find_variable_at_position() и find_variable_with_scope()
    fn extract_variable_name(node: &SemanticNode) -> Option<String> {
        match &node.kind {
            // Присваивание: МассивДанных = Новый Массив
            SemanticNodeKind::Assignment { variable, .. } => Some(variable.clone()),

            // Доступ к переменной: МассивДанных
            SemanticNodeKind::VariableAccess { name } => Some(name.clone()),

            // Доступ к члену: МассивДанных.Добавить
            SemanticNodeKind::MemberAccess {
                object_name: Some(obj_name),
                ..
            } => Some(obj_name.clone()),
            SemanticNodeKind::MemberAccess {
                object_name: None, ..
            } => None,

            // Объявление переменной: Перем МассивДанных
            SemanticNodeKind::VariableDeclaration { name, .. } => Some(name.clone()),

            // Вызов функции: может быть вызов метода переменной
            SemanticNodeKind::FunctionCall {
                object_name: Some(obj_name),
                ..
            } => Some(obj_name.clone()),
            SemanticNodeKind::FunctionCall {
                object_name: None, ..
            } => None,

            // Для остальных узлов не поддерживаем
            _ => None,
        }
    }

    /// Найти переменную в позиции (line, column) для Inline Scope Analysis
    ///
    /// Используется для LSP hover: находит переменную в scope в указанной позиции
    /// и возвращает её имя и тип.
    ///
    /// # Алгоритм
    ///
    /// 1. Найти SemanticNode в позиции (line, column)
    /// 2. Получить scope_id узла
    /// 3. Извлечь имя переменной из узла (Assignment, MemberAccess, VariableDeclaration)
    /// 4. Вызвать resolve_variable() для поиска в scope hierarchy
    /// 5. Вернуть (имя_переменной, VariableState)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_shared::ir::SemanticProgram;
    /// let program = SemanticProgram::new();
    /// // Предположим, в программе есть: МассивДанных = Новый Массив;
    /// if let Some((var_name, state)) = program.find_variable_at_position(5, 10) {
    ///     assert_eq!(var_name, "МассивДанных");
    ///     assert!(state.initialized);
    /// }
    /// ```
    pub fn find_variable_at_position(&self, line: u32, column: u32) -> Option<(String, super::types::VariableState)> {
        // 1. Находим узел в позиции
        let node = self.find_node_at_position(line, column)?;

        // 2. Получаем scope узла
        let scope_id = node.scope_id;

        // 3. Извлекаем имя переменной из узла (Phase 4: используем общий метод)
        let var_name = Self::extract_variable_name(node)?;

        // 4. Ищем переменную в scope (с поиском в родительских scope)
        let state = self.resolve_variable(&var_name, scope_id)?;

        // 5. Возвращаем результат
        Some((var_name, state.clone()))
    }

    /// Найти переменную по позиции с возвратом scope_id
    ///
    pub fn find_variable_with_scope(&self, line: u32, column: u32) -> Option<(String, super::types::VariableState, ScopeId)> {
        // 1. Находим узел в позиции
        let node = self.find_node_at_position(line, column)?;

        // 2. Получаем scope узла
        let scope_id = node.scope_id;

        // 3. Извлекаем имя переменной из узла (Phase 4: используем общий метод)
        let var_name = Self::extract_variable_name(node)?;

        // 4. Ищем переменную в scope (с поиском в родительских scope)
        let state = self.resolve_variable(&var_name, scope_id)?;

        // 5. Возвращаем результат с scope_id
        Some((var_name, state.clone(), scope_id))
    }
}

impl Default for SemanticProgram {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE: extract_runtime_types() удалена в Milestone 2.9
// Используем Inline Scope Analysis вместо загрузки runtime типов в TypeRepository
// См. SemanticProgram::find_variable_at_position() для нового подхода
