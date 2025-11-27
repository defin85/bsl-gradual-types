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
    /// Пути к модулям для конфигурационных типов (Milestone 3.14)
    /// Используется для Go To Definition навигации к ObjectModule.bsl, ManagerModule.bsl и т.д.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_paths: Option<super::type_definition_location::ModulePaths>,
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
    /// MILESTONE 3.11 Phase 4: Требования к контексту выполнения
    #[serde(default)]
    pub context_requirements: Option<crate::domain::runtime_context::ContextRequirements>,
    /// MILESTONE 3.11 Phase 4: Фасет возвращаемого типа
    #[serde(default)]
    pub return_facet: Option<FacetKind>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MetadataKind {
    #[default]
    Unknown,
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
            MetadataKind::Unknown => "Неизвестный",
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

    /// Возвращает русское название вида метаданных (единственное число)
    ///
    /// Используется для сообщений об ошибках и hover.
    /// В отличие от `to_prefix()`, который возвращает множественное число
    /// для коллекций (например, "Справочники"), этот метод возвращает
    /// единственное число для конкретного объекта (например, "Справочник").
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::MetadataKind;
    ///
    /// assert_eq!(MetadataKind::Catalog.to_russian_name(), "Справочник");
    /// assert_eq!(MetadataKind::Document.to_russian_name(), "Документ");
    /// assert_eq!(MetadataKind::InformationRegister.to_russian_name(), "Регистр сведений");
    /// ```
    pub fn to_russian_name(&self) -> &'static str {
        match self {
            MetadataKind::Unknown => "Объект метаданных",
            MetadataKind::Catalog => "Справочник",
            MetadataKind::Document => "Документ",
            MetadataKind::Enum => "Перечисление",
            MetadataKind::Report => "Отчет",
            MetadataKind::DataProcessor => "Обработка",
            MetadataKind::InformationRegister => "Регистр сведений",
            MetadataKind::AccumulationRegister => "Регистр накопления",
            MetadataKind::AccountingRegister => "Регистр бухгалтерии",
            MetadataKind::CalculationRegister => "Регистр расчета",
            MetadataKind::ChartOfAccounts => "План счетов",
            MetadataKind::ChartOfCharacteristicTypes => "План видов характеристик",
            MetadataKind::ChartOfCalculationTypes => "План видов расчета",
            MetadataKind::BusinessProcess => "Бизнес-процесс",
            MetadataKind::Task => "Задача",
            MetadataKind::ExchangePlan => "План обмена",
            MetadataKind::Constant => "Константа",
            MetadataKind::CommonModule => "Общий модуль",
            MetadataKind::Role => "Роль",
            MetadataKind::Subsystem => "Подсистема",
            MetadataKind::Language => "Язык",
            MetadataKind::Register => "Регистр",
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

    /// Получить местоположение определения типа для Go To Definition
    ///
    /// # Возвращает
    /// - `Some(TypeDefinitionLocation::Primitive)` для примитивных типов
    /// - `Some(TypeDefinitionLocation::Platform)` для платформенных типов
    /// - `Some(TypeDefinitionLocation::Configuration)` для конфигурационных типов
    /// - `None` для Dynamic типов
    pub fn get_definition_location(
        &self,
    ) -> Option<super::type_definition_location::TypeDefinitionLocation> {
        use super::type_definition_location::TypeDefinitionLocation;
        use std::path::PathBuf;

        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(_)) => {
                Some(TypeDefinitionLocation::primitive())
            }

            ResolutionResult::Concrete(ConcreteType::Platform(pt)) => {
                Some(TypeDefinitionLocation::platform(&pt.name))
            }

            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
                // TODO: MILESTONE 3.14 STUB - type_key используется как placeholder для metadata_path.
                // Реальные пути к модулям (ObjectModule.bsl, ManagerModule.bsl) должны
                // получаться из ConfigParser после парсинга Configuration.xml.
                // До интеграции с config parser Go To Definition для конфигурационных
                // типов будет возвращать None (путь не является валидным файлом).
                // См. Milestone 3.12 (config_parser) для интеграции.
                Some(TypeDefinitionLocation::configuration(PathBuf::from(
                    &type_key,
                )))
            }

            ResolutionResult::Concrete(ConcreteType::Special(_)) => {
                // Special типы (Null, Undefined, Type) — нет навигации
                Some(TypeDefinitionLocation::primitive())
            }

            ResolutionResult::Concrete(ConcreteType::GlobalFunction(func)) => {
                // Глобальные функции — платформенные, навигация на документацию
                Some(TypeDefinitionLocation::platform(&func.name))
            }

            ResolutionResult::Concrete(ConcreteType::TabularRow(tr)) => {
                // Строка табличной части — конфигурационный тип
                let type_key = format!("{}.{}", tr.parent_type, tr.tabular_section_name);
                Some(TypeDefinitionLocation::configuration(PathBuf::from(
                    &type_key,
                )))
            }

            ResolutionResult::Generic(gen) => {
                // Для Generic возвращаем location базового типа
                Some(TypeDefinitionLocation::platform(&gen.base_type))
            }

            ResolutionResult::Nullable(inner) => {
                // Для Nullable возвращаем location внутреннего типа
                let inner_resolution = TypeResolution::known(*inner.clone());
                inner_resolution.get_definition_location()
            }

            ResolutionResult::Union(variants) => {
                // Для Union возвращаем location первого (наиболее вероятного) типа
                if let Some(first) = variants.first() {
                    let first_resolution = TypeResolution::known(first.type_.clone());
                    first_resolution.get_definition_location()
                } else {
                    None
                }
            }

            ResolutionResult::Intersection(types) => {
                // Для Intersection возвращаем location первого типа
                if let Some(first) = types.first() {
                    let first_resolution = TypeResolution::known(first.clone());
                    first_resolution.get_definition_location()
                } else {
                    None
                }
            }

            ResolutionResult::Dynamic => None,
        }
    }

    /// Получить местоположение определения типа для Go To Definition
    /// с учётом путей к модулям конфигурации (Milestone 3.14)
    ///
    /// # Параметры
    /// - `module_paths` - Опциональные пути к модулям из RawTypeData
    ///
    /// # Возвращает
    /// - `Some(TypeDefinitionLocation::Configuration)` с реальными путями к модулям
    /// - Делегирует к `get_definition_location()` для остальных типов
    pub fn get_definition_location_with_modules(
        &self,
        module_paths: Option<&super::type_definition_location::ModulePaths>,
    ) -> Option<super::type_definition_location::TypeDefinitionLocation> {
        use super::type_definition_location::TypeDefinitionLocation;
        use std::path::PathBuf;

        match &self.result {
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => {
                let type_key = format!("{}.{}", cfg.kind.to_prefix(), cfg.name);
                let metadata_path = PathBuf::from(&type_key);

                // Если есть module_paths - используем их для реальной навигации
                if let Some(paths) = module_paths {
                    Some(TypeDefinitionLocation::configuration_with_modules(
                        metadata_path,
                        paths.clone(),
                    ))
                } else {
                    // Fallback: старое поведение (stub)
                    Some(TypeDefinitionLocation::configuration(metadata_path))
                }
            }

            ResolutionResult::Concrete(ConcreteType::TabularRow(tr)) => {
                let type_key = format!("{}.{}", tr.parent_type, tr.tabular_section_name);
                let metadata_path = PathBuf::from(&type_key);

                // TabularRow также может использовать module_paths родителя
                if let Some(paths) = module_paths {
                    Some(TypeDefinitionLocation::configuration_with_modules(
                        metadata_path,
                        paths.clone(),
                    ))
                } else {
                    Some(TypeDefinitionLocation::configuration(metadata_path))
                }
            }

            // Для остальных типов делегируем стандартному методу
            _ => self.get_definition_location(),
        }
    }

    // ============================================================================
    // Milestone 3.18 Phase 1: Constructors and Utility Methods
    // ============================================================================

    /// Известный примитивный тип (Число, Строка, Булево, Дата)
    ///
    /// Для нераспознанных имён создаёт Platform тип с этим именем.
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::TypeResolution;
    ///
    /// let string = TypeResolution::primitive("Строка");
    /// assert_eq!(string.type_name(), "Строка");
    ///
    /// let number = TypeResolution::primitive("Number");
    /// assert_eq!(number.type_name(), "Число");
    /// ```
    pub fn primitive(type_name: &str) -> Self {
        let lower = type_name.to_lowercase();
        let concrete = match lower.as_str() {
            "число" | "number" => ConcreteType::Primitive(PrimitiveType::Number),
            "строка" | "string" => ConcreteType::Primitive(PrimitiveType::String),
            "булево" | "boolean" => ConcreteType::Primitive(PrimitiveType::Boolean),
            "дата" | "date" => ConcreteType::Primitive(PrimitiveType::Date),
            _ => ConcreteType::Platform(PlatformType {
                name: type_name.to_string(),
            }),
        };

        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(concrete),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Выведенный тип с уровнем уверенности (0.0 - 1.0)
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::{TypeResolution, Certainty};
    ///
    /// let inferred = TypeResolution::inferred("Число", 0.75);
    /// assert_eq!(inferred.certainty, Certainty::Inferred(0.75));
    /// ```
    pub fn inferred(type_name: &str, confidence: f32) -> Self {
        let mut resolution = Self::primitive(type_name);
        resolution.certainty = Certainty::Inferred(confidence);
        resolution.source = ResolutionSource::Inferred;
        resolution
    }

    /// Тип метаданных (справочник, документ и т.д.) с опциональным фасетом
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::{TypeResolution, MetadataKind, FacetKind};
    ///
    /// let catalog = TypeResolution::metadata_type(
    ///     MetadataKind::Catalog,
    ///     "Контрагенты",
    ///     Some(FacetKind::Manager)
    /// );
    /// assert_eq!(catalog.active_facet, Some(FacetKind::Manager));
    /// ```
    pub fn metadata_type(kind: MetadataKind, name: &str, facet: Option<FacetKind>) -> Self {
        let available_facets = if facet.is_some() {
            vec![facet.unwrap()]
        } else {
            vec![]
        };

        Self {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: name.to_string(),
                facet,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: facet,
            available_facets,
        }
    }

    /// Проверка: тип Unknown?
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::TypeResolution;
    ///
    /// assert!(TypeResolution::unknown().is_unknown());
    /// assert!(!TypeResolution::primitive("Строка").is_unknown());
    /// ```
    pub fn is_unknown(&self) -> bool {
        matches!(self.certainty, Certainty::Unknown)
    }

    /// Получить имя типа как String (для совместимости)
    ///
    /// # Примеры
    /// ```
    /// use bsl_shared::domain::types::{TypeResolution, MetadataKind, FacetKind};
    ///
    /// assert_eq!(TypeResolution::primitive("Строка").type_name(), "Строка");
    /// assert_eq!(TypeResolution::primitive("Number").type_name(), "Число");
    ///
    /// let catalog = TypeResolution::metadata_type(MetadataKind::Catalog, "Контрагенты", None);
    /// assert_eq!(catalog.type_name(), "Справочники.Контрагенты");
    /// ```
    pub fn type_name(&self) -> String {
        match &self.result {
            ResolutionResult::Concrete(concrete) => match concrete {
                ConcreteType::Primitive(pt) => pt.display_name().to_string(),
                ConcreteType::Platform(platform) => platform.name.clone(),
                ConcreteType::Configuration(cfg) => {
                    format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
                }
                ConcreteType::Special(special) => special.display_name().to_string(),
                ConcreteType::GlobalFunction(func) => func.name.clone(),
                ConcreteType::TabularRow(tr) => tr.get_full_name(),
            },
            ResolutionResult::Generic(gen) => {
                if gen.type_params.is_empty() {
                    gen.base_type.clone()
                } else {
                    let params: Vec<String> = gen
                        .type_params
                        .iter()
                        .map(|ct| {
                            let resolution = TypeResolution::known(ct.clone());
                            resolution.type_name()
                        })
                        .collect();
                    format!("{}<{}>", gen.base_type, params.join(", "))
                }
            }
            ResolutionResult::Union(variants) => {
                if variants.is_empty() {
                    "Dynamic".to_string()
                } else {
                    let types: Vec<String> = variants
                        .iter()
                        .map(|wt| {
                            let resolution = TypeResolution::known(wt.type_.clone());
                            resolution.type_name()
                        })
                        .collect();
                    types.join(" | ")
                }
            }
            ResolutionResult::Intersection(types) => {
                if types.is_empty() {
                    "Dynamic".to_string()
                } else {
                    let names: Vec<String> = types
                        .iter()
                        .map(|ct| {
                            let resolution = TypeResolution::known(ct.clone());
                            resolution.type_name()
                        })
                        .collect();
                    names.join(" & ")
                }
            }
            ResolutionResult::Nullable(inner) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                format!("{}?", inner_resolution.type_name())
            }
            ResolutionResult::Dynamic => "Dynamic".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Certainty {
    Known,
    Inferred(f32),
    Unknown,
}

/// Reason why type resolution resulted in Unknown certainty
/// Used for precise error messages and validation decisions
///
/// # MILESTONE 3.16: Unknown Certainty Semantics
///
/// This enum captures WHY a type is unknown, enabling:
/// - Precise error messages (e.g., "Document 'Контрогенты' not found")
/// - Graceful degradation when configuration is not loaded
/// - Suggestions for typo fixes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UncertaintyReason {
    /// Configuration metadata is not loaded yet
    /// In this case, we don't report errors (graceful degradation)
    ConfigurationNotLoaded,

    /// Metadata object was not found in the loaded configuration
    /// This indicates a potential typo or missing object
    MetadataObjectNotFound {
        /// Kind of metadata (Document, Catalog, etc.)
        kind: MetadataKind,
        /// Name that was not found
        name: String,
    },

    /// Other reason for uncertainty
    Other(String),
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
    /// Фасет типа (Manager, Object, Reference, Selection, List)
    /// Определяет, какое представление типа активно
    pub facet: Option<FacetKind>,
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
    /// MILESTONE 3.16: Reason why type resolution resulted in Unknown/Inferred certainty
    /// Used for precise error messages and validation decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncertainty_reason: Option<UncertaintyReason>,
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
                // Если facet указан - используем фасетный префикс (СправочникМенеджер.Контрагенты)
                // Иначе используем стандартный display_name (Справочники.Контрагенты)
                if let Some(facet) = &config.facet {
                    write!(f, "{}.{}", config.kind.faceted_type_prefix(facet), config.name)
                } else {
                    write!(f, "{}.{}", config.kind.display_name(), config.name)
                }
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
            MetadataKind::Unknown => "Неизвестный",
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

    /// Возвращает префикс типа с учётом фасета
    ///
    /// # Примеры
    /// - Catalog + Manager -> "СправочникМенеджер"
    /// - Catalog + Object -> "СправочникОбъект"
    /// - Document + Reference -> "ДокументСсылка"
    pub fn faceted_type_prefix(&self, facet: &FacetKind) -> String {
        let base = match self {
            MetadataKind::Catalog => "Справочник",
            MetadataKind::Document => "Документ",
            MetadataKind::Enum => "Перечисление",
            MetadataKind::Report => "Отчет",
            MetadataKind::DataProcessor => "Обработка",
            MetadataKind::ChartOfAccounts => "ПланСчетов",
            MetadataKind::ChartOfCharacteristicTypes => "ПланВидовХарактеристик",
            MetadataKind::ChartOfCalculationTypes => "ПланВидовРасчета",
            MetadataKind::InformationRegister => "РегистрСведений",
            MetadataKind::AccumulationRegister => "РегистрНакопления",
            MetadataKind::AccountingRegister => "РегистрБухгалтерии",
            MetadataKind::CalculationRegister => "РегистрРасчета",
            MetadataKind::BusinessProcess => "БизнесПроцесс",
            MetadataKind::Task => "Задача",
            MetadataKind::ExchangePlan => "ПланОбмена",
            MetadataKind::Constant => "Константа",
            // Для типов без фасетов возвращаем display_name
            _ => return self.display_name().to_string(),
        };

        let suffix = facet.platform_suffix();
        format!("{}{}", base, suffix)
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
// Milestone 3.13: Object-Based Type Comparison
// ============================================================================

/// Ссылка на тип в TypeRepository (lazy lookup)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeRef {
    /// Имя типа для поиска в repository
    pub lookup_key: String,
    /// Кешированный hash для быстрого сравнения
    pub type_hash: u64,
}

impl TypeRef {
    pub fn new(lookup_key: &str) -> Self {
        Self {
            lookup_key: lookup_key.to_string(),
            type_hash: Self::hash_type_name(lookup_key),
        }
    }

    fn hash_type_name(name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.to_lowercase().hash(&mut hasher);
        hasher.finish()
    }
}

/// Результат сравнения совместимости типов
#[derive(Debug, Clone, PartialEq)]
pub enum TypeCompatibility {
    /// Полностью совместимы
    Compatible,
    /// Несовместимы с причиной
    Incompatible { reason: String },
    /// Частично совместимы (gradual typing)
    PartiallyCompatible { certainty: f32, reason: String },
}

impl TypeCompatibility {
    pub fn is_compatible(&self) -> bool {
        matches!(
            self,
            TypeCompatibility::Compatible | TypeCompatibility::PartiallyCompatible { .. }
        )
    }

    pub fn reason(&self) -> String {
        match self {
            TypeCompatibility::Compatible => String::new(),
            TypeCompatibility::Incompatible { reason } => reason.clone(),
            TypeCompatibility::PartiallyCompatible { reason, .. } => reason.clone(),
        }
    }
}

impl TypeResolution {
    /// Объектное сравнение типов с учётом семантики
    pub fn is_compatible_with(&self, other: &TypeResolution) -> TypeCompatibility {
        // Dynamic/Unknown совместимы со всем (gradual typing)
        if matches!(self.result, ResolutionResult::Dynamic)
            || matches!(other.result, ResolutionResult::Dynamic)
        {
            return TypeCompatibility::Compatible;
        }

        match (&self.result, &other.result) {
            // Concrete типы
            (ResolutionResult::Concrete(a), ResolutionResult::Concrete(b)) => {
                Self::check_concrete_compatibility(a, b, self.active_facet, other.active_facet)
            }

            // Union — actual должен быть совместим с хотя бы одним членом
            (_, ResolutionResult::Union(variants)) => {
                for variant in variants {
                    let variant_resolution = TypeResolution::known(variant.type_.clone());
                    if self.is_compatible_with(&variant_resolution).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Не совместим ни с одним вариантом Union".to_string(),
                }
            }

            // Generic — проверяем base type и параметры
            (ResolutionResult::Generic(g1), ResolutionResult::Generic(g2)) => {
                Self::check_generic_compatibility(g1, g2)
            }

            // Nullable
            (ResolutionResult::Nullable(inner), other_result) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                inner_resolution.is_compatible_with(&TypeResolution {
                    result: other_result.clone(),
                    ..TypeResolution::unknown()
                })
            }

            // Intersection — actual должен быть совместим со ВСЕМИ типами
            (_, ResolutionResult::Intersection(types)) => {
                for concrete in types {
                    let type_resolution = TypeResolution::known(concrete.clone());
                    if !self.is_compatible_with(&type_resolution).is_compatible() {
                        return TypeCompatibility::Incompatible {
                            reason: "Не совместим со всеми типами Intersection".to_string(),
                        };
                    }
                }
                TypeCompatibility::Compatible
            }

            // Intersection как actual - должен содержать хотя бы один совместимый тип
            (ResolutionResult::Intersection(types), _) => {
                for concrete in types {
                    let type_resolution = TypeResolution::known(concrete.clone());
                    if type_resolution.is_compatible_with(other).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Ни один тип из Intersection не совместим".to_string(),
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: format!("Типы {:?} и {:?} несовместимы", self.result, other.result),
            },
        }
    }

    /// Проверка совместимости конкретных типов
    fn check_concrete_compatibility(
        from: &ConcreteType,
        to: &ConcreteType,
        from_facet: Option<FacetKind>,
        to_facet: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            // Примитивы — точное совпадение
            (ConcreteType::Primitive(a), ConcreteType::Primitive(b)) => {
                if a == b {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Примитивы {:?} и {:?} несовместимы", a, b),
                    }
                }
            }

            // Configuration типы — учитываем фасеты!
            (ConcreteType::Configuration(cfg1), ConcreteType::Configuration(cfg2)) => {
                if cfg1.kind != cfg2.kind || !Self::names_equal(&cfg1.name, &cfg2.name) {
                    return TypeCompatibility::Incompatible {
                        reason: format!(
                            "Разные типы конфигурации: {}.{} vs {}.{}",
                            cfg1.kind.to_prefix(),
                            cfg1.name,
                            cfg2.kind.to_prefix(),
                            cfg2.name
                        ),
                    };
                }
                // Проверяем фасетную совместимость
                Self::check_facet_compatibility(from_facet.or(cfg1.facet), to_facet.or(cfg2.facet))
            }

            // Platform типы — case-insensitive сравнение имён
            (ConcreteType::Platform(pt1), ConcreteType::Platform(pt2)) => {
                if Self::names_equal(&pt1.name, &pt2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!(
                            "Платформенные типы {} и {} несовместимы",
                            pt1.name, pt2.name
                        ),
                    }
                }
            }

            // Special типы
            (ConcreteType::Special(s1), ConcreteType::Special(s2)) => {
                if s1 == s2 {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Специальные типы {:?} и {:?} несовместимы", s1, s2),
                    }
                }
            }

            // TabularRow - сравнение по parent_type и tabular_section_name
            (ConcreteType::TabularRow(tr1), ConcreteType::TabularRow(tr2)) => {
                if Self::names_equal(&tr1.parent_type, &tr2.parent_type)
                    && Self::names_equal(&tr1.tabular_section_name, &tr2.tabular_section_name)
                {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!(
                            "Разные строки табличных частей: {}.{} vs {}.{}",
                            tr1.parent_type, tr1.tabular_section_name,
                            tr2.parent_type, tr2.tabular_section_name
                        ),
                    }
                }
            }

            // GlobalFunction - функции как типы несовместимы между собой (разные сигнатуры)
            (ConcreteType::GlobalFunction(f1), ConcreteType::GlobalFunction(f2)) => {
                if Self::names_equal(&f1.name, &f2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Разные глобальные функции: {} vs {}", f1.name, f2.name),
                    }
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: "Несовместимые категории типов".to_string(),
            },
        }
    }

    /// Проверка фасетной совместимости
    fn check_facet_compatibility(
        from: Option<FacetKind>,
        to: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            (None, _) | (_, None) => TypeCompatibility::Compatible,
            (Some(f1), Some(f2)) if f1 == f2 => TypeCompatibility::Compatible,
            // Object → Reference: допустимо (неявная конвертация)
            (Some(FacetKind::Object), Some(FacetKind::Reference)) => TypeCompatibility::Compatible,
            // Reference → Object: НЕ допустимо (нужен ПолучитьОбъект())
            (Some(FacetKind::Reference), Some(FacetKind::Object)) => {
                TypeCompatibility::Incompatible {
                    reason: "Ссылка не может быть неявно преобразована в Объект (используйте ПолучитьОбъект())".to_string(),
                }
            }
            // Manager → любой другой: НЕ допустимо
            (Some(FacetKind::Manager), Some(other)) => TypeCompatibility::Incompatible {
                reason: format!("Менеджер несовместим с фасетом {:?}", other),
            },
            (Some(f1), Some(f2)) => TypeCompatibility::Incompatible {
                reason: format!("Фасет {:?} несовместим с {:?}", f1, f2),
            },
        }
    }

    /// Проверка совместимости Generic типов
    fn check_generic_compatibility(g1: &GenericType, g2: &GenericType) -> TypeCompatibility {
        // Проверяем базовый тип
        if !Self::names_equal(&g1.base_type, &g2.base_type) {
            return TypeCompatibility::Incompatible {
                reason: format!(
                    "Несовместимые базовые типы: {} vs {}",
                    g1.base_type, g2.base_type
                ),
            };
        }

        // Проверяем параметры
        if g1.type_params.len() != g2.type_params.len() {
            return TypeCompatibility::Incompatible {
                reason: "Разное количество параметров Generic".to_string(),
            };
        }

        for (p1, p2) in g1.type_params.iter().zip(g2.type_params.iter()) {
            let r1 = TypeResolution::known(p1.clone());
            let r2 = TypeResolution::known(p2.clone());
            if !r1.is_compatible_with(&r2).is_compatible() {
                return TypeCompatibility::Incompatible {
                    reason: format!("Параметры Generic {:?} и {:?} несовместимы", p1, p2),
                };
            }
        }

        TypeCompatibility::Compatible
    }

    fn names_equal(a: &str, b: &str) -> bool {
        a.to_lowercase() == b.to_lowercase()
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

// ============================================================================
// Tests for Milestone 3.13: Object-Based Type Comparison
// ============================================================================

#[cfg(test)]
mod type_compatibility_tests {
    use super::*;

    #[test]
    fn test_primitive_compatibility() {
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
        let number = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number));

        assert!(string.is_compatible_with(&string).is_compatible());
        assert!(!string.is_compatible_with(&number).is_compatible());
    }

    #[test]
    fn test_facet_compatibility_object_to_reference() {
        let obj = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Object),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };
        let ref_ = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Reference),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Reference),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        // Object -> Reference: OK
        assert!(obj.is_compatible_with(&ref_).is_compatible());
        // Reference -> Object: NOT OK
        assert!(!ref_.is_compatible_with(&obj).is_compatible());
    }

    #[test]
    fn test_dynamic_compatible_with_everything() {
        let dynamic = TypeResolution::unknown();
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));

        assert!(dynamic.is_compatible_with(&string).is_compatible());
        assert!(string.is_compatible_with(&dynamic).is_compatible());
    }

    #[test]
    fn test_union_compatibility() {
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
        let union = TypeResolution {
            result: ResolutionResult::Union(vec![
                WeightedType {
                    type_: ConcreteType::Primitive(PrimitiveType::String),
                    weight: 0.5,
                },
                WeightedType {
                    type_: ConcreteType::Primitive(PrimitiveType::Number),
                    weight: 0.5,
                },
            ]),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(string.is_compatible_with(&union).is_compatible());
    }

    #[test]
    fn test_platform_type_case_insensitive() {
        let t1 = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        }));
        let t2 = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "массив".to_string(),
        }));

        assert!(t1.is_compatible_with(&t2).is_compatible());
    }

    #[test]
    fn test_manager_incompatible_with_object() {
        let manager = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Manager),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Manager),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };
        let object = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Object),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        // Manager -> Object: NOT OK
        assert!(!manager.is_compatible_with(&object).is_compatible());
    }

    #[test]
    fn test_type_ref_hash() {
        let ref1 = TypeRef::new("Массив");
        let ref2 = TypeRef::new("массив");
        let ref3 = TypeRef::new("МАССИВ");

        // Same hash for case-insensitive names
        assert_eq!(ref1.type_hash, ref2.type_hash);
        assert_eq!(ref2.type_hash, ref3.type_hash);
    }

    #[test]
    fn test_type_compatibility_reason() {
        let compatible = TypeCompatibility::Compatible;
        let incompatible = TypeCompatibility::Incompatible {
            reason: "Test reason".to_string(),
        };
        let partial = TypeCompatibility::PartiallyCompatible {
            certainty: 0.7,
            reason: "Partial reason".to_string(),
        };

        assert_eq!(compatible.reason(), "");
        assert_eq!(incompatible.reason(), "Test reason");
        assert_eq!(partial.reason(), "Partial reason");
    }

    #[test]
    fn test_generic_type_compatibility() {
        let array_string = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Массив".to_string(),
                type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let array_string2 = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Массив".to_string(),
                type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let array_number = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Массив".to_string(),
                type_params: vec![ConcreteType::Primitive(PrimitiveType::Number)],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Same generic types are compatible
        assert!(array_string.is_compatible_with(&array_string2).is_compatible());
        // Different generic params are incompatible
        assert!(!array_string.is_compatible_with(&array_number).is_compatible());
    }

    #[test]
    fn test_different_configuration_types_incompatible() {
        let catalog = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Object),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };
        let document = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Document,
                name: "ЗаказПокупателя".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Object),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        // Different configuration types are incompatible
        assert!(!catalog.is_compatible_with(&document).is_compatible());
    }

    // ============================================================================
    // Milestone 3.13 Additional Tests: Intersection, TabularRow, GlobalFunction
    // ============================================================================

    #[test]
    fn test_intersection_compatibility_all_must_match() {
        // Intersection требует совместимости со ВСЕМИ типами
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
        let intersection = TypeResolution {
            result: ResolutionResult::Intersection(vec![
                ConcreteType::Primitive(PrimitiveType::String),
                ConcreteType::Primitive(PrimitiveType::Number),
            ]),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // String не совместим с Intersection(String, Number) потому что не совместим с Number
        assert!(!string.is_compatible_with(&intersection).is_compatible());
    }

    #[test]
    fn test_tabular_row_same_parent_compatible() {
        let tr1 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.ЗаказНаряды".to_string(),
            tabular_section_name: "Работы".to_string(),
            attributes: vec![],
        }));
        let tr2 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.ЗаказНаряды".to_string(),
            tabular_section_name: "Работы".to_string(),
            attributes: vec![],
        }));

        assert!(tr1.is_compatible_with(&tr2).is_compatible());
    }

    #[test]
    fn test_tabular_row_different_section_incompatible() {
        let tr1 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.ЗаказНаряды".to_string(),
            tabular_section_name: "Работы".to_string(),
            attributes: vec![],
        }));
        let tr2 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.ЗаказНаряды".to_string(),
            tabular_section_name: "Материалы".to_string(),
            attributes: vec![],
        }));

        assert!(!tr1.is_compatible_with(&tr2).is_compatible());
    }

    #[test]
    fn test_global_function_same_name_compatible() {
        let f1 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
            name: "СтрДлина".to_string(),
            english_name: Some("StrLen".to_string()),
            description: None,
            parameters: vec![],
            return_type: Some("Число".to_string()),
            return_description: None,
            polymorphic: false,
            pure: true,
            contexts: vec![],
            category: None,
        }));
        let f2 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
            name: "СтрДлина".to_string(),
            english_name: Some("StrLen".to_string()),
            description: None,
            parameters: vec![],
            return_type: Some("Число".to_string()),
            return_description: None,
            polymorphic: false,
            pure: true,
            contexts: vec![],
            category: None,
        }));

        assert!(f1.is_compatible_with(&f2).is_compatible());
    }

    #[test]
    fn test_global_function_different_name_incompatible() {
        let f1 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
            name: "СтрДлина".to_string(),
            english_name: None,
            description: None,
            parameters: vec![],
            return_type: None,
            return_description: None,
            polymorphic: false,
            pure: false,
            contexts: vec![],
            category: None,
        }));
        let f2 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
            name: "СтрНайти".to_string(),
            english_name: None,
            description: None,
            parameters: vec![],
            return_type: None,
            return_description: None,
            polymorphic: false,
            pure: false,
            contexts: vec![],
            category: None,
        }));

        assert!(!f1.is_compatible_with(&f2).is_compatible());
    }

    #[test]
    fn test_nested_generic_compatibility() {
        // Массив<Массив<Строка>> должен быть совместим сам с собой
        let outer = GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![ConcreteType::Platform(PlatformType {
                name: "Массив".to_string(),
            })],
        };

        let t1 = TypeResolution {
            result: ResolutionResult::Generic(outer.clone()),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let t2 = TypeResolution {
            result: ResolutionResult::Generic(outer),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(t1.is_compatible_with(&t2).is_compatible());
    }

    #[test]
    fn test_nullable_with_concrete() {
        // Nullable<String> должен быть совместим с String
        let nullable = TypeResolution {
            result: ResolutionResult::Nullable(Box::new(ConcreteType::Primitive(
                PrimitiveType::String,
            ))),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));

        assert!(nullable.is_compatible_with(&string).is_compatible());
    }

    #[test]
    fn test_configuration_same_kind_different_name_incompatible() {
        let cfg1 = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: None,
            attributes: vec![],
            tabular_sections: vec![],
        }));
        let cfg2 = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Номенклатура".to_string(),
            facet: None,
            attributes: vec![],
            tabular_sections: vec![],
        }));

        // Разные справочники несовместимы
        assert!(!cfg1.is_compatible_with(&cfg2).is_compatible());
    }

    #[test]
    fn test_selection_facet_compatible_with_selection() {
        let sel1 = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Selection),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Selection),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };
        let sel2 = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Selection),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Selection),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        assert!(sel1.is_compatible_with(&sel2).is_compatible());
    }

    #[test]
    fn test_list_facet_incompatible_with_object() {
        let list = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::List),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::List),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };
        let obj = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: Some(FacetKind::Object),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            active_facet: Some(FacetKind::Object),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            available_facets: vec![],
        };

        // List несовместим с Object
        assert!(!list.is_compatible_with(&obj).is_compatible());
    }

    #[test]
    fn test_partial_compatibility_is_compatible() {
        let partial = TypeCompatibility::PartiallyCompatible {
            certainty: 0.8,
            reason: "Gradual typing".to_string(),
        };

        assert!(partial.is_compatible());
        assert_eq!(partial.reason(), "Gradual typing");
    }
}

