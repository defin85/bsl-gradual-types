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

        // Если нет метаданных — показываем предупреждение
        if !has_metadata {
            return HoverBuilder::new(&self.config)
                .add_header("Переменная", name)
                .add_type_info(resolution)
                .add_certainty(&resolution.certainty)
                .add_section("⚠️", "**Детали типа недоступны**")
                .add_section("💡", "*Возможные причины:*\n• Тип не загружен из Syntax Helper\n• Требуется парсинг документации платформы")
                .build();
        }

        // Полный hover с методами и свойствами
        HoverBuilder::new(&self.config)
            .add_header("Переменная", name)
            .add_type_info(resolution)
            .add_certainty(&resolution.certainty)
            .add_methods(resolution, &self.metadata_lookup)
            .add_properties(resolution, &self.metadata_lookup)
            .build()
    }

    /// Форматировать hover для функции (stub для будущего)
    pub fn format_function(&self, name: &str, signature: &str) -> String {
        HoverBuilder::new(&self.config)
            .add_header("Функция", name)
            .add_section("Сигнатура", signature)
            .build()
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
            let display_count = self.config.max_methods.min(total_count);

            let mut method_lines = vec![format!(
                "Методы (показано {} из {}):",
                display_count, total_count
            )];

            for method in methods.iter().take(display_count) {
                // Форматирование параметров
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

                // Форматирование возвращаемого типа
                let return_str = if method.return_type.is_empty() {
                    "void".to_string()
                } else {
                    method.return_type.clone()
                };

                let line = match self.config.output_format {
                    OutputFormat::Markdown => {
                        format!("• **{}({})** → {}", method.name, params_str, return_str)
                    }
                    OutputFormat::PlainText => {
                        format!("  - {}({}) → {}", method.name, params_str, return_str)
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

            self.sections.push(method_lines.join("\n"));
        }

        self
    }

    fn add_properties(
        mut self,
        resolution: &TypeResolution,
        metadata_lookup: &TypeMetadataLookup,
    ) -> Self {
        let properties = metadata_lookup.get_properties(resolution);

        if !properties.is_empty() {
            let total_count = properties.len();
            let display_count = self.config.max_properties.min(total_count);

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

            self.sections.push(property_lines.join("\n"));
        }

        self
    }

    fn build(self) -> String {
        self.sections.join("\n")
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
}
