//! Domain Layer: Member Resolution
//!
//! Резолюция доступа к членам типов (Справочники.Контрагенты, ТабличныеЧасти)

use crate::domain::metadata_constants::get_base_type_info;
use crate::domain::repository::TypeRepository;
use crate::domain::types::{
    Certainty, ConcreteType, ConfigurationType, FacetKind, GenericType, MetadataKind,
    RawTabularSectionData, ResolutionMetadata, ResolutionResult, ResolutionSource, TypeResolution,
    UncertaintyReason,
};
use std::sync::Arc;

/// Резолвер для доступа к членам типов
pub struct MemberResolver<'a> {
    repository: &'a Arc<dyn TypeRepository>,
}

impl<'a> MemberResolver<'a> {
    pub fn new(repository: &'a Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Check if configuration metadata is loaded
    fn is_configuration_loaded(&self) -> bool {
        self.repository.get_stats().configuration_types > 0
    }

    /// Парсинг доступа к членам вида "Base.Member"
    pub fn parse_member_access(expression: &str) -> Option<(String, String)> {
        if let Some(dot_pos) = expression.find('.') {
            let base = expression[..dot_pos].to_string();
            let member = expression[dot_pos + 1..].to_string();
            if !base.is_empty() && !member.is_empty() {
                return Some((base, member));
            }
        }
        None
    }

    /// Разрешение доступа к членам конфигурации
    pub fn resolve(&self, base: &str, member: &str) -> TypeResolution {
        // Поддержка вложенных точек (Документы.ЗаказНаряды.Работы)
        // Если member содержит точку, рекурсивно резолвим
        if member.contains('.') {
            // Пробуем склеить base.первая_часть_member и резолвить как тип
            if let Some((first_part, rest)) = member.split_once('.') {
                let potential_type = format!("{}.{}", base, first_part);

                // Проверяем, существует ли такой тип в репозитории
                if let Some(raw_type) = self.repository.find_type(&potential_type) {
                    // Тип найден — проверяем, не табличная ли часть
                    if let Some(tabular_section) =
                        raw_type.tabular_sections.iter().find(|ts| ts.name == rest)
                    {
                        return self.resolve_tabular_section_access(
                            potential_type,
                            tabular_section.clone(),
                        );
                    }
                    // Иначе пробуем рекурсивно
                    return self.resolve(&potential_type, rest);
                }
            }
        }

        // Проверяем, не является ли base конфигурационным типом с табличными частями
        if let Some(raw_type) = self.repository.find_type(base) {
            // Проверка табличных частей
            if let Some(tabular_section) = raw_type
                .tabular_sections
                .iter()
                .find(|ts| ts.name == member)
            {
                return self
                    .resolve_tabular_section_access(base.to_string(), tabular_section.clone());
            }
        }

        // MILESTONE 3.11: Распознаём как коллекции, так и фасетные типы
        // Коллекции: Справочники.X → Manager facet
        // Фасетные: СправочникМенеджер.X, СправочникОбъект.X, etc.
        // Используем централизованные константы из metadata_constants
        let (kind, facet) = match get_base_type_info(base) {
            Some((kind, facet)) => (kind, facet),
            None => {
                let mut resolution = TypeResolution::unknown();
                resolution
                    .metadata
                    .notes
                    .push(format!("Unknown base type: {}", base));
                return resolution;
            }
        };

        // Получаем префикс коллекции для поиска в repository
        let collection_prefix = kind.to_prefix();

        // MILESTONE 3.16: Three-level resolution logic
        // Формируем имя типа для поиска в repository
        let type_name = format!("{}.{}", collection_prefix, member);
        let has_metadata = self.repository.find_type(&type_name).is_some();
        let config_loaded = self.is_configuration_loaded();

        // Three-level certainty determination:
        // 1. Known (100%) - type found in loaded configuration metadata
        // 2. Inferred (50%) - configuration not loaded, syntax parsed only (graceful degradation)
        // 3. Unknown - configuration loaded but object not found (potential error)
        let (certainty, source, uncertainty_reason, note) = match (has_metadata, config_loaded) {
            // Case 1: Metadata found - full confidence
            (true, _) => (
                Certainty::Known,
                ResolutionSource::Static,
                None,
                format!(
                    "Found {} type in metadata: {}.{}",
                    Self::kind_to_name(kind),
                    base,
                    member
                ),
            ),
            // Case 2: Configuration not loaded - graceful degradation
            (false, false) => (
                Certainty::InferredWeak,
                ResolutionSource::Inferred,
                Some(UncertaintyReason::ConfigurationNotLoaded),
                format!(
                    "Inferred {} type from syntax: {}.{} (configuration not loaded)",
                    Self::kind_to_name(kind),
                    base,
                    member
                ),
            ),
            // Case 3: Configuration loaded but object not found - potential error
            (false, true) => (
                Certainty::Unknown,
                ResolutionSource::Static,
                Some(UncertaintyReason::MetadataObjectNotFound {
                    kind,
                    name: member.to_string(),
                }),
                format!(
                    "{} '{}' not found in loaded configuration",
                    Self::kind_to_name_capitalized(kind),
                    member
                ),
            ),
        };

        TypeResolution {
            certainty,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: member.to_string(),
                facet: Some(facet), // Используем определённый facet (Copy trait)
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source,
            metadata: ResolutionMetadata {
                file: Some(format!("{}:{}", collection_prefix, member)),
                line: None,
                column: None,
                notes: vec![note],
                uncertainty_reason,
            },
            active_facet: Some(facet),
            available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        }
    }

    /// Резолвит доступ к табличной части конфигурационного объекта
    ///
    /// # Пример
    /// ```bsl
    /// Документ = Документы.ЗаказНаряды.СоздатьДокумент();
    /// Работы = Документ.Работы;  // вызывает resolve_tabular_section_access()
    /// ```
    ///
    /// # Возвращает
    /// `TypeResolution` с `ResolutionResult::Generic`:
    /// - base_type: "ТабличнаяЧасть"
    /// - type_params: [ConcreteType::TabularRow(СтрокаРаботы)]
    pub fn resolve_tabular_section_access(
        &self,
        parent_type: String,
        tabular_section: RawTabularSectionData,
    ) -> TypeResolution {
        use crate::domain::types::TabularRowType;

        tracing::debug!(
            "Резолюция табличной части: {}.{}",
            parent_type,
            tabular_section.name
        );

        // 1. Создаём TabularRowType для строки табличной части
        let row_type = TabularRowType::new(
            parent_type.clone(),
            tabular_section.name.clone(),
            tabular_section.attributes.clone(),
        );

        tracing::trace!(
            "  Создан TabularRowType: {} с {} атрибутами",
            row_type.get_full_name(),
            row_type.attributes.len()
        );

        // 2. Оборачиваем в Generic тип: ТабличнаяЧасть<СтрокаРаботы>
        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        tracing::debug!(
            "  Создан Generic тип: ТабличнаяЧасть<{}>",
            tabular_section.name
        );

        // 3. Возвращаем резолюцию с высокой уверенностью
        TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known, // 100% - данные из метаданных
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: Some(format!("{}.{}", parent_type, tabular_section.name)),
                line: None,
                column: None,
                notes: vec![format!(
                    "Табличная часть '{}' с {} атрибутами",
                    tabular_section.name,
                    tabular_section.attributes.len()
                )],
                uncertainty_reason: None,
            },
            active_facet: Some(FacetKind::Collection), // Табличная часть - это коллекция
            available_facets: vec![FacetKind::Collection],
        }
    }

    /// Конвертация MetadataKind в строку для сообщений (lowercase)
    fn kind_to_name(kind: MetadataKind) -> &'static str {
        match kind {
            MetadataKind::Catalog => "catalog",
            MetadataKind::Document => "document",
            MetadataKind::Enum => "enum",
            MetadataKind::InformationRegister => "information register",
            MetadataKind::AccumulationRegister => "accumulation register",
            _ => "configuration object",
        }
    }

    /// Конвертация MetadataKind в строку для сообщений (capitalized)
    fn kind_to_name_capitalized(kind: MetadataKind) -> &'static str {
        match kind {
            MetadataKind::Catalog => "Catalog",
            MetadataKind::Document => "Document",
            MetadataKind::Enum => "Enum",
            MetadataKind::InformationRegister => "Information register",
            MetadataKind::AccumulationRegister => "Accumulation register",
            _ => "Configuration object",
        }
    }
}
