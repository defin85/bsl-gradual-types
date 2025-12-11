//! HoverFormatter - основной компонент для форматирования hover responses
//!
//! Предоставляет методы для форматирования hover информации для переменных,
//! функций и объектов метаданных.

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::types::{Certainty, ResolutionResult, TypeResolution};
use bsl_shared::formatting::DetailLevel;

use super::builder::HoverBuilder;
use super::config::{HoverFormatConfig, HoverOutputFormat, LOW_CONFIDENCE_THRESHOLD};

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
#[derive(Clone)]
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
    /// Форматированную строку в формате Markdown или PlainText
    pub fn format_variable(&self, name: &str, resolution: &TypeResolution) -> String {
        // NOTE: Ошибки о несуществующих объектах метаданных показываются через Diagnostics

        // Проверка наличия метаданных
        let has_metadata = self.metadata_lookup.get_raw_type(resolution).is_some();

        // Если Unknown — показываем специальное предупреждение
        if matches!(resolution.certainty, Certainty::Unknown) {
            return HoverBuilder::new(&self.config)
                .add_header("Переменная", name)
                .add_type_info(resolution)
                .add_certainty(&resolution.certainty)
                .add_section("", "**Тип не распознан системой**")
                .add_section("", "*Возможные причины:*\n* Опечатка в имени типа\n* Тип из конфигурации 1С (требуется Configuration Loader)")
                .build();
        }

        // Generic типы могут показывать информацию даже без метаданных
        let is_generic = matches!(resolution.result, ResolutionResult::Generic(_));

        // Если нет метаданных И это НЕ Generic тип — показываем предупреждение
        if !has_metadata && !is_generic {
            return HoverBuilder::new(&self.config)
                .add_header("Переменная", name)
                .add_type_info(resolution)
                .add_certainty(&resolution.certainty)
                .add_section("", "**Детали типа недоступны**")
                .add_section("", "*Возможные причины:*\n* Тип не загружен из Syntax Helper\n* Требуется парсинг документации платформы")
                .build();
        }

        // Выбор формата в зависимости от detail_level
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
                // Полный hover с методами, свойствами, фасетами и документацией
                // Порядок: тип -> фасеты -> свойства (state) -> табличные части -> методы (behavior) -> документация
                HoverBuilder::new(&self.config)
                    .add_header("Переменная", name)
                    .add_type_info(resolution)
                    .add_certainty(&resolution.certainty)
                    .add_facet_info(resolution)
                    .add_generic_info(resolution)
                    .add_properties(resolution, &self.metadata_lookup)
                    .add_tabular_sections(resolution, &self.metadata_lookup)
                    .add_methods(resolution, &self.metadata_lookup)
                    .add_documentation_links(resolution)
                    .build()
            }
        }
    }

    /// Форматировать hover для функции
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
    #[allow(dead_code)]
    pub fn format_unknown_metadata_object(
        &self,
        kind: bsl_shared::domain::types::MetadataKind,
        name: &str,
        suggestions: &[String],
    ) -> String {
        let kind_name = kind.to_russian_name();

        match self.config.output_format {
            HoverOutputFormat::Markdown => {
                let mut result = format!(
                    "## {} \"{}\" не найден\n\n\
                     Объект не существует в загруженной конфигурации.",
                    kind_name, name
                );

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
            HoverOutputFormat::PlainText => {
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
    #[allow(dead_code)]
    pub fn check_unknown_metadata_object(
        &self,
        resolution: &TypeResolution,
    ) -> Option<(bsl_shared::domain::types::MetadataKind, String)> {
        use bsl_shared::domain::types::ConcreteType;

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
