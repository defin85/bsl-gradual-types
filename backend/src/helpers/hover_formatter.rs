//! Унифицированное форматирование LSP hover responses
//!
//! Этот модуль предоставляет чистый API для форматирования информации о типах
//! в различных форматах (Markdown, PlainText) с конфигурируемыми лимитами.
//!
//! # Примеры
//!
//! ```rust,no_run
//! use bsl_backend::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig};
//! use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
//! use bsl_shared::domain::repository::InMemoryTypeRepository;
//! use bsl_shared::domain::types::TypeResolution;
//! use std::sync::Arc;
//!
//! // Создание TypeMetadataLookup
//! let repo = Arc::new(InMemoryTypeRepository::new());
//! let metadata_lookup = TypeMetadataLookup::new(repo);
//!
//! // Создание HoverFormatter с конфигурацией
//! let config = HoverFormatConfig {
//!     max_methods: 10,
//!     max_properties: 5,
//!     ..Default::default()
//! };
//! let formatter = HoverFormatter::new(config, metadata_lookup);
//!
//! // Форматирование hover для переменной
//! // let hover = formatter.format_variable("МассивДанных", &resolution);
//! // println!("{}", hover);
//! ```
//!
//! # Архитектурные преимущества
//!
//! - ✅ Устранение ~150 строк дублированного кода
//! - ✅ Конфигурируемые лимиты для компактных tooltips
//! - ✅ Separation of Concerns — TypeSystemService делегирует форматирование
//! - ✅ Переиспользование в LSP/Web/CLI

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::types::{Certainty, ResolutionResult, TypeResolution};
use bsl_shared::formatting::DetailLevel; // MILESTONE 3.6 Phase 1

/// Порог уверенности, ниже которого тип считается "низкой уверенности"
/// и проверяется существование объекта метаданных.
/// Используется в check_unknown_metadata_object() для определения,
/// нужно ли показывать hover с ошибкой вместо стандартного hover.
const LOW_CONFIDENCE_THRESHOLD: f32 = 0.6;

/// Формат вывода hover информации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Markdown для LSP hover
    Markdown,
    /// Plain text для CLI
    PlainText,
}

/// Тема оформления (для будущего использования с темами VSCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

/// Локаль для текстовых сообщений
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Ru,
}

/// Конфигурация форматирования hover
#[derive(Debug, Clone)]
pub struct HoverFormatConfig {
    /// Максимальное количество методов для отображения
    pub max_methods: usize,
    /// Максимальное количество свойств для отображения
    pub max_properties: usize,
    /// MILESTONE 3.6 Phase 1: Уровень детализации (compact/full/detailed)
    pub detail_level: DetailLevel,
    /// MILESTONE 3.6 Phase 1: Показывать ли уверенность в типе (🟢🟡⚪)
    pub show_certainty: bool,
    /// MILESTONE 3.6 Phase 2 - Task 2.3: Путь к Syntax Helper для документации
    pub syntax_helper_path: Option<std::path::PathBuf>,
    /// Формат вывода
    pub output_format: OutputFormat,
    /// Тема оформления
    pub theme: Theme,
    /// Локаль
    pub locale: Locale,
}

impl Default for HoverFormatConfig {
    fn default() -> Self {
        Self {
            max_methods: 10,
            max_properties: 5,
            detail_level: DetailLevel::Detailed,
            show_certainty: true,
            syntax_helper_path: None, // По умолчанию нет
            output_format: OutputFormat::Markdown,
            theme: Theme::Dark,
            locale: Locale::Ru,
        }
    }
}

/// Главный компонент для форматирования hover responses
///
///  # Примеры
///
/// ```rust,no_run
/// use bsl_backend::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig};
/// use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
/// use bsl_shared::domain::repository::InMemoryTypeRepository;
/// use std::sync::Arc;
///
/// let repo = Arc::new(InMemoryTypeRepository::new());
/// let metadata_lookup = TypeMetadataLookup::new(repo);
/// let config = HoverFormatConfig::default();
///
/// let formatter = HoverFormatter::new(config, metadata_lookup);
/// ```
#[derive(Clone)] // MILESTONE 3.6 Phase 1: Нужен Clone для fallback
pub struct HoverFormatter {
    config: HoverFormatConfig,
    metadata_lookup: TypeMetadataLookup,
}