// ============================================================================
// Tests for Milestone 3.14: Go To Definition Location
// ============================================================================

#[cfg(test)]
mod type_definition_location_tests {
    use super::*;
    use crate::domain::type_definition_location::TypeDefinitionLocation;

    #[test]
    fn test_primitive_definition_location() {
        let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
        let loc = string.get_definition_location();

        assert!(loc.is_some());
        assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Primitive));
    }

    #[test]
    fn test_platform_definition_location() {
        let array = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        }));
        let loc = array.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Platform { type_name, docs_uri }) = loc {
            assert_eq!(type_name, "Массив");
            assert!(docs_uri.is_some());
        } else {
            panic!("Expected Platform location");
        }
    }

    #[test]
    fn test_configuration_definition_location() {
        let catalog = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: None,
            attributes: vec![],
            tabular_sections: vec![],
        }));
        let loc = catalog.get_definition_location();

        assert!(loc.is_some());
        assert!(matches!(
            loc.unwrap(),
            TypeDefinitionLocation::Configuration { .. }
        ));
    }

    #[test]
    fn test_generic_definition_location() {
        let generic = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Массив".to_string(),
                type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let loc = generic.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
            assert_eq!(type_name, "Массив");
        } else {
            panic!("Expected Platform location for Generic base type");
        }
    }

    #[test]
    fn test_dynamic_no_definition_location() {
        let dynamic = TypeResolution::unknown();
        let loc = dynamic.get_definition_location();

        assert!(loc.is_none());
    }

    #[test]
    fn test_union_definition_location_first_type() {
        let union = TypeResolution {
            result: ResolutionResult::Union(vec![
                WeightedType {
                    type_: ConcreteType::Primitive(PrimitiveType::String),
                    weight: 0.7,
                },
                WeightedType {
                    type_: ConcreteType::Primitive(PrimitiveType::Number),
                    weight: 0.3,
                },
            ]),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let loc = union.get_definition_location();

        // Должен вернуть location первого (String) типа
        assert!(loc.is_some());
        assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Primitive));
    }

    #[test]
    fn test_nullable_definition_location_inner() {
        let nullable = TypeResolution {
            result: ResolutionResult::Nullable(Box::new(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            }))),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let loc = nullable.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
            assert_eq!(type_name, "ТаблицаЗначений");
        } else {
            panic!("Expected Platform location for Nullable inner type");
        }
    }

    #[test]
    fn test_special_type_definition_location() {
        let null = TypeResolution::known(ConcreteType::Special(SpecialType::Null));
        let loc = null.get_definition_location();

        assert!(loc.is_some());
        // Special типы возвращают Primitive (нет навигации)
        assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Primitive));
    }

    #[test]
    fn test_global_function_definition_location() {
        let func = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
            name: "СтрДлина".to_string(),
            english_name: Some("StrLen".to_string()),
            description: None,
            parameters: vec![],
            return_type: Some("Число".to_string()),
            return_description: None,
            polymorphic: false,
            pure: true,
            contexts: vec![],
            category: None,
        }));
        let loc = func.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
            assert_eq!(type_name, "СтрДлина");
        } else {
            panic!("Expected Platform location for GlobalFunction");
        }
    }

    #[test]
    fn test_tabular_row_definition_location() {
        let tr = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.ЗаказНаряды".to_string(),
            tabular_section_name: "Работы".to_string(),
            attributes: vec![],
        }));
        let loc = tr.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Configuration { metadata_path, .. }) = loc {
            assert!(metadata_path
                .to_string_lossy()
                .contains("ЗаказНаряды"));
        } else {
            panic!("Expected Configuration location for TabularRow");
        }
    }

    #[test]
    fn test_intersection_definition_location_first_type() {
        let intersection = TypeResolution {
            result: ResolutionResult::Intersection(vec![
                ConcreteType::Platform(PlatformType {
                    name: "Массив".to_string(),
                }),
                ConcreteType::Platform(PlatformType {
                    name: "ФиксированныйМассив".to_string(),
                }),
            ]),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let loc = intersection.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
            // Должен вернуть первый тип (Массив)
            assert_eq!(type_name, "Массив");
        } else {
            panic!("Expected Platform location for Intersection first type");
        }
    }

    #[test]
    fn test_empty_union_no_definition_location() {
        let empty_union = TypeResolution {
            result: ResolutionResult::Union(vec![]),
            certainty: Certainty::Unknown,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let loc = empty_union.get_definition_location();

        assert!(loc.is_none());
    }

    #[test]
    fn test_empty_intersection_no_definition_location() {
        let empty_intersection = TypeResolution {
            result: ResolutionResult::Intersection(vec![]),
            certainty: Certainty::Unknown,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        let loc = empty_intersection.get_definition_location();

        assert!(loc.is_none());
    }

    #[test]
    fn test_configuration_location_has_metadata_key() {
        let doc = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Document,
            name: "ЗаказПокупателя".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        }));
        let loc = doc.get_definition_location();

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Configuration { metadata_path, .. }) = loc {
            let path_str = metadata_path.to_string_lossy();
            // Проверяем, что путь содержит правильный префикс и имя
            assert!(path_str.contains("Документы"));
            assert!(path_str.contains("ЗаказПокупателя"));
        } else {
            panic!("Expected Configuration location");
        }
    }

    // ============================================================================
    // Milestone 3.14: Go To Definition with Module Paths
    // ============================================================================

    #[test]
    fn test_configuration_with_module_paths() {
        use crate::domain::type_definition_location::ModulePaths;
        use std::path::PathBuf;

        let catalog = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        }));

        let module_paths = ModulePaths {
            object_module: Some(PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl")),
            manager_module: Some(PathBuf::from("Catalogs/Контрагенты/Ext/ManagerModule.bsl")),
            recordset_module: None,
        };

        let loc = catalog.get_definition_location_with_modules(Some(&module_paths));

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Configuration {
            metadata_path,
            module_paths: mp,
        }) = loc
        {
            // metadata_path должен содержать тип
            assert!(metadata_path.to_string_lossy().contains("Справочники"));
            assert!(metadata_path.to_string_lossy().contains("Контрагенты"));

            // module_paths должны быть скопированы
            assert!(mp.object_module.is_some());
            assert!(mp.manager_module.is_some());
            assert!(mp.recordset_module.is_none());
            assert!(mp
                .object_module
                .as_ref()
                .unwrap()
                .to_string_lossy()
                .contains("ObjectModule.bsl"));
        } else {
            panic!("Expected Configuration location with module paths");
        }
    }

    #[test]
    fn test_configuration_without_module_paths_fallback() {
        let catalog = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Товары".to_string(),
            facet: None,
            attributes: vec![],
            tabular_sections: vec![],
        }));

        // Без module_paths - должен работать как старый метод
        let loc = catalog.get_definition_location_with_modules(None);

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Configuration { module_paths, .. }) = loc {
            // module_paths должны быть пустыми (default)
            assert!(!module_paths.has_any_module());
        } else {
            panic!("Expected Configuration location");
        }
    }

    #[test]
    fn test_platform_type_ignores_module_paths() {
        use crate::domain::type_definition_location::ModulePaths;
        use std::path::PathBuf;

        let array = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        }));

        let module_paths = ModulePaths {
            object_module: Some(PathBuf::from("some/path.bsl")),
            manager_module: None,
            recordset_module: None,
        };

        // Platform тип должен игнорировать module_paths
        let loc = array.get_definition_location_with_modules(Some(&module_paths));

        assert!(loc.is_some());
        // Должен вернуть Platform location, не Configuration
        assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Platform { .. }));
    }

    #[test]
    fn test_tabular_row_with_module_paths() {
        use crate::domain::type_definition_location::ModulePaths;
        use std::path::PathBuf;

        let tr = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.ЗаказПокупателя".to_string(),
            tabular_section_name: "Товары".to_string(),
            attributes: vec![],
        }));

        let module_paths = ModulePaths {
            object_module: Some(PathBuf::from("Documents/ЗаказПокупателя/Ext/ObjectModule.bsl")),
            manager_module: None,
            recordset_module: None,
        };

        let loc = tr.get_definition_location_with_modules(Some(&module_paths));

        assert!(loc.is_some());
        if let Some(TypeDefinitionLocation::Configuration {
            metadata_path,
            module_paths: mp,
        }) = loc
        {
            // metadata_path должен содержать родительский тип и имя табличной части
            assert!(metadata_path.to_string_lossy().contains("ЗаказПокупателя"));
            assert!(metadata_path.to_string_lossy().contains("Товары"));

            // module_paths должны быть скопированы
            assert!(mp.object_module.is_some());
        } else {
            panic!("Expected Configuration location for TabularRow");
        }
    }

    #[test]
    fn test_dynamic_with_module_paths() {
        use crate::domain::type_definition_location::ModulePaths;
        use std::path::PathBuf;

        let dynamic = TypeResolution::unknown();

        let module_paths = ModulePaths {
            object_module: Some(PathBuf::from("some/path.bsl")),
            manager_module: None,
            recordset_module: None,
        };

        // Dynamic тип не должен иметь location даже с module_paths
        let loc = dynamic.get_definition_location_with_modules(Some(&module_paths));
        assert!(loc.is_none());
    }
}

