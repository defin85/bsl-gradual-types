//! DTO структуры для семантической визуализации
//!
//! Milestone 2.12: Real-time Semantic Tree Visualization
//!
//! Эти структуры используются для передачи результатов семантического анализа
//! между LSP Server, Web API и клиентами (VSCode Extension, Web Frontend).
//!
//! Два формата передачи:
//! 1. SemanticTreeDto - сырые данные для клиентского рендеринга
//! 2. RenderedHtmlDto - готовый HTML для простого отображения

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Основные DTO для семантического дерева
// ============================================================================

/// DTO для передачи семантического дерева клиентам
///
/// Содержит полную информацию о структуре программы после семантического анализа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTreeDto {
    /// Путь к анализируемому файлу
    pub file_path: String,

    /// Корневые узлы (procedures, functions, global scope variables)
    pub root_nodes: Vec<SemanticNodeDto>,

    /// Таблица символов: имя → информация о символе
    pub symbol_table: HashMap<String, SymbolInfoDto>,

    /// Граф вызовов функций и процедур
    pub call_graph: Vec<CallEdgeDto>,

    /// Метрики семантического анализа
    pub metrics: SemanticMetricsDto,

    /// Временная метка создания
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// DTO для узла семантического дерева
///
/// Представляет один узел в иерархии программы (procedure, variable, if-statement, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNodeDto {
    /// Тип узла (Procedure, Function, Variable, IfStatement, ForLoop, etc.)
    pub kind: String,

    /// Имя узла (для именованных элементов: переменные, функции, процедуры)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Позиция в исходном коде
    pub location: SourceLocationDto,

    /// Диапазон в исходном коде (для узлов с телом)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<SourceRangeDto>,

    /// Вложенные узлы (тело функции, блок if, etc.)
    pub children: Vec<SemanticNodeDto>,

    /// Дополнительные атрибуты узла (специфичны для типа узла)
    /// Примеры: { "return_type": "Число", "is_export": "true", "parameter_count": "2" }
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub attributes: HashMap<String, String>,
}

/// DTO для символа в таблице символов
///
/// Содержит информацию о переменной, параметре, функции или процедуре
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfoDto {
    /// Имя символа
    pub name: String,

    /// Тип символа (Variable, Parameter, Function, Procedure, Constant)
    pub kind: String,

    /// Разрешённый тип из системы типов (если удалось определить)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_type: Option<TypeResolutionDto>,

    /// Область видимости (Global, Local, Parameter)
    pub scope: String,

    /// Позиция объявления в коде
    pub declaration_location: SourceLocationDto,

    /// Flow-sensitive варианты типа (если тип меняется в условиях)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flow_variants: Vec<FlowTypeVariantDto>,

    /// Дополнительные метаданные (экспорт, директивы компиляции, etc.)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

/// DTO для flow-sensitive вариантов типа
///
/// Когда тип переменной меняется в зависимости от условий (if/else)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTypeVariantDto {
    /// Условие, при котором тип имеет это значение
    /// Пример: "Элемент <> Неопределено"
    pub condition: String,

    /// Тип в этой ветке кода
    pub type_info: TypeResolutionDto,

    /// Диапазон кода, где действует этот тип
    pub range: SourceRangeDto,

    /// Уверенность в этом варианте (0.0 - 1.0)
    pub confidence: f32,
}

/// DTO для рёбра графа вызовов
///
/// Представляет вызов одной функции/процедуры из другой
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdgeDto {
    /// Откуда вызов (имя вызывающей функции/процедуры)
    pub from: String,

    /// Куда вызов (имя вызываемой функции/процедуры)
    pub to: String,

    /// Позиция вызова в коде
    pub location: SourceLocationDto,

    /// Тип вызова (Direct - прямой, Indirect - через переменную)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
}

// ============================================================================
// DTO для типов и метаданных
// ============================================================================

/// DTO для разрешённого типа из системы типов
///
/// Упрощённая версия TypeResolution для передачи клиентам
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeResolutionDto {
    /// Имя типа (Массив, Справочники.Контрагенты, etc.)
    pub name: String,

    /// Категория типа (Platform, Configuration, Runtime, Unknown)
    pub category: String,

    /// Уверенность в типе (Known, Inferred, Unknown)
    pub certainty: String,

    /// Процент уверенности (0-100)
    pub certainty_percent: u8,

    /// Активный фасет (Manager, Object, Reference, Selection, List)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_facet: Option<String>,

    /// Доступные методы (если тип известен)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,

    /// Доступные свойства (если тип известен)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<String>,

    /// Является ли union типом (несколько возможных типов)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_union: Option<bool>,

    /// Компоненты union типа (если is_union = true)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub union_components: Vec<String>,
}

/// DTO для метрик семантического анализа
///
/// Статистика по результатам анализа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMetricsDto {
    /// Количество процедур
    pub procedure_count: usize,

    /// Количество функций
    pub function_count: usize,

    /// Количество переменных
    pub variable_count: usize,

    /// Количество параметров
    pub parameter_count: usize,

    /// Количество типов Known (полностью определены)
    pub known_types: usize,

    /// Количество типов Inferred (выведены)
    pub inferred_types: usize,

    /// Количество типов Unknown (не удалось определить)
    pub unknown_types: usize,

    /// Средняя уверенность типизации (0.0 - 1.0)
    pub average_certainty: f32,

    /// Время анализа (миллисекунды)
    pub analysis_time_ms: u64,

    /// Количество узлов в семантическом дереве
    pub node_count: usize,

    /// Глубина дерева (максимальная вложенность)
    pub tree_depth: usize,

    /// Количество вызовов функций/процедур
    pub call_count: usize,
}

