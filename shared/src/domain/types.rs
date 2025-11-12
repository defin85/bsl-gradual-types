//! Core type definitions for the gradual type system

use serde::{Deserialize, Serialize};

// --- RawTypeData and its components ---
// This structure is designed to hold all information from all parsers.

/// Информация о Generic параметрах типа коллекции
///
/// # Примеры
/// - `Массив<T>` — 1 параметр (element)
/// - `Соответствие<K,V>` — 2 параметра (key, value)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenericInfo {
    /// Базовый тип (например, "Массив")
    pub base_type: String,

    /// Количество типовых параметров (1 для Массив, 2 для Соответствие)
    pub type_param_count: usize,

    /// Методы, которые позволяют вывести тип параметра
    pub inference_methods: Vec<InferenceMethodInfo>,
}

/// Информация о методе для вывода Generic типа
///
/// # Примеры
/// - `Массив.Добавить(Значение: T)` — параметр 0 определяет T
/// - `Соответствие.Вставить(Ключ: K, Значение: V)` — параметры 0 и 1 определяют K и V
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceMethodInfo {
    /// Имя метода (например, "Добавить")
    pub method_name: String,

    /// Индексы параметров метода, которые определяют Generic типы
    /// Для Массив.Добавить(Значение) — param_indices = [0]
    /// Для Соответствие.Вставить(Ключ, Значение) — param_indices = [0, 1]
    pub param_indices: Vec<usize>,

    /// Какие Generic параметры выводятся (0 для T в Массив<T>, 0 и 1 для K и V в Соответствие<K,V>)
    pub inferred_type_params: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawTypeData {
    pub name: String,
    pub english_name: String,
    pub description: String,
    pub category: String,
    pub source: RawDataSource,
    pub methods: Vec<RawMethodData>,
    pub properties: Vec<RawPropertyData>,
    pub facets: Vec<FacetKind>,
    pub kind: Option<MetadataKind>,
    pub attributes: Vec<RawAttributeData>,
    pub tabular_sections: Vec<RawTabularSectionData>,
    /// Enum values for platform enumeration types (e.g., "Авто (Auto)", "НеИспользовать (DontUse)")
    pub enum_values: Vec<String>,
    /// Generic метаданные для типов коллекций (Массив<T>, Соответствие<K,V>, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generic_info: Option<GenericInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RawDataSource {
    #[default]
    Platform,
    Configuration,
    UserDefined,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawMethodData {
    pub name: String,
    pub english_name: String,
    pub return_type: String,
    pub params: Vec<RawParamData>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_deprecated: bool,
    #[serde(default)]
    pub is_constructor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawPropertyData {
    pub name: String,
    pub prop_type: String,
    pub is_readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RawParamData {
    pub name: String,
    pub param_type: String,
    pub is_optional: bool,
    #[serde(default)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawAttributeData {
    pub name: String,
    pub attr_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTabularSectionData {
    pub name: String,
    pub attributes: Vec<RawAttributeData>,
}

// --- Core Abstractions (Restored from previous version) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacetKind {
    Manager,     // Создание, поиск (СправочникМенеджер)
    Object,      // Изменяемый объект (СправочникОбъект)
    Reference,   // Ссылка на элемент (СправочникСсылка)
    Metadata,    // Метаданные
    Constructor, // Конструктор
    Collection,  // Коллекция
    Singleton,   // Одиночный объект
    Selection,   // Обход элементов (СправочникВыборка) - из статьи Balyuk & Popova
    List,        // Управление списком в форме (СправочникСписок) - из статьи Balyuk & Popova
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataKind {
    Catalog,
    Document,
    Register,
    Report,
    DataProcessor,
    Enum,
    ChartOfAccounts,
    ChartOfCharacteristicTypes,
    ChartOfCalculationTypes,
    // Регистры (добавлено для config_parser.rs)
    InformationRegister,
    AccumulationRegister,
    AccountingRegister,
    CalculationRegister,
    // Бизнес-процессы и задачи
    BusinessProcess,
    Task,
    // Остальные типы конфигурации
    ExchangePlan,
    Constant,
    CommonModule,
    Role,
    Subsystem,
    Language,
}

impl MetadataKind {
    /// Конвертирует XML тег объекта метаданных в MetadataKind
    pub fn from_xml_tag(tag: &str) -> Option<Self> {
        match tag {
            "Catalog" => Some(MetadataKind::Catalog),
            "Document" => Some(MetadataKind::Document),
            "Enum" => Some(MetadataKind::Enum),
            "Report" => Some(MetadataKind::Report),
            "DataProcessor" => Some(MetadataKind::DataProcessor),
            "ChartOfAccounts" => Some(MetadataKind::ChartOfAccounts),
            "ChartOfCharacteristicTypes" => Some(MetadataKind::ChartOfCharacteristicTypes),
            "ChartOfCalculationTypes" => Some(MetadataKind::ChartOfCalculationTypes),
            "InformationRegister" => Some(MetadataKind::InformationRegister),
            "AccumulationRegister" => Some(MetadataKind::AccumulationRegister),
            "AccountingRegister" => Some(MetadataKind::AccountingRegister),
            "CalculationRegister" => Some(MetadataKind::CalculationRegister),
            "BusinessProcess" => Some(MetadataKind::BusinessProcess),
            "Task" => Some(MetadataKind::Task),
            "ExchangePlan" => Some(MetadataKind::ExchangePlan),
            "Constant" => Some(MetadataKind::Constant),
            "CommonModule" => Some(MetadataKind::CommonModule),
            "Role" => Some(MetadataKind::Role),
            "Subsystem" => Some(MetadataKind::Subsystem),
            "Language" => Some(MetadataKind::Language),
            _ => None,
        }
    }

    /// Возвращает префикс для конфигурационного типа
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::MetadataKind;
    ///
    /// assert_eq!(MetadataKind::Catalog.to_prefix(), "Справочники");
    /// assert_eq!(MetadataKind::Document.to_prefix(), "Документы");
    /// ```
    pub fn to_prefix(&self) -> &'static str {
        match self {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Register => "Регистры",
            MetadataKind::Report => "Отчеты",
            MetadataKind::DataProcessor => "Обработки",
            MetadataKind::Enum => "Перечисления",
            MetadataKind::ChartOfAccounts => "ПланыСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
            MetadataKind::ChartOfCalculationTypes => "ПланыВидовРасчета",
            MetadataKind::InformationRegister => "РегистрыСведений",
            MetadataKind::AccumulationRegister => "РегистрыНакопления",
            MetadataKind::AccountingRegister => "РегистрыБухгалтерии",
            MetadataKind::CalculationRegister => "РегистрыРасчета",
            MetadataKind::BusinessProcess => "БизнесПроцессы",
            MetadataKind::Task => "Задачи",
            MetadataKind::ExchangePlan => "ПланыОбмена",
            MetadataKind::Constant => "Константы",
            MetadataKind::CommonModule => "ОбщиеМодули",
            MetadataKind::Role => "Роли",
            MetadataKind::Subsystem => "Подсистемы",
            MetadataKind::Language => "Языки",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeResolution {
    pub certainty: Certainty,
    pub result: ResolutionResult,
    pub source: ResolutionSource,
    pub metadata: ResolutionMetadata,
    pub active_facet: Option<FacetKind>,
    pub available_facets: Vec<FacetKind>,
}

impl TypeResolution {
    pub fn unknown() -> Self {
        Self {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    pub fn known(concrete: ConcreteType) -> Self {
        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(concrete),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Создать TypeResolution из RawTypeData с сохранением всех метаданных (в т.ч. фасетов)
    pub fn from_raw_type(raw_type: &RawTypeData) -> Self {
        let mut resolution = Self::known(ConcreteType::Platform(PlatformType {
            name: raw_type.name.clone(),
        }));
        // Копируем фасеты из RawTypeData
        resolution.available_facets = raw_type.facets.clone();
        resolution
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Certainty {
    Known,
    Inferred(f32),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolutionResult {
    Concrete(ConcreteType),
    Union(Vec<WeightedType>),
    Intersection(Vec<ConcreteType>),
    Generic(GenericType),
    Nullable(Box<ConcreteType>),
    Dynamic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedType {
    pub type_: ConcreteType,
    pub weight: f32,
}

/// Generic type with type parameters
/// Examples: Массив<Строка>, Соответствие<Строка, Число>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericType {
    pub base_type: String,
    pub type_params: Vec<ConcreteType>,
}

impl GenericType {
    /// Создать типизированный массив: Массив<T>
    pub fn array(element_type: ConcreteType) -> Self {
        Self {
            base_type: "Массив".to_string(),
            type_params: vec![element_type],
        }
    }

    /// Создать типизированное соответствие: Соответствие<K, V>
    pub fn map(key_type: ConcreteType, value_type: ConcreteType) -> Self {
        Self {
            base_type: "Соответствие".to_string(),
            type_params: vec![key_type, value_type],
        }
    }

    /// Создать типизированный список: Список<T>
    pub fn list(element_type: ConcreteType) -> Self {
        Self {
            base_type: "Список".to_string(),
            type_params: vec![element_type],
        }
    }

    /// Создать типизированную структуру: Структура<...>
    pub fn structure(field_types: Vec<ConcreteType>) -> Self {
        Self {
            base_type: "Структура".to_string(),
            type_params: field_types,
        }
    }

    /// Получить тип элемента для коллекций (первый параметр)
    pub fn element_type(&self) -> Option<&ConcreteType> {
        self.type_params.first()
    }
}

/// Информация о глобальной функции (определена в Domain Layer)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalFunctionInfo {
    pub name: String,
    pub english_name: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_description: Option<String>,
    pub polymorphic: bool,
    pub pure: bool,
    pub contexts: Vec<String>,
    pub category: Option<String>,
}

/// Информация о параметре функции
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_name: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Тип строки табличной части конфигурационного объекта
///
/// # Примеры
/// - `СтрокаРаботы` для `Документы.ЗаказНаряды.Работы`
/// - `СтрокаСторон` для `Документы.ДоговорКонтрагента.Стороны`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularRowType {
    /// Полное имя родительского типа (например, "Документы.ЗаказНаряды")
    pub parent_type: String,
    /// Имя табличной части (например, "Работы")
    pub tabular_section_name: String,
    /// Атрибуты строки табличной части
    pub attributes: Vec<RawAttributeData>,
}

impl TabularRowType {
    /// Создаёт новый тип строки табличной части
    pub fn new(
        parent_type: String,
        section_name: String,
        attributes: Vec<RawAttributeData>,
    ) -> Self {
        Self {
            parent_type,
            tabular_section_name: section_name,
            attributes,
        }
    }

    /// Возвращает полное имя типа строки (например, "СтрокаРаботы")
    pub fn get_full_name(&self) -> String {
        format!("Строка{}", self.tabular_section_name)
    }

    /// Возвращает имя атрибута строки по индексу
    pub fn get_attribute_name(&self, index: usize) -> Option<&str> {
        self.attributes.get(index).map(|attr| attr.name.as_str())
    }

    /// Возвращает тип атрибута строки по имени
    pub fn get_attribute_type(&self, name: &str) -> Option<&String> {
        self.attributes
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| &attr.attr_type)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConcreteType {
    Platform(PlatformType),
    Configuration(ConfigurationType),
    Primitive(PrimitiveType),
    Special(SpecialType),
    GlobalFunction(GlobalFunctionInfo),
    TabularRow(TabularRowType),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationType {
    pub kind: MetadataKind,
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub tabular_sections: Vec<TabularSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub type_: String,
    pub is_composite: bool,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabularSection {
    pub name: String,
    pub synonym: Option<String>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Date,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialType {
    Undefined,
    Null,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionSource {
    Static,
    Inferred,
    Annotated,
    Runtime,
    Predicted,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResolutionMetadata {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub notes: Vec<String>,
}

// --- Analysis-related structures ---

#[derive(Debug, Clone, Default)]
pub struct TypeContext {
    pub symbol_table: std::collections::HashMap<String, TypeResolution>,
}

impl TypeContext {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {}

// === PARSE ERROR STRUCTURES (for Milestone 2.18) ===

/// Тип синтаксической ошибки
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    /// Общая ошибка парсинга
    ParseError,
    /// Некорректный синтаксис
    InvalidSyntax,
    /// Отсутствует обязательный токен (например, КонецЕсли)
    MissingToken,
    /// Неожиданный токен
    UnexpectedToken,
}

/// Синтаксическая ошибка из парсера
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseError {
    /// Тип ошибки
    pub error_type: ErrorType,
    /// Сообщение об ошибке
    pub message: String,
    /// Позиция ошибки в исходном коде
    pub span: crate::ir::Span,
}

impl ParseError {
    /// Создать ошибку отсутствующего токена
    pub fn missing_token(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::MissingToken,
            message,
            span,
        }
    }

    /// Создать ошибку некорректного синтаксиса
    pub fn invalid_syntax(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::InvalidSyntax,
            message,
            span,
        }
    }

    /// Создать ошибку неожиданного токена
    pub fn unexpected_token(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::UnexpectedToken,
            message,
            span,
        }
    }

    /// Создать общую ошибку парсинга
    pub fn new_parse_error(message: String, span: crate::ir::Span) -> Self {
        Self {
            error_type: ErrorType::ParseError,
            message,
            span,
        }
    }
}

// === DISPLAY IMPLEMENTATIONS ===

use std::fmt;

impl fmt::Display for ConcreteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcreteType::Platform(platform) => write!(f, "{}", platform.name),
            ConcreteType::Configuration(config) => {
                write!(f, "{}.{}", config.kind.display_name(), config.name)
            }
            ConcreteType::Primitive(primitive) => write!(f, "{}", primitive.display_name()),
            ConcreteType::Special(special) => write!(f, "{}", special.display_name()),
            ConcreteType::GlobalFunction(func) => write!(f, "{}()", func.name),
            ConcreteType::TabularRow(tr) => write!(f, "{}", tr.get_full_name()),
        }
    }
}

impl PrimitiveType {
    pub fn display_name(&self) -> &'static str {
        match self {
            PrimitiveType::String => "Строка",
            PrimitiveType::Number => "Число",
            PrimitiveType::Boolean => "Булево",
            PrimitiveType::Date => "Дата",
        }
    }
}

impl SpecialType {
    pub fn display_name(&self) -> &'static str {
        match self {
            SpecialType::Undefined => "Неопределено",
            SpecialType::Null => "Null",
            SpecialType::Type => "Тип",
        }
    }
}

impl MetadataKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            MetadataKind::Catalog => "Справочники",
            MetadataKind::Document => "Документы",
            MetadataKind::Enum => "Перечисления",
            MetadataKind::Report => "Отчеты",
            MetadataKind::DataProcessor => "Обработки",
            MetadataKind::Register => "Регистры",
            MetadataKind::ChartOfAccounts => "ПланыСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
            MetadataKind::ChartOfCalculationTypes => "ПланыВидовРасчета",
            MetadataKind::InformationRegister => "РегистрыСведений",
            MetadataKind::AccumulationRegister => "РегистрыНакопления",
            MetadataKind::AccountingRegister => "РегистрыБухгалтерии",
            MetadataKind::CalculationRegister => "РегистрыРасчета",
            MetadataKind::BusinessProcess => "БизнесПроцессы",
            MetadataKind::Task => "Задачи",
            MetadataKind::ExchangePlan => "ПланыОбмена",
            MetadataKind::Constant => "Константы",
            MetadataKind::CommonModule => "ОбщиеМодули",
            MetadataKind::Role => "Роли",
            MetadataKind::Subsystem => "Подсистемы",
            MetadataKind::Language => "Языки",
        }
    }
}

impl FacetKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Менеджер",
            FacetKind::Object => "Объект",
            FacetKind::Reference => "Ссылка",
            FacetKind::Metadata => "Метаданные",
            FacetKind::Constructor => "Конструктор",
            FacetKind::Collection => "Коллекция",
            FacetKind::Singleton => "Одиночный",
            FacetKind::Selection => "Выборка",
            FacetKind::List => "Список",
        }
    }

    pub fn platform_suffix(&self) -> &'static str {
        match self {
            FacetKind::Manager => "Менеджер",
            FacetKind::Object => "Объект",
            FacetKind::Reference => "Ссылка",
            FacetKind::Selection => "Выборка",
            FacetKind::List => "Список",
            _ => "",
        }
    }
}

// ============================================================================
// Advanced Type System Utilities (Milestone 2.3)
// ============================================================================

impl ResolutionResult {
    /// Normalize Union types: deduplicate, simplify, and sort
    ///
    /// Examples:
    /// - `String | String` → `String`
    /// - `Number | String | Number` → `Number | String`
    /// - `String | Dynamic` → `Dynamic`
    pub fn normalize_union(types: Vec<WeightedType>) -> Self {
        if types.is_empty() {
            return ResolutionResult::Dynamic;
        }

        // 1. Check for Dynamic - if present, return Dynamic
        if types
            .iter()
            .any(|wt| matches!(wt.type_, ConcreteType::Special(SpecialType::Undefined)))
        {
            return ResolutionResult::Dynamic;
        }

        // 2. Deduplicate and merge weights
        let mut type_map: std::collections::HashMap<String, (ConcreteType, f32)> =
            std::collections::HashMap::new();

        for weighted in types {
            let key = format!("{:?}", weighted.type_); // Simple key based on Debug representation
            type_map
                .entry(key)
                .and_modify(|(_, w)| *w += weighted.weight)
                .or_insert((weighted.type_, weighted.weight));
        }

        // 3. Convert back to Vec and sort by weight (descending)
        let mut normalized: Vec<WeightedType> = type_map
            .into_values()
            .map(|(t, w)| WeightedType {
                type_: t,
                weight: w,
            })
            .collect();

        normalized.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 4. If only one type remains, return Concrete
        if normalized.len() == 1 {
            return ResolutionResult::Concrete(normalized.into_iter().next().unwrap().type_);
        }

        ResolutionResult::Union(normalized)
    }

    /// Create an intersection type with validation
    pub fn intersection(types: Vec<ConcreteType>) -> Self {
        if types.is_empty() {
            return ResolutionResult::Dynamic;
        }

        if types.len() == 1 {
            return ResolutionResult::Concrete(types.into_iter().next().unwrap());
        }

        // Deduplicate
        let mut unique_types = Vec::new();
        for t in types {
            if !unique_types.contains(&t) {
                unique_types.push(t);
            }
        }

        if unique_types.len() == 1 {
            return ResolutionResult::Concrete(unique_types.into_iter().next().unwrap());
        }

        ResolutionResult::Intersection(unique_types)
    }

    /// Create a nullable type (T | Null)
    pub fn nullable(base_type: ConcreteType) -> Self {
        ResolutionResult::Nullable(Box::new(base_type))
    }

    /// Check if this result is nullable
    pub fn is_nullable(&self) -> bool {
        match self {
            ResolutionResult::Nullable(_) => true,
            ResolutionResult::Union(types) => types
                .iter()
                .any(|wt| matches!(wt.type_, ConcreteType::Special(SpecialType::Null))),
            _ => false,
        }
    }

    /// Extract the non-null type from nullable
    pub fn unwrap_nullable(&self) -> Option<&ConcreteType> {
        match self {
            ResolutionResult::Nullable(t) => Some(t),
            _ => None,
        }
    }
}

impl WeightedType {
    /// Create a weighted type with default weight (1.0)
    pub fn new(type_: ConcreteType) -> Self {
        Self { type_, weight: 1.0 }
    }

    /// Create a weighted type with custom weight
    pub fn with_weight(type_: ConcreteType, weight: f32) -> Self {
        Self { type_, weight }
    }
}

impl ConcreteType {
    /// Check if this type is compatible with another for intersection
    pub fn is_intersection_compatible(&self, other: &Self) -> bool {
        // Primitive types cannot be intersected
        if matches!(self, ConcreteType::Primitive(_)) && matches!(other, ConcreteType::Primitive(_))
        {
            return false;
        }

        // Special types (Null, Undefined) cannot be intersected with primitives
        if matches!(self, ConcreteType::Special(_)) || matches!(other, ConcreteType::Special(_)) {
            return false;
        }

        // Platform types can be intersected if they share common facets
        true
    }

    /// Create a primitive string type
    pub fn string() -> Self {
        ConcreteType::Primitive(PrimitiveType::String)
    }

    /// Create a primitive number type
    pub fn number() -> Self {
        ConcreteType::Primitive(PrimitiveType::Number)
    }

    /// Create a primitive boolean type
    pub fn boolean() -> Self {
        ConcreteType::Primitive(PrimitiveType::Boolean)
    }

    /// Create a null type
    pub fn null() -> Self {
        ConcreteType::Special(SpecialType::Null)
    }

    /// Create an undefined type
    pub fn undefined() -> Self {
        ConcreteType::Special(SpecialType::Undefined)
    }
}

// ============================================================================
// Tests for Milestone 2.3: Advanced Type System
// ============================================================================

#[cfg(test)]
mod advanced_types_tests {
    use super::*;

    // === Task 1: Union Types Tests ===

    #[test]
    fn test_union_normalization_deduplicate() {
        // String | String → String
        let types = vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::string()),
        ];

        let result = ResolutionResult::normalize_union(types);

        assert!(matches!(result, ResolutionResult::Concrete(_)));
        if let ResolutionResult::Concrete(ct) = result {
            assert!(matches!(ct, ConcreteType::Primitive(PrimitiveType::String)));
        }
    }

    #[test]
    fn test_union_normalization_sort_by_weight() {
        // Number(0.3) | String(0.7) → String | Number (sorted by weight)
        let types = vec![
            WeightedType::with_weight(ConcreteType::number(), 0.3),
            WeightedType::with_weight(ConcreteType::string(), 0.7),
        ];

        let result = ResolutionResult::normalize_union(types);

        if let ResolutionResult::Union(normalized) = result {
            assert_eq!(normalized.len(), 2);
            // First should be String (higher weight)
            assert!(matches!(
                normalized[0].type_,
                ConcreteType::Primitive(PrimitiveType::String)
            ));
            assert_eq!(normalized[0].weight, 0.7);
            // Second should be Number (lower weight)
            assert!(matches!(
                normalized[1].type_,
                ConcreteType::Primitive(PrimitiveType::Number)
            ));
            assert_eq!(normalized[1].weight, 0.3);
        } else {
            panic!("Expected Union type, got {:?}", result);
        }
    }

    #[test]
    fn test_union_with_dynamic_returns_dynamic() {
        // String | Dynamic → Dynamic
        let types = vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::undefined()),
        ];

        let result = ResolutionResult::normalize_union(types);

        assert!(matches!(result, ResolutionResult::Dynamic));
    }

    #[test]
    fn test_union_merge_weights() {
        // String(0.3) | Number(0.4) | String(0.3) → String(0.6) | Number(0.4)
        let types = vec![
            WeightedType::with_weight(ConcreteType::string(), 0.3),
            WeightedType::with_weight(ConcreteType::number(), 0.4),
            WeightedType::with_weight(ConcreteType::string(), 0.3),
        ];

        let result = ResolutionResult::normalize_union(types);

        if let ResolutionResult::Union(normalized) = result {
            assert_eq!(normalized.len(), 2);
            // String weight should be merged: 0.3 + 0.3 = 0.6
            let string_type = normalized
                .iter()
                .find(|wt| matches!(wt.type_, ConcreteType::Primitive(PrimitiveType::String)))
                .expect("String type should be present");
            assert_eq!(string_type.weight, 0.6);
        } else {
            panic!("Expected Union type, got {:?}", result);
        }
    }

    // === Task 2: Intersection Types Tests ===

    #[test]
    fn test_intersection_deduplicate() {
        let types = vec![ConcreteType::string(), ConcreteType::string()];

        let result = ResolutionResult::intersection(types);

        // Должно вернуть Concrete (один тип после дедупликации)
        assert!(matches!(result, ResolutionResult::Concrete(_)));
    }

    #[test]
    fn test_intersection_multiple_types() {
        let types = vec![ConcreteType::string(), ConcreteType::number()];

        let result = ResolutionResult::intersection(types);

        if let ResolutionResult::Intersection(inter_types) = result {
            assert_eq!(inter_types.len(), 2);
        } else {
            panic!("Expected Intersection type, got {:?}", result);
        }
    }

    #[test]
    fn test_intersection_empty_returns_dynamic() {
        let types: Vec<ConcreteType> = vec![];

        let result = ResolutionResult::intersection(types);

        assert!(matches!(result, ResolutionResult::Dynamic));
    }

    // === Task 3: Generic Types Tests ===

    #[test]
    fn test_generic_array() {
        let array = GenericType::array(ConcreteType::string());

        assert_eq!(array.base_type, "Массив");
        assert_eq!(array.type_params.len(), 1);
        assert!(matches!(
            array.type_params[0],
            ConcreteType::Primitive(PrimitiveType::String)
        ));
    }

    #[test]
    fn test_generic_map() {
        let map = GenericType::map(ConcreteType::string(), ConcreteType::number());

        assert_eq!(map.base_type, "Соответствие");
        assert_eq!(map.type_params.len(), 2);
        assert!(matches!(
            map.type_params[0],
            ConcreteType::Primitive(PrimitiveType::String)
        ));
        assert!(matches!(
            map.type_params[1],
            ConcreteType::Primitive(PrimitiveType::Number)
        ));
    }

    #[test]
    fn test_generic_element_type() {
        let array = GenericType::array(ConcreteType::string());

        let element = array.element_type();
        assert!(element.is_some());
        assert!(matches!(
            element.unwrap(),
            ConcreteType::Primitive(PrimitiveType::String)
        ));
    }

    // === Task 4: Nullable Types Tests ===

    #[test]
    fn test_nullable_creation() {
        let nullable = ResolutionResult::nullable(ConcreteType::string());

        assert!(matches!(nullable, ResolutionResult::Nullable(_)));
    }

    #[test]
    fn test_is_nullable_true() {
        let nullable = ResolutionResult::nullable(ConcreteType::string());

        assert!(nullable.is_nullable());
    }

    #[test]
    fn test_is_nullable_false() {
        let concrete = ResolutionResult::Concrete(ConcreteType::string());

        assert!(!concrete.is_nullable());
    }

    #[test]
    fn test_is_nullable_union_with_null() {
        let union = ResolutionResult::Union(vec![
            WeightedType::new(ConcreteType::string()),
            WeightedType::new(ConcreteType::null()),
        ]);

        assert!(union.is_nullable());
    }

    #[test]
    fn test_unwrap_nullable() {
        let nullable = ResolutionResult::nullable(ConcreteType::string());

        let inner = nullable.unwrap_nullable();
        assert!(inner.is_some());
        assert!(matches!(
            inner.unwrap(),
            ConcreteType::Primitive(PrimitiveType::String)
        ));
    }

    #[test]
    fn test_weighted_type_creation() {
        let weighted = WeightedType::new(ConcreteType::string());

        assert_eq!(weighted.weight, 1.0);
        assert!(matches!(
            weighted.type_,
            ConcreteType::Primitive(PrimitiveType::String)
        ));
    }

    #[test]
    fn test_weighted_type_with_custom_weight() {
        let weighted = WeightedType::with_weight(ConcreteType::string(), 0.75);

        assert_eq!(weighted.weight, 0.75);
    }
}