impl HoverFormatter {
    /// Создать новый форматтер с указанной конфигурацией
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация форматирования (лимиты, формат вывода)
    /// * `metadata_lookup` - сервис для получения метаданных типов
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use bsl_backend::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig, OutputFormat};
    /// use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use std::sync::Arc;
    ///
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let metadata_lookup = TypeMetadataLookup::new(repo);
    ///
    /// let config = HoverFormatConfig {
    ///     max_methods: 5,
    ///     max_properties: 3,
    ///     output_format: OutputFormat::PlainText,
    ///     ..Default::default()
    /// };
    ///
    /// let formatter = HoverFormatter::new(config, metadata_lookup);
    /// ```
    pub fn new(config: HoverFormatConfig, metadata_lookup: TypeMetadataLookup) -> Self {
        Self {
            config,
            metadata_lookup,
        }
    }

    /// Форматировать hover для переменной
    ///
    /// Создаёт форматированное hover сообщение с информацией о типе переменной,
    /// включая методы, свойства и уровень уверенности (certainty).
    ///
    /// # Аргументы
    ///
    /// * `name` - имя переменной
    /// * `resolution` - результат анализа типа (TypeResolution)
    ///
    /// # Возвращает
    ///
    /// Форматированную строку в формате Markdown или PlainText (в зависимости от конфигурации)
    ///
    /// # Поведение
    ///
    /// - Если `Certainty::Unknown` — показывает предупреждение о нераспознанном типе
    /// - Если нет метаданных — показывает предупреждение "Детали типа недоступны"
    /// - Если метаданные найдены — показывает полную информацию с методами и свойствами
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use bsl_backend::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig};
    /// use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult, ConcreteType, PlatformType};
    /// use std::sync::Arc;
    ///
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let metadata_lookup = TypeMetadataLookup::new(repo);
    /// let formatter = HoverFormatter::new(HoverFormatConfig::default(), metadata_lookup);
    ///
    /// let resolution = TypeResolution {
    ///     result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
    ///         name: "Массив".to_string(),
    ///     })),
    ///     certainty: Certainty::Known,
    ///     source: bsl_shared::domain::types::ResolutionSource::Static,
    ///     metadata: Default::default(),
    ///     active_facet: None,
    ///     available_facets: vec![],
    /// };
    ///
    /// let hover = formatter.format_variable("МассивДанных", &resolution);
    /// // hover содержит: "Переменная: МассивДанных\nТип: Массив\nУверенность: 🟢 Known (100%)\n..."
    /// ```
    pub fn format_variable(&self, name: &str, resolution: &TypeResolution) -> String {
        use bsl_shared::domain::types::Certainty;

        // NOTE: Ошибки о несуществующих объектах метаданных показываются через Diagnostics,
        // а не через Hover. Hover показывает информацию о типе.

        // Проверка наличия метаданных
        let has_metadata = self.metadata_lookup.get_raw_type(resolution).is_some();

        // Если Unknown — показываем специальное предупреждение
        if matches!(resolution.certainty, Certainty::Unknown) {
            return HoverBuilder::new(&self.config)
                .add_header("Переменная", name)
                .add_type_info(resolution)
                .add_certainty(&resolution.certainty)
                .add_section("⚠️", "**Тип не распознан системой**")
                .add_section("💡", "*Возможные причины:*\n• Опечатка в имени типа\n• Тип из конфигурации 1С (требуется Configuration Loader)")
                .build();
        }

        // MILESTONE 3.6 Phase 2: Generic типы могут показывать информацию даже без метаданных
        let is_generic = matches!(resolution.result, ResolutionResult::Generic(_));

        // Если нет метаданных И это НЕ Generic тип — показываем предупреждение
        if !has_metadata && !is_generic {
            return HoverBuilder::new(&self.config)
                .add_header("Переменная", name)
                .add_type_info(resolution)
                .add_certainty(&resolution.certainty)
                .add_section("⚠️", "**Детали типа недоступны**")
                .add_section("💡", "*Возможные причины:*\n• Тип не загружен из Syntax Helper\n• Требуется парсинг документации платформы")
                .build();
        }

        // MILESTONE 3.6 Phase 1: Выбор формата в зависимости от detail_level
        match self.config.detail_level {
            DetailLevel::Compact => {
                // Только тип + certainty (если включено)
                HoverBuilder::new(&self.config)
                    .add_header("Переменная", name)
                    .add_type_info(resolution)
                    .add_certainty(&resolution.certainty)
                    .build()
            }
            DetailLevel::Full => {
                // Тип + методы (до max_methods)
                HoverBuilder::new(&self.config)
                    .add_header("Переменная", name)
                    .add_type_info(resolution)
                    .add_certainty(&resolution.certainty)
                    .add_methods(resolution, &self.metadata_lookup)
                    .build()
            }
            DetailLevel::Detailed => {
                // Полный hover с методами, свойствами, фасетами и документацией (Phase 2)
                // Порядок: тип → фасеты → свойства (state) → табличные части → методы (behavior) → документация
                HoverBuilder::new(&self.config)
                    .add_header("Переменная", name)
                    .add_type_info(resolution)
                    .add_certainty(&resolution.certainty)
                    .add_facet_info(resolution)              // ← MILESTONE 3.6 Phase 2: Task 2.1
                    .add_generic_info(resolution)            // ← MILESTONE 3.6 Phase 2: Task 2.2
                    .add_properties(resolution, &self.metadata_lookup) // ← Свойства ПЕРЕД методами (best practice)
                    .add_tabular_sections(resolution, &self.metadata_lookup) // ← Табличные части
                    .add_methods(resolution, &self.metadata_lookup)
                    .add_documentation_links(resolution)    // ← MILESTONE 3.6 Phase 2: Task 2.4
                    .build()
            }
        }
    }

    /// Форматировать hover для функции (stub для будущего)
    pub fn format_function(&self, name: &str, signature: &str) -> String {
        HoverBuilder::new(&self.config)
            .add_header("Функция", name)
            .add_section("Сигнатура", signature)
            .build()
    }

    /// Форматирует hover для несуществующего объекта метаданных
    ///
    /// MILESTONE 3.16: Отображает информативное сообщение об ошибке
    /// с предложениями похожих имён (fuzzy matching).
    ///
    /// NOTE: В текущей реализации ошибки показываются через Diagnostics.
    /// Этот метод оставлен для возможного использования в будущем.
    ///
    /// # Аргументы
    ///
    /// * `kind` - вид метаданных (Catalog, Document, etc.)
    /// * `name` - имя несуществующего объекта
    /// * `suggestions` - список похожих имён для предложения исправления
    ///
    /// # Формат вывода (Markdown)
    ///
    /// ```text
    /// ## Справочник "Контрагенты" не найден
    ///
    /// Объект не существует в загруженной конфигурации.
    ///
    /// ### Возможно, вы имели в виду:
    /// - `Контрагент`
    /// - `КонтрагентыПоставщики`
    ///
    /// ---
    /// *Загрузите конфигурацию командой `BSL: Parse Configuration`*
    /// ```
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use bsl_backend::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig};
    /// use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// use bsl_shared::domain::types::MetadataKind;
    /// use std::sync::Arc;
    ///
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let metadata_lookup = TypeMetadataLookup::new(repo);
    /// let formatter = HoverFormatter::new(HoverFormatConfig::default(), metadata_lookup);
    ///
    /// let hover = formatter.format_unknown_metadata_object(
    ///     MetadataKind::Catalog,
    ///     "Контрагенты",
    ///     &["Контрагент".to_string(), "Контрагенты_Поставщики".to_string()],
    /// );
    /// // hover содержит информативное сообщение с предложениями
    /// ```
    #[allow(dead_code)]
    pub fn format_unknown_metadata_object(
        &self,
        kind: bsl_shared::domain::types::MetadataKind,
        name: &str,
        suggestions: &[String],
    ) -> String {
        // Используем централизованный метод to_russian_name() из MetadataKind
        let kind_name = kind.to_russian_name();

        match self.config.output_format {
            OutputFormat::Markdown => {
                let mut result = format!(
                    "## {} \"{}\" не найден\n\n\
                     Объект не существует в загруженной конфигурации.",
                    kind_name, name
                );

                // Добавляем предложения, если есть
                if !suggestions.is_empty() {
                    result.push_str("\n\n### Возможно, вы имели в виду:\n");
                    for suggestion in suggestions {
                        result.push_str(&format!("- `{}`\n", suggestion));
                    }
                }

                result.push_str("\n---\n");
                result.push_str("*Загрузите конфигурацию командой `BSL: Parse Configuration`*");

                result
            }
            OutputFormat::PlainText => {
                let mut result = format!(
                    "{} \"{}\" не найден\n\
                     Объект не существует в загруженной конфигурации.",
                    kind_name, name
                );

                if !suggestions.is_empty() {
                    result.push_str("\n\nВозможно, вы имели в виду:\n");
                    for suggestion in suggestions {
                        result.push_str(&format!("- {}\n", suggestion));
                    }
                }

                result
            }
        }
    }

    /// Проверяет, является ли TypeResolution несуществующим объектом метаданных
    ///
    /// MILESTONE 3.16: Определяет, нужно ли показывать hover с ошибкой
    /// вместо стандартного hover.
    ///
    /// # Условия для показа ошибки:
    ///
    /// 1. Тип = Configuration с Certainty::Inferred(0.5) (низкая уверенность)
    /// 2. Конфигурация загружена (is_configuration_loaded() == true)
    /// 3. Объект НЕ существует в метаданных (exists_metadata_object() == false)
    ///
    /// # Возвращает
    ///
    /// `Some((kind, name))` если объект не найден и нужно показать hover с ошибкой
    /// `None` если объект существует или конфигурация не загружена
    ///
    /// NOTE: В текущей реализации ошибки показываются через Diagnostics.
    /// Этот метод оставлен для возможного использования в будущем.
    #[allow(dead_code)]
    pub fn check_unknown_metadata_object(
        &self,
        resolution: &TypeResolution,
    ) -> Option<(bsl_shared::domain::types::MetadataKind, String)> {
        use bsl_shared::domain::types::{Certainty, ConcreteType};

        // Проверяем что это Configuration тип с низкой уверенностью (50%)
        if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &resolution.result
        {
            // Certainty должна быть Inferred с низким значением (около 0.5)
            if let Certainty::Inferred(conf) = resolution.certainty {
                if conf <= LOW_CONFIDENCE_THRESHOLD {
                    // Проверяем загружена ли конфигурация
                    if self.metadata_lookup.is_configuration_loaded() {
                        // Проверяем существует ли объект
                        if !self
                            .metadata_lookup
                            .exists_metadata_object(config.kind, &config.name)
                        {
                            return Some((config.kind, config.name.clone()));
                        }
                    }
                }
            }
        }

        None
    }
}

