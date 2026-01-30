//! HoverFormatter - основной компонент для форматирования hover responses
//!
//! Предоставляет методы для форматирования hover информации для переменных,
//! функций и объектов метаданных.

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::signature_index::MethodSignature;
use bsl_shared::domain::types::{Certainty, ConcreteType, ResolutionResult, TypeResolution};
use bsl_shared::formatting::DetailLevel;

use super::builder::HoverBuilder;
use super::config::{HoverFormatConfig, HoverOutputFormat};

/// Главный компонент для форматирования hover responses
///
///  # Примеры
///
/// ```rust,no_run
/// use bsl_runtime::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig};
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
        let platform_docs_loaded = self.metadata_lookup.platform_docs_loaded();
        let is_platform_like = is_platform_like_resolution(resolution);
        let has_metadata = if is_platform_like && !platform_docs_loaded {
            false
        } else {
            has_metadata
        };

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
                let description = self.metadata_lookup.get_description(resolution);
                let mut builder = HoverBuilder::new(&self.config)
                    .add_header("Переменная", name)
                    .add_type_info(resolution)
                    .add_certainty(&resolution.certainty);

                if !description.trim().is_empty() {
                    builder = builder.add_section("Описание", description.trim());
                }

                builder
                    .add_methods(resolution, &self.metadata_lookup)
                    .build()
            }
            DetailLevel::Detailed => {
                // Полный hover с методами, свойствами, фасетами и документацией
                // Порядок: тип -> фасеты -> свойства (state) -> табличные части -> методы (behavior) -> документация
                let description = self.metadata_lookup.get_description(resolution);
                let mut builder = HoverBuilder::new(&self.config)
                    .add_header("Переменная", name)
                    .add_type_info(resolution)
                    .add_certainty(&resolution.certainty)
                    .add_facet_info(resolution)
                    .add_generic_info(resolution);

                if !description.trim().is_empty() {
                    builder = builder.add_section("Описание", description.trim());
                }

                builder
                    .add_properties(resolution, &self.metadata_lookup)
                    .add_tabular_sections(resolution, &self.metadata_lookup)
                    .add_methods(resolution, &self.metadata_lookup)
                    .add_documentation_links(resolution)
                    .build()
            }
        }
    }

    /// Форматировать hover для свойства (obj.Property)
    ///
    /// Показывает информацию о свойстве (имя, readonly) и тип значения свойства,
    /// включая методы/свойства самого типа (как у обычной переменной).
    pub fn format_property(
        &self,
        owner_name: Option<&str>,
        owner_resolution: &TypeResolution,
        property_name: &str,
        property_resolution: &TypeResolution,
        is_readonly: Option<bool>,
    ) -> String {
        let display_name = if let Some(owner) = owner_name {
            format!("{}.{}", owner, property_name)
        } else {
            property_name.to_string()
        };

        let readonly_str = is_readonly
            .map(|v| if v { "Да" } else { "Нет" })
            .unwrap_or("?");

        match self.config.detail_level {
            DetailLevel::Compact => HoverBuilder::new(&self.config)
                .add_header("Свойство", &display_name)
                .add_section("Владелец", &owner_resolution.type_name())
                .add_section("Только чтение", readonly_str)
                .add_type_info(property_resolution)
                .add_certainty(&property_resolution.certainty)
                .build(),
            DetailLevel::Full | DetailLevel::Detailed => {
                let description = self.metadata_lookup.get_description(property_resolution);
                let mut builder = HoverBuilder::new(&self.config)
                    .add_header("Свойство", &display_name)
                    .add_section("Владелец", &owner_resolution.type_name())
                    .add_section("Только чтение", readonly_str)
                    .add_type_info(property_resolution)
                    .add_certainty(&property_resolution.certainty)
                    .add_facet_info(property_resolution)
                    .add_generic_info(property_resolution);

                if !description.trim().is_empty() {
                    builder = builder.add_section("Описание", description.trim());
                }

                builder
                    .add_properties(property_resolution, &self.metadata_lookup)
                    .add_tabular_sections(property_resolution, &self.metadata_lookup)
                    .add_methods(property_resolution, &self.metadata_lookup)
                    .add_documentation_links(property_resolution)
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

    /// Форматировать hover для вызова функции/метода
    pub fn format_function_signature(&self, label: &str, signature: &MethodSignature) -> String {
        let params_str = signature
            .params
            .iter()
            .map(|p| {
                let type_str = p.type_name.as_deref().unwrap_or("Any");
                if p.is_optional {
                    format!("[{}: {}]", p.name, type_str)
                } else {
                    format!("{}: {}", p.name, type_str)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let signature_str = if let Some(return_type) = signature
            .return_type
            .as_deref()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
        {
            format!("{}({}) -> {}", signature.name, params_str, return_type)
        } else {
            format!("{}({})", signature.name, params_str)
        };

        let mut builder = HoverBuilder::new(&self.config)
            .add_header(label, &signature.name)
            .add_section("Сигнатура", &signature_str);

        if let Some(description) = signature
            .description
            .as_ref()
            .map(|d| d.trim())
            .filter(|d| !d.is_empty())
        {
            builder = builder.add_section("Описание", description);
        }

        if let Some(return_description) = signature
            .return_description
            .as_ref()
            .map(|d| d.trim())
            .filter(|d| !d.is_empty())
        {
            builder = builder.add_section("Описание возврата", return_description);
        }

        builder.build()
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

        // Проверяем что это Configuration тип с низкой уверенностью (InferredWeak)
        if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) = &resolution.result
        {
            // Certainty должна быть InferredWeak (бывший Inferred с низким значением)
            if matches!(resolution.certainty, Certainty::InferredWeak) {
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

        None
    }
}

fn is_platform_like_resolution(resolution: &TypeResolution) -> bool {
    match &resolution.result {
        ResolutionResult::Concrete(concrete) => matches!(
            concrete,
            ConcreteType::Platform(_) | ConcreteType::Primitive(_) | ConcreteType::Special(_)
        ),
        ResolutionResult::Nullable(inner) => matches!(
            inner.as_ref(),
            ConcreteType::Platform(_) | ConcreteType::Primitive(_) | ConcreteType::Special(_)
        ),
        ResolutionResult::Intersection(types) => types.iter().any(|t| {
            matches!(
                t,
                ConcreteType::Platform(_) | ConcreteType::Primitive(_) | ConcreteType::Special(_)
            )
        }),
        ResolutionResult::Union(variants) => variants.iter().any(|wt| {
            matches!(
                wt.type_,
                ConcreteType::Platform(_) | ConcreteType::Primitive(_) | ConcreteType::Special(_)
            )
        }),
        _ => false,
    }
}