// ============================================================================
// Tests for Milestone 3.18: TypeResolution Constructors and Utilities
// ============================================================================

#[cfg(test)]
mod type_resolution_constructors_tests {
    use super::*;

    // === primitive() tests ===

    #[test]
    fn test_primitive_string_russian() {
        let t = TypeResolution::primitive("Строка");
        assert_eq!(t.certainty, Certainty::Known);
        assert_eq!(t.type_name(), "Строка");
        assert!(matches!(
            t.result,
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
        ));
    }

    #[test]
    fn test_primitive_string_english() {
        let t = TypeResolution::primitive("String");
        assert_eq!(t.certainty, Certainty::Known);
        assert_eq!(t.type_name(), "Строка");
    }

    #[test]
    fn test_primitive_number_russian() {
        let t = TypeResolution::primitive("Число");
        assert_eq!(t.type_name(), "Число");
        assert!(matches!(
            t.result,
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
        ));
    }

    #[test]
    fn test_primitive_number_english() {
        let t = TypeResolution::primitive("Number");
        assert_eq!(t.type_name(), "Число");
    }

    #[test]
    fn test_primitive_boolean_russian() {
        let t = TypeResolution::primitive("Булево");
        assert_eq!(t.type_name(), "Булево");
        assert!(matches!(
            t.result,
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean))
        ));
    }

    #[test]
    fn test_primitive_boolean_english() {
        let t = TypeResolution::primitive("Boolean");
        assert_eq!(t.type_name(), "Булево");
    }

    #[test]
    fn test_primitive_date_russian() {
        let t = TypeResolution::primitive("Дата");
        assert_eq!(t.type_name(), "Дата");
        assert!(matches!(
            t.result,
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Date))
        ));
    }

    #[test]
    fn test_primitive_date_english() {
        let t = TypeResolution::primitive("Date");
        assert_eq!(t.type_name(), "Дата");
    }

    #[test]
    fn test_primitive_case_insensitive() {
        // Должен распознавать независимо от регистра
        let t1 = TypeResolution::primitive("СТРОКА");
        let t2 = TypeResolution::primitive("строка");
        let t3 = TypeResolution::primitive("Строка");

        assert_eq!(t1.type_name(), "Строка");
        assert_eq!(t2.type_name(), "Строка");
        assert_eq!(t3.type_name(), "Строка");
    }

    #[test]
    fn test_primitive_unknown_creates_platform() {
        // Нераспознанное имя создаёт Platform тип
        let t = TypeResolution::primitive("Массив");
        assert_eq!(t.certainty, Certainty::Known);
        assert_eq!(t.type_name(), "Массив");
        assert!(matches!(
            t.result,
            ResolutionResult::Concrete(ConcreteType::Platform(_))
        ));
    }

    #[test]
    fn test_primitive_source_is_static() {
        let t = TypeResolution::primitive("Строка");
        assert_eq!(t.source, ResolutionSource::Static);
    }

    // === inferred() tests ===

    #[test]
    fn test_inferred_with_confidence() {
        let t = TypeResolution::inferred("Число", 0.75);
        assert_eq!(t.certainty, Certainty::Inferred(0.75));
        assert_eq!(t.type_name(), "Число");
    }

    #[test]
    fn test_inferred_source_is_inferred() {
        let t = TypeResolution::inferred("Строка", 0.5);
        assert_eq!(t.source, ResolutionSource::Inferred);
    }

    #[test]
    fn test_inferred_with_zero_confidence() {
        let t = TypeResolution::inferred("Булево", 0.0);
        assert_eq!(t.certainty, Certainty::Inferred(0.0));
    }

    #[test]
    fn test_inferred_with_full_confidence() {
        let t = TypeResolution::inferred("Дата", 1.0);
        assert_eq!(t.certainty, Certainty::Inferred(1.0));
    }

    #[test]
    fn test_inferred_unknown_type() {
        // Нераспознанный тип также создаёт Platform
        let t = TypeResolution::inferred("ТаблицаЗначений", 0.8);
        assert_eq!(t.certainty, Certainty::Inferred(0.8));
        assert_eq!(t.type_name(), "ТаблицаЗначений");
    }

    // === metadata_type() tests ===

    #[test]
    fn test_metadata_type_catalog_with_manager() {
        let t = TypeResolution::metadata_type(
            MetadataKind::Catalog,
            "Контрагенты",
            Some(FacetKind::Manager),
        );
        assert_eq!(t.certainty, Certainty::Known);
        assert_eq!(t.active_facet, Some(FacetKind::Manager));
        assert_eq!(t.type_name(), "Справочники.Контрагенты");
    }

    #[test]
    fn test_metadata_type_document_with_object() {
        let t = TypeResolution::metadata_type(
            MetadataKind::Document,
            "ЗаказПокупателя",
            Some(FacetKind::Object),
        );
        assert_eq!(t.active_facet, Some(FacetKind::Object));
        assert_eq!(t.type_name(), "Документы.ЗаказПокупателя");
    }

    #[test]
    fn test_metadata_type_without_facet() {
        let t = TypeResolution::metadata_type(MetadataKind::Catalog, "Номенклатура", None);
        assert_eq!(t.active_facet, None);
        assert!(t.available_facets.is_empty());
        assert_eq!(t.type_name(), "Справочники.Номенклатура");
    }

    #[test]
    fn test_metadata_type_with_reference() {
        let t = TypeResolution::metadata_type(
            MetadataKind::Document,
            "СчетНаОплату",
            Some(FacetKind::Reference),
        );
        assert_eq!(t.active_facet, Some(FacetKind::Reference));
    }

    #[test]
    fn test_metadata_type_available_facets() {
        let t = TypeResolution::metadata_type(
            MetadataKind::Catalog,
            "Валюты",
            Some(FacetKind::Selection),
        );
        assert!(t.available_facets.contains(&FacetKind::Selection));
    }

    #[test]
    fn test_metadata_type_enum() {
        let t = TypeResolution::metadata_type(MetadataKind::Enum, "ВидыОперации", None);
        assert_eq!(t.type_name(), "Перечисления.ВидыОперации");
    }

    #[test]
    fn test_metadata_type_register() {
        let t = TypeResolution::metadata_type(
            MetadataKind::InformationRegister,
            "КурсыВалют",
            None,
        );
        assert_eq!(t.type_name(), "РегистрыСведений.КурсыВалют");
    }

    // === is_unknown() tests ===

    #[test]
    fn test_is_unknown_true() {
        let t = TypeResolution::unknown();
        assert!(t.is_unknown());
    }

    #[test]
    fn test_is_unknown_false_for_primitive() {
        let t = TypeResolution::primitive("Строка");
        assert!(!t.is_unknown());
    }

    #[test]
    fn test_is_unknown_false_for_known() {
        let t = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number));
        assert!(!t.is_unknown());
    }

    #[test]
    fn test_is_unknown_false_for_inferred() {
        let t = TypeResolution::inferred("Число", 0.5);
        assert!(!t.is_unknown());
    }

    #[test]
    fn test_is_unknown_false_for_metadata() {
        let t = TypeResolution::metadata_type(MetadataKind::Catalog, "Товары", None);
        assert!(!t.is_unknown());
    }

    // === type_name() tests ===

    #[test]
    fn test_type_name_primitive() {
        assert_eq!(TypeResolution::primitive("Строка").type_name(), "Строка");
        assert_eq!(TypeResolution::primitive("Число").type_name(), "Число");
        assert_eq!(TypeResolution::primitive("Булево").type_name(), "Булево");
        assert_eq!(TypeResolution::primitive("Дата").type_name(), "Дата");
    }

    #[test]
    fn test_type_name_platform() {
        let t = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: "ТаблицаЗначений".to_string(),
        }));
        assert_eq!(t.type_name(), "ТаблицаЗначений");
    }

    #[test]
    fn test_type_name_configuration() {
        let t = TypeResolution::metadata_type(MetadataKind::Catalog, "Контрагенты", None);
        assert_eq!(t.type_name(), "Справочники.Контрагенты");
    }

    #[test]
    fn test_type_name_special_undefined() {
        let t = TypeResolution::known(ConcreteType::Special(SpecialType::Undefined));
        assert_eq!(t.type_name(), "Неопределено");
    }

    #[test]
    fn test_type_name_special_null() {
        let t = TypeResolution::known(ConcreteType::Special(SpecialType::Null));
        assert_eq!(t.type_name(), "Null");
    }

    #[test]
    fn test_type_name_special_type() {
        let t = TypeResolution::known(ConcreteType::Special(SpecialType::Type));
        assert_eq!(t.type_name(), "Тип");
    }

    #[test]
    fn test_type_name_global_function() {
        let t = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
            name: "СтрДлина".to_string(),
            english_name: None,
            description: None,
            parameters: vec![],
            return_type: None,
            return_description: None,
            polymorphic: false,
            pure: false,
            contexts: vec![],
            category: None,
        }));
        assert_eq!(t.type_name(), "СтрДлина");
    }

    #[test]
    fn test_type_name_tabular_row() {
        let t = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
            parent_type: "Документы.Заказ".to_string(),
            tabular_section_name: "Товары".to_string(),
            attributes: vec![],
        }));
        assert_eq!(t.type_name(), "СтрокаТовары");
    }

    #[test]
    fn test_type_name_generic() {
        let t = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Массив".to_string(),
                type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Массив<Строка>");
    }

    #[test]
    fn test_type_name_generic_multiple_params() {
        let t = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Соответствие".to_string(),
                type_params: vec![
                    ConcreteType::Primitive(PrimitiveType::String),
                    ConcreteType::Primitive(PrimitiveType::Number),
                ],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Соответствие<Строка, Число>");
    }

    #[test]
    fn test_type_name_generic_empty_params() {
        let t = TypeResolution {
            result: ResolutionResult::Generic(GenericType {
                base_type: "Массив".to_string(),
                type_params: vec![],
            }),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Массив");
    }

    #[test]
    fn test_type_name_union() {
        let t = TypeResolution {
            result: ResolutionResult::Union(vec![
                WeightedType::new(ConcreteType::Primitive(PrimitiveType::String)),
                WeightedType::new(ConcreteType::Primitive(PrimitiveType::Number)),
            ]),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Строка | Число");
    }

    #[test]
    fn test_type_name_union_empty() {
        let t = TypeResolution {
            result: ResolutionResult::Union(vec![]),
            certainty: Certainty::Unknown,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Dynamic");
    }

    #[test]
    fn test_type_name_intersection() {
        let t = TypeResolution {
            result: ResolutionResult::Intersection(vec![
                ConcreteType::Platform(PlatformType {
                    name: "Массив".to_string(),
                }),
                ConcreteType::Platform(PlatformType {
                    name: "Коллекция".to_string(),
                }),
            ]),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Массив & Коллекция");
    }

    #[test]
    fn test_type_name_intersection_empty() {
        let t = TypeResolution {
            result: ResolutionResult::Intersection(vec![]),
            certainty: Certainty::Unknown,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Dynamic");
    }

    #[test]
    fn test_type_name_nullable() {
        let t = TypeResolution {
            result: ResolutionResult::Nullable(Box::new(ConcreteType::Primitive(
                PrimitiveType::String,
            ))),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };
        assert_eq!(t.type_name(), "Строка?");
    }

    #[test]
    fn test_type_name_dynamic() {
        let t = TypeResolution::unknown();
        assert_eq!(t.type_name(), "Dynamic");
    }
}