/// Builder для пошагового построения hover content
struct HoverBuilder<'a> {
    config: &'a HoverFormatConfig,
    sections: Vec<String>,
}

impl<'a> HoverBuilder<'a> {
    fn new(config: &'a HoverFormatConfig) -> Self {
        Self {
            config,
            sections: Vec::new(),
        }
    }

    fn add_header(mut self, label: &str, value: &str) -> Self {
        let section = match self.config.output_format {
            OutputFormat::Markdown => format!("**{}:** {}", label, value),
            OutputFormat::PlainText => format!("{}: {}", label, value),
        };
        self.sections.push(section);
        self
    }

    fn add_section(mut self, label: &str, value: &str) -> Self {
        let section = match self.config.output_format {
            OutputFormat::Markdown => format!("**{}:** {}", label, value),
            OutputFormat::PlainText => format!("{}: {}", label, value),
        };
        self.sections.push(section);
        self
    }

    fn add_type_info(self, resolution: &TypeResolution) -> Self {
        let type_str = match &resolution.result {
            ResolutionResult::Concrete(concrete_type) => {
                format!("{}", concrete_type)
            }
            ResolutionResult::Generic(generic_type) => {
                // Формат: Массив<Строка>
                let params = generic_type
                    .type_params
                    .iter()
                    .map(|p| format!("{}", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", generic_type.base_type, params)
            }
            ResolutionResult::Union(union_types) => {
                // Формат: Строка | Число
                union_types
                    .iter()
                    .map(|weighted| format!("{}", weighted.type_))
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            ResolutionResult::Intersection(intersection_types) => {
                // Формат: ТипА & ТипВ
                intersection_types
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect::<Vec<_>>()
                    .join(" & ")
            }
            ResolutionResult::Dynamic => "Неопределено".to_string(),
            ResolutionResult::Nullable(inner) => {
                format!("{} | Неопределено", inner)
            }
        };

        self.add_section("Тип", &type_str)
    }

    fn add_certainty(self, certainty: &Certainty) -> Self {
        // MILESTONE 3.6 Phase 1: Показывать certainty только если включено
        if !self.config.show_certainty {
            return self;
        }

        let certainty_str = match certainty {
            Certainty::Known => "🟢 Known (100%)".to_string(),
            Certainty::Inferred(conf) => {
                let percentage = (conf * 100.0) as u8;
                format!("🟡 Inferred ({}%)", percentage)
            }
            Certainty::Unknown => "⚪ Unknown (0%)".to_string(),
        };

        self.add_section("Уверенность", &certainty_str)
    }

    fn add_methods(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        let methods = metadata_lookup.get_methods(resolution);

        if !methods.is_empty() {
            let total_count = methods.len();
            // ИСПРАВЛЕНИЕ: Для DetailLevel::Detailed показываем ВСЕ методы без ограничений
            let display_count = if matches!(self.config.detail_level, DetailLevel::Detailed) {
                total_count
            } else {
                self.config.max_methods.min(total_count)
            };

            let mut method_lines = vec![format!(
                "Методы (показано {} из {}):",
                display_count, total_count
            )];

            for method in methods.iter().take(display_count) {
                // MILESTONE 3.6 Phase 1: Multiline formatting для методов с 4+ параметров
                let param_count = method.params.len();
                let return_str = if method.return_type.is_empty() {
                    "void".to_string()
                } else {
                    method.return_type.clone()
                };

                // MILESTONE 3.11 Phase 4: Context badge для методов
                let context_badge = if matches!(self.config.detail_level, DetailLevel::Detailed) {
                    method.context_requirements.as_ref().map(|req| {
                        use bsl_shared::domain::runtime_context::ContextRequirements;
                        match req {
                            ContextRequirements::ServerOnly => " (🖥️ Server)",
                            ContextRequirements::ClientOnly => " (💻 Client)",
                            ContextRequirements::Universal => " (🌐 Universal)",
                            ContextRequirements::ServerPreferred => " (⚡ Server Preferred)",
                        }
                    }).unwrap_or("")
                } else {
                    ""
                };

                let line = if param_count >= 4 {
                    // Multiline формат для методов с 4+ параметров
                    let mut result = match self.config.output_format {
                        OutputFormat::Markdown => format!("• **{}**(\n", method.name),
                        OutputFormat::PlainText => format!("  - {}(\n", method.name),
                    };

                    for (i, param) in method.params.iter().enumerate() {
                        let optional_marker = if param.is_optional { "?" } else { "" };
                        let default_suffix = param
                            .default_value
                            .as_ref()
                            .map(|v| format!(" = {}", v))
                            .unwrap_or_default();
                        let comma = if i < param_count - 1 { "," } else { "" };

                        result.push_str(&format!(
                            "    {}{}: {}{}{}\n",
                            param.name, optional_marker, param.param_type, default_suffix, comma
                        ));
                    }

                    result.push_str(&format!("  ) → {}{}", return_str, context_badge));
                    result
                } else {
                    // Inline формат для методов с < 4 параметров
                    let params_str = method
                        .params
                        .iter()
                        .map(|p| {
                            let optional_marker = if p.is_optional { "?" } else { "" };
                            let default_suffix = p
                                .default_value
                                .as_ref()
                                .map(|v| format!(" = {}", v))
                                .unwrap_or_default();
                            format!(
                                "{}{}: {}{}",
                                p.name, optional_marker, p.param_type, default_suffix
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    match self.config.output_format {
                        OutputFormat::Markdown => {
                            format!("• **{}({})** → {}{}", method.name, params_str, return_str, context_badge)
                        }
                        OutputFormat::PlainText => {
                            format!("  - {}({}) → {}{}", method.name, params_str, return_str, context_badge)
                        }
                    }
                };

                method_lines.push(line);
            }

            if total_count > display_count {
                method_lines.push(format!(
                    "\n... и ещё {} методов",
                    total_count - display_count
                ));
            }

            // ИСПРАВЛЕНИЕ: Используем "  \n" (два пробела + \n) для Markdown hard break в VSCode hover
            self.sections.push(method_lines.join("  \n"));
        }

        self
    }

    fn add_properties(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        // Фильтрация по фасету теперь происходит внутри get_properties()
        let properties = metadata_lookup.get_properties(resolution);

        if !properties.is_empty() {
            let total_count = properties.len();
            // ИСПРАВЛЕНИЕ: Для DetailLevel::Detailed показываем ВСЕ свойства без ограничений
            let display_count = if matches!(self.config.detail_level, DetailLevel::Detailed) {
                total_count
            } else {
                self.config.max_properties.min(total_count)
            };

            let mut property_lines = vec![format!(
                "Свойства (показано {} из {}):",
                display_count, total_count
            )];

            for property in properties.iter().take(display_count) {
                let line = match self.config.output_format {
                    OutputFormat::Markdown => {
                        format!("• **{}**: {}", property.name, property.prop_type)
                    }
                    OutputFormat::PlainText => {
                        format!("  - {}: {}", property.name, property.prop_type)
                    }
                };
                property_lines.push(line);
            }

            if total_count > display_count {
                property_lines.push(format!(
                    "\n... и ещё {} свойств",
                    total_count - display_count
                ));
            }

            // ИСПРАВЛЕНИЕ: Используем "  \n" (два пробела + \n) для Markdown hard break в VSCode hover
            self.sections.push(property_lines.join("  \n"));
        }

        self
    }

    /// Добавить информацию о табличных частях (только для Detailed level)
    ///
    /// Табличные части отображаются для конфигурационных типов (Документы, Справочники)
    /// с фасетами Object или Reference.
    fn add_tabular_sections(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        // Только для Detailed уровня
        if !matches!(self.config.detail_level, DetailLevel::Detailed) {
            return self;
        }

        let sections = metadata_lookup.get_tabular_sections(resolution);

        if sections.is_empty() {
            return self;
        }

        let total = sections.len();
        // Для Detailed показываем все табличные части, для остальных - max_properties
        let display_count = total; // Показываем все табличные части

        let mut lines = vec![format!(
            "Табличные части (показано {} из {}):",
            display_count, total
        )];

        for section in sections.iter().take(display_count) {
            let attr_count = section.attributes.len();
            let line = match self.config.output_format {
                OutputFormat::Markdown => {
                    format!("• **{}** ({} колонок)", section.name, attr_count)
                }
                OutputFormat::PlainText => {
                    format!("  - {} ({} колонок)", section.name, attr_count)
                }
            };
            lines.push(line);
        }

        if total > display_count {
            lines.push(format!("... и ещё {} табличных частей", total - display_count));
        }

        // ИСПРАВЛЕНИЕ: Используем "  \n" (два пробела + \n) для Markdown hard break в VSCode hover
        self.sections.push(lines.join("  \n"));
        self
    }

    /// MILESTONE 3.11 Phase 4: Добавить информацию о фасете (для Detailed level)
    fn add_facet_info(mut self, resolution: &TypeResolution) -> Self {
        // Только для Detailed уровня
        if !matches!(self.config.detail_level, DetailLevel::Detailed) {
            return self;
        }

        // Получить активный фасет
        if let Some(active_facet) = &resolution.active_facet {
            let (facet_russian, facet_description) = match active_facet {
                bsl_shared::domain::types::FacetKind::Manager => ("Менеджер", "создание, поиск элементов"),
                bsl_shared::domain::types::FacetKind::Object => ("Объект", "изменяемый объект"),
                bsl_shared::domain::types::FacetKind::Reference => ("Ссылка", "ссылка на элемент"),
                bsl_shared::domain::types::FacetKind::Selection => ("Выборка", "обход элементов"),
                bsl_shared::domain::types::FacetKind::List => ("Список", "UI представление"),
                bsl_shared::domain::types::FacetKind::Metadata => ("Метаданные", "метаданные объекта"),
                bsl_shared::domain::types::FacetKind::Constructor => ("Конструктор", "создание объектов"),
                bsl_shared::domain::types::FacetKind::Collection => ("Коллекция", "набор элементов"),
                bsl_shared::domain::types::FacetKind::Singleton => ("Одиночный", "одиночный объект"),
            };

            let facet_info = format!("**Фасет:** {} ({})", facet_russian, facet_description);
            self.sections.push(facet_info);

            // Показать доступные фасеты для данного типа
            if !resolution.available_facets.is_empty() {
                let facets_list = resolution
                    .available_facets
                    .iter()
                    .map(|f| match f {
                        bsl_shared::domain::types::FacetKind::Manager => "Менеджер",
                        bsl_shared::domain::types::FacetKind::Object => "Объект",
                        bsl_shared::domain::types::FacetKind::Reference => "Ссылка",
                        bsl_shared::domain::types::FacetKind::Selection => "Выборка",
                        bsl_shared::domain::types::FacetKind::List => "Список",
                        bsl_shared::domain::types::FacetKind::Metadata => "Метаданные",
                        bsl_shared::domain::types::FacetKind::Constructor => "Конструктор",
                        bsl_shared::domain::types::FacetKind::Collection => "Коллекция",
                        bsl_shared::domain::types::FacetKind::Singleton => "Одиночный",
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                self.sections.push(format!("💡 **Доступные фасеты:** {}", facets_list));
            }
        }

        self
    }

    /// MILESTONE 3.6 Phase 2 - Task 2.2: Добавить пояснение для Generic типов (только для Detailed level)
    fn add_generic_info(mut self, resolution: &TypeResolution) -> Self {
        // Только для Detailed уровня
        if !matches!(self.config.detail_level, DetailLevel::Detailed) {
            return self;
        }

        // Проверить что это Generic тип
        if let ResolutionResult::Generic(generic) = &resolution.result {
            let params_str = generic
                .type_params
                .iter()
                .map(|p| format!("{}", p))
                .collect::<Vec<_>>()
                .join(", ");

            let generic_explanation = format!(
                "💡 **Generic тип:**\n• Базовый тип: {}\n• Параметры типа: {}",
                generic.base_type, params_str
            );

            self.sections.push(generic_explanation);

            // Добавить пояснение что означает Generic тип
            let explanation = match generic.base_type.as_str() {
                "Массив" | "Array" => {
                    "Generic тип означает, что массив содержит элементы определённого типа"
                }
                "Соответствие" | "Map" => {
                    "Generic тип означает, что соответствие хранит пары ключ-значение определённых типов"
                }
                "ТаблицаЗначений" | "ValueTable" => {
                    "Generic тип означает, что строки таблицы содержат данные определённого типа"
                }
                "Список" | "List" => {
                    "Generic тип означает, что список содержит элементы определённого типа"
                }
                "Структура" | "Structure" => {
                    "Generic тип означает, что структура содержит поля определённых типов"
                }
                _ => "Generic тип параметризован одним или несколькими типами",
            };

            self.sections.push(format!("ℹ️ {}", explanation));
        }

        self
    }

    /// MILESTONE 3.6 Phase 2 - Task 2.4: Добавить ссылки на документацию (только для Detailed level)
    fn add_documentation_links(mut self, resolution: &TypeResolution) -> Self {
        // Только для Detailed уровня
        if !matches!(self.config.detail_level, DetailLevel::Detailed) {
            return self;
        }

        // Получить имя типа для документации
        let type_name = match self.get_platform_type_name(resolution) {
            Some(name) => name,
            None => return self, // Нет типа платформы
        };

        let mut links = Vec::new();

        // 1. Ссылка на локальный Syntax Helper (если доступен)
        if let Some(path) = &self.config.syntax_helper_path {
            let html_path = path.join(format!("{}.html", type_name));

            if html_path.exists() {
                let file_url = format!("file:///{}", html_path.display());
                links.push(format!(
                    "[Синтакс Помощник: {}]({})",
                    type_name,
                    file_url.replace("\\", "/") // Windows path fix
                ));
            }
        }

        // 2. Ссылка на онлайн документацию 1С
        let online_url = format!("https://docs.1c.ru/search?q={}", type_name);
        links.push(format!("[1С Platform Docs]({})", online_url));

        // Добавить секцию с ссылками
        if !links.is_empty() {
            let links_section = format!(
                "📖 **Документация:**\n{}",
                links
                    .iter()
                    .map(|l| format!("• {}", l))
                    .collect::<Vec<_>>()
                    .join("  \n")  // ИСПРАВЛЕНИЕ: Markdown hard break
            );

            self.sections.push(links_section);
        }

        self
    }

    /// Получить имя типа платформы для документации
    fn get_platform_type_name(&self, resolution: &TypeResolution) -> Option<String> {
        match &resolution.result {
            ResolutionResult::Concrete(concrete_type) => {
                use bsl_shared::domain::types::ConcreteType;
                // Извлечь базовое имя типа
                match concrete_type {
                    ConcreteType::Platform(platform) => {
                        // Для платформенных типов с фасетами извлекаем базовое имя
                        // Например: "СправочникСсылка" → "СправочникСсылка"
                        //           "Массив" → "Массив"
                        Some(platform.name.clone())
                    }
                    ConcreteType::Configuration(config) => {
                        // Для конфигурационных типов используем kind.to_prefix()
                        // Например: "Справочники.Контрагенты" → "Справочник"
                        Some(config.kind.to_prefix().trim_end_matches('ы').to_string())
                    }
                    _ => None,
                }
            }
            ResolutionResult::Generic(generic) => {
                // Для Generic типов используем базовый тип
                Some(generic.base_type.clone())
            }
            _ => None,
        }
    }

    fn build(self) -> String {
        // ИСПРАВЛЕНИЕ: Используем двойной перенос для разделения секций (параграфы в Markdown)
        self.sections.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::types::{
        ConcreteType, PlatformType, RawMethodData, RawPropertyData, ResolutionMetadata,
        ResolutionSource,
    };
    use std::sync::Arc;

    #[test]
    fn test_hover_format_config_default() {
        let config = HoverFormatConfig::default();
        assert_eq!(config.max_methods, 10);
        assert_eq!(config.max_properties, 5);
        assert_eq!(config.output_format, OutputFormat::Markdown);
    }

    #[test]
    fn test_hover_builder_basic() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_header("Переменная", "МассивДанных")
            .build();

        assert!(result.contains("Переменная"));
        assert!(result.contains("МассивДанных"));
    }

    #[test]
    fn test_output_format_markdown_vs_plaintext() {
        // Test Markdown format
        let config_md = HoverFormatConfig {
            output_format: OutputFormat::Markdown,
            ..Default::default()
        };
        let result_md = HoverBuilder::new(&config_md)
            .add_header("Тест", "Значение")
            .build();
        assert!(result_md.contains("**Тест:**"));

        // Test PlainText format
        let config_txt = HoverFormatConfig {
            output_format: OutputFormat::PlainText,
            ..Default::default()
        };
        let result_txt = HoverBuilder::new(&config_txt)
            .add_header("Тест", "Значение")
            .build();
        assert!(result_txt.contains("Тест:"));
        assert!(!result_txt.contains("**"));
    }

    #[test]
    fn test_certainty_formatting_known() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_certainty(&Certainty::Known)
            .build();

        assert!(result.contains("🟢 Known (100%)"));
    }

    #[test]
    fn test_certainty_formatting_inferred() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_certainty(&Certainty::Inferred(0.85))
            .build();

        assert!(result.contains("🟡 Inferred (85%)"));
    }

    #[test]
    fn test_generic_type_formatting() {
        use bsl_shared::domain::types::{ConcreteType, GenericType, PrimitiveType};

        let generic = GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic),
            certainty: Certainty::Known,
            source: bsl_shared::domain::types::ResolutionSource::Static,
            metadata: bsl_shared::domain::types::ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_type_info(&resolution)
            .build();

        assert!(result.contains("Массив<Строка>"));
    }

    // Helper functions for testing
    fn create_test_repository_with_methods(method_count: usize) -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{FacetKind, RawDataSource, RawTypeData};

        let repo = Arc::new(InMemoryTypeRepository::new());

        let methods: Vec<RawMethodData> = (0..method_count)
            .map(|i| RawMethodData {
                name: format!("Метод{}", i),
                english_name: format!("Method{}", i),
                return_type: "Строка".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            })
            .collect();

        let test_type = RawTypeData {
            name: "ТестовыйТип".to_string(),
            english_name: "TestType".to_string(),
            description: "Тип для тестирования".to_string(),
            category: "Test".to_string(),
            source: RawDataSource::Platform,
            methods,
            properties: vec![],
            facets: vec![FacetKind::Object],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_repository_with_properties(
        property_count: usize,
    ) -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{FacetKind, RawDataSource, RawTypeData};

        let repo = Arc::new(InMemoryTypeRepository::new());

        let properties: Vec<RawPropertyData> = (0..property_count)
            .map(|i| RawPropertyData {
                name: format!("Свойство{}", i),
                prop_type: "Строка".to_string(),
                is_readonly: false,
            })
            .collect();

        let test_type = RawTypeData {
            name: "ТестовыйТип".to_string(),
            english_name: "TestType".to_string(),
            description: "Тип для тестирования".to_string(),
            category: "Test".to_string(),
            source: RawDataSource::Platform,
            methods: vec![],
            properties,
            facets: vec![FacetKind::Object],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_resolution() -> TypeResolution {
        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТестовыйТип".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    #[test]
    fn test_methods_with_limit() {
        let repo = create_test_repository_with_methods(20);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            max_methods: 10,
            detail_level: DetailLevel::Full, // Явно указываем для тестирования лимита
            ..Default::default()
        };

        let resolution = create_test_resolution();

        let result = HoverBuilder::new(&config)
            .add_methods(&resolution, &metadata_lookup)
            .build();

        assert!(result.contains("показано 10 из 20"));
        assert!(result.contains("Метод0"));
        assert!(result.contains("Метод9"));
        assert!(!result.contains("Метод10")); // За пределами лимита
        assert!(result.contains("... и ещё 10 методов"));
    }

    #[test]
    fn test_properties_with_limit() {
        let repo = create_test_repository_with_properties(10);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            max_properties: 5,
            detail_level: DetailLevel::Full, // Явно указываем для тестирования лимита
            ..Default::default()
        };

        let resolution = create_test_resolution();

        let result = HoverBuilder::new(&config)
            .add_properties(&resolution, &metadata_lookup)
            .build();

        assert!(result.contains("показано 5 из 10"));
        assert!(result.contains("Свойство0"));
        assert!(result.contains("Свойство4"));
        assert!(!result.contains("Свойство5")); // За пределами лимита
        assert!(result.contains("... и ещё 5 свойств"));
    }

    // === MILESTONE 3.16: Тесты для format_unknown_metadata_object ===

    #[test]
    fn test_format_unknown_metadata_object_markdown() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            output_format: OutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        let result = formatter.format_unknown_metadata_object(
            MetadataKind::Catalog,
            "Контрагенты",
            &["Контрагент".to_string(), "КонтрагентыПоставщики".to_string()],
        );

        // Проверяем заголовок
        assert!(result.contains("## Справочник \"Контрагенты\" не найден"));
        // Проверяем описание
        assert!(result.contains("Объект не существует в загруженной конфигурации"));
        // Проверяем предложения
        assert!(result.contains("### Возможно, вы имели в виду:"));
        assert!(result.contains("- `Контрагент`"));
        assert!(result.contains("- `КонтрагентыПоставщики`"));
        // Проверяем инструкцию
        assert!(result.contains("BSL: Parse Configuration"));
    }

    #[test]
    fn test_format_unknown_metadata_object_plaintext() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            output_format: OutputFormat::PlainText,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        let result = formatter.format_unknown_metadata_object(
            MetadataKind::Document,
            "ЗаказПокупателя",
            &[],
        );

        // Проверяем заголовок (plain text без Markdown)
        assert!(result.contains("Документ \"ЗаказПокупателя\" не найден"));
        // Проверяем что нет Markdown форматирования
        assert!(!result.contains("##"));
        // Проверяем что нет предложений (пустой массив)
        assert!(!result.contains("Возможно, вы имели в виду"));
    }

    #[test]
    fn test_format_unknown_metadata_object_without_suggestions() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let result = formatter.format_unknown_metadata_object(
            MetadataKind::Enum,
            "НесуществующееПеречисление",
            &[],
        );

        // Проверяем заголовок
        assert!(result.contains("## Перечисление \"НесуществующееПеречисление\" не найден"));
        // Проверяем что нет блока предложений
        assert!(!result.contains("### Возможно, вы имели в виду:"));
        // Но есть инструкция
        assert!(result.contains("BSL: Parse Configuration"));
    }

    #[test]
    fn test_format_unknown_metadata_object_different_kinds() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        // Тестируем разные виды метаданных
        let test_cases = vec![
            (MetadataKind::Catalog, "Справочник"),
            (MetadataKind::Document, "Документ"),
            (MetadataKind::InformationRegister, "Регистр сведений"),
            (MetadataKind::AccumulationRegister, "Регистр накопления"),
            (MetadataKind::Report, "Отчет"),
            (MetadataKind::DataProcessor, "Обработка"),
        ];

        for (kind, expected_name) in test_cases {
            let result = formatter.format_unknown_metadata_object(kind, "Тест", &[]);
            assert!(
                result.contains(&format!("{} \"Тест\" не найден", expected_name)),
                "Failed for kind {:?}: expected '{}', got: {}",
                kind,
                expected_name,
                result
            );
        }
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_none_for_known_type() {
        use bsl_shared::domain::types::ConfigurationType;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        // Known certainty - должен вернуть None
        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: bsl_shared::domain::types::MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::Known, // Known = 100% уверенность
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Должен вернуть None, т.к. certainty = Known
        assert!(formatter.check_unknown_metadata_object(&resolution).is_none());
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_none_for_platform_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        // Platform type - должен вернуть None (не Configuration)
        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "Массив".to_string(),
            })),
            certainty: Certainty::Inferred(0.5),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Должен вернуть None, т.к. это Platform type, не Configuration
        assert!(formatter.check_unknown_metadata_object(&resolution).is_none());
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_none_when_config_not_loaded() {
        use bsl_shared::domain::types::ConfigurationType;

        // Пустой репозиторий = конфигурация не загружена
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: bsl_shared::domain::types::MetadataKind::Catalog,
                name: "НесуществующийСправочник".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::Inferred(0.5),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Должен вернуть None, т.к. конфигурация не загружена
        // (is_configuration_loaded() == false)
        assert!(formatter.check_unknown_metadata_object(&resolution).is_none());
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_some_when_object_not_found() {
        use bsl_shared::domain::types::{ConfigurationType, FacetKind, RawDataSource, RawTypeData};

        // Создаём репозиторий с одним справочником
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Добавляем существующий справочник "Контрагенты"
        let existing_catalog = RawTypeData {
            name: "Справочники.Контрагенты".to_string(),
            english_name: "Catalogs.Contractors".to_string(),
            description: "Справочник контрагентов".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager],
            kind: Some(bsl_shared::domain::types::MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };
        repo.load_types(vec![existing_catalog]).unwrap();

        let metadata_lookup = TypeMetadataLookup::new(repo);
        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        // Запрашиваем НЕсуществующий справочник с низкой certainty
        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: bsl_shared::domain::types::MetadataKind::Catalog,
                name: "НесуществующийСправочник".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::Inferred(0.5), // Низкая certainty
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        // Должен вернуть Some, т.к.:
        // 1. Это Configuration type
        // 2. Certainty = Inferred(0.5) <= 0.6
        // 3. Конфигурация загружена (есть Справочники.Контрагенты)
        // 4. НесуществующийСправочник не существует
        let result = formatter.check_unknown_metadata_object(&resolution);
        assert!(result.is_some());

        let (kind, name) = result.unwrap();
        assert_eq!(kind, bsl_shared::domain::types::MetadataKind::Catalog);
        assert_eq!(name, "НесуществующийСправочник");
    }

    // REMOVED: test_format_variable_uses_unknown_metadata_hover - hover should not show errors (Milestone 3.16)

    // === Тесты для add_tabular_sections ===

    fn create_test_repository_with_tabular_sections() -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{
            FacetKind, MetadataKind, RawAttributeData, RawDataSource, RawTabularSectionData,
            RawTypeData,
        };

        let repo = Arc::new(InMemoryTypeRepository::new());

        let document = RawTypeData {
            name: "Документы.ЗаказНаряды".to_string(),
            english_name: "Documents.WorkOrders".to_string(),
            description: "Документ заказ-наряды".to_string(),
            category: "Документы".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            attributes: vec![],
            tabular_sections: vec![
                RawTabularSectionData {
                    name: "Работы".to_string(),
                    attributes: vec![
                        RawAttributeData {
                            name: "Номенклатура".to_string(),
                            attr_type: "СправочникСсылка.Номенклатура".to_string(),
                        },
                        RawAttributeData {
                            name: "Количество".to_string(),
                            attr_type: "Число".to_string(),
                        },
                    ],
                },
                RawTabularSectionData {
                    name: "Материалы".to_string(),
                    attributes: vec![RawAttributeData {
                        name: "Материал".to_string(),
                        attr_type: "СправочникСсылка.Номенклатура".to_string(),
                    }],
                },
            ],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![document]).unwrap();
        repo
    }

    fn create_config_resolution(
        type_name: &str,
        kind: bsl_shared::domain::types::MetadataKind,
        facet: Option<bsl_shared::domain::types::FacetKind>,
    ) -> TypeResolution {
        use bsl_shared::domain::types::ConfigurationType;

        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: type_name.to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: facet,
            available_facets: vec![],
        }
    }