// ============================================================================
// DTO для позиций в коде
// ============================================================================

/// DTO для позиции в исходном коде
///
/// Представляет точку в файле (строка, колонка)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SourceLocationDto {
    /// Номер строки (1-based)
    pub line: u32,

    /// Номер колонки (1-based)
    pub column: u32,
}

/// DTO для диапазона в исходном коде
///
/// Представляет отрезок кода от start до end
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRangeDto {
    /// Начало диапазона
    pub start: SourceLocationDto,

    /// Конец диапазона
    pub end: SourceLocationDto,
}

// ============================================================================
// DTO для готового HTML рендеринга
// ============================================================================

/// DTO для готового HTML рендеринга
///
/// Содержит полностью отрендеренный HTML документ для отображения в webview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedHtmlDto {
    /// Путь к файлу
    pub file_path: String,

    /// Полный HTML документ (включая <html>, <head>, CSS стили)
    pub html: String,

    /// Метрики (для отображения в статусбаре или header)
    pub metrics: SemanticMetricsDto,

    /// Временная метка генерации (ISO 8601)
    pub generated_at: String,

    /// Тема рендеринга (Light, Dark, HighContrast)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

// ============================================================================
// Request/Response обёртки для LSP и Web API
// ============================================================================

/// Запрос семантического дерева (LSP custom request)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSemanticTreeRequest {
    /// URI документа (для LSP) или путь к файлу (для Web API)
    pub uri: String,

    /// Включить граф вызовов
    #[serde(default = "default_true")]
    pub include_call_graph: bool,

    /// Включить flow-sensitive анализ.
    ///
    /// Если поле отсутствует, effective режим определяется внешним адаптером (например,
    /// workspace setting `enableFlowSensitive` в LSP).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_flow_sensitive: Option<bool>,

    /// Максимальная глубина дерева (для оптимизации)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,
}

fn default_true() -> bool {
    true
}

/// Запрос HTML рендеринга (LSP custom request)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSemanticHtmlRequest {
    /// URI документа (для LSP) или путь к файлу (для Web API)
    pub uri: String,

    /// Тема рендеринга (Light, Dark, HighContrast, Auto)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Компактный режим (меньше отступов)
    #[serde(default)]
    pub compact: bool,
}

/// Запрос анализа произвольного кода (Web API POST /api/semantic/analyze)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeCodeRequest {
    /// Исходный код BSL для анализа
    pub code: String,

    /// Формат ответа (json, html)
    #[serde(default = "default_json")]
    pub format: String,

    /// Тема для HTML (если format = html)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

fn default_json() -> String {
    "json".to_string()
}

/// Универсальный ответ для Web API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SemanticVisualizationResponse {
    /// Сырые данные (JSON)
    #[serde(rename = "tree")]
    Tree(SemanticTreeDto),

    /// Готовый HTML
    #[serde(rename = "html")]
    Html(RenderedHtmlDto),
}

// ============================================================================
// Вспомогательные impl для удобства
// ============================================================================

impl SemanticTreeDto {
    /// Создать пустое дерево (для тестов)
    pub fn empty(file_path: String) -> Self {
        Self {
            file_path,
            root_nodes: Vec::new(),
            symbol_table: HashMap::new(),
            call_graph: Vec::new(),
            metrics: SemanticMetricsDto::default(),
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Подсчитать общее количество узлов (рекурсивно)
    pub fn count_nodes(&self) -> usize {
        self.root_nodes.iter().map(|n| n.count_nodes()).sum()
    }

    /// Вычислить максимальную глубину дерева
    pub fn calculate_depth(&self) -> usize {
        self.root_nodes
            .iter()
            .map(|n| n.calculate_depth())
            .max()
            .unwrap_or(0)
    }
}

impl SemanticNodeDto {
    /// Подсчитать узлы рекурсивно
    fn count_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_nodes()).sum::<usize>()
    }

    /// Вычислить глубину поддерева
    fn calculate_depth(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            1 + self
                .children
                .iter()
                .map(|c| c.calculate_depth())
                .max()
                .unwrap_or(0)
        }
    }
}

impl Default for SemanticMetricsDto {
    fn default() -> Self {
        Self {
            procedure_count: 0,
            function_count: 0,
            variable_count: 0,
            parameter_count: 0,
            known_types: 0,
            inferred_types: 0,
            unknown_types: 0,
            average_certainty: 0.0,
            analysis_time_ms: 0,
            node_count: 0,
            tree_depth: 0,
            call_count: 0,
        }
    }
}

impl SourceLocationDto {
    /// Создать позицию из координат
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

impl SourceRangeDto {
    /// Создать диапазон из двух позиций
    pub fn new(start: SourceLocationDto, end: SourceLocationDto) -> Self {
        Self { start, end }
    }
}

#[cfg(test)]
#[path = "semantic_dtos/tests.rs"]
mod tests;