    #[test]
    fn test_tabular_sections_in_hover() {
        use bsl_shared::domain::types::{FacetKind, MetadataKind};

        let repo = create_test_repository_with_tabular_sections();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let resolution = create_config_resolution(
            "ЗаказНаряды",
            MetadataKind::Document,
            Some(FacetKind::Object),
        );

        let result = HoverBuilder::new(&config)
            .add_tabular_sections(&resolution, &metadata_lookup)
            .build();

        // Проверяем, что табличные части отображаются
        assert!(result.contains("Табличные части"));
        assert!(result.contains("Работы"));
        assert!(result.contains("Материалы"));
        assert!(result.contains("2 колонок")); // У "Работы" 2 колонки
        assert!(result.contains("1 колонок")); // У "Материалы" 1 колонка
    }

    #[test]
    fn test_tabular_sections_empty_for_manager_facet() {
        use bsl_shared::domain::types::{FacetKind, MetadataKind};

        let repo = create_test_repository_with_tabular_sections();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let resolution = create_config_resolution(
            "ЗаказНаряды",
            MetadataKind::Document,
            Some(FacetKind::Manager), // Manager не показывает табличные части
        );

        let result = HoverBuilder::new(&config)
            .add_tabular_sections(&resolution, &metadata_lookup)
            .build();

        // Для Manager фасета табличные части не должны отображаться
        assert!(!result.contains("Табличные части"));
    }

    #[test]
    fn test_tabular_sections_only_for_detailed_level() {
        use bsl_shared::domain::types::{FacetKind, MetadataKind};

        let repo = create_test_repository_with_tabular_sections();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full, // НЕ Detailed
            ..Default::default()
        };

        let resolution = create_config_resolution(
            "ЗаказНаряды",
            MetadataKind::Document,
            Some(FacetKind::Object),
        );

        let result = HoverBuilder::new(&config)
            .add_tabular_sections(&resolution, &metadata_lookup)
            .build();

        // Для Full уровня табличные части не должны отображаться
        assert!(!result.contains("Табличные части"));
    }
}
