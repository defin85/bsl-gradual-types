//! Domain Layer: Type Resolver
//!
//! Чистая бизнес-логика разрешения типов без Application concerns

use std::sync::Arc;
use crate::domain::repository::TypeRepository;
use crate::domain::types::TypeResolution;

/// Чистый Domain resolver - только бизнес-логика типизации
pub struct TypeResolver {
    repository: Arc<dyn TypeRepository>,
}

impl TypeResolver {
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    /// Синхронное разрешение выражения (чистая Domain логика)
    pub fn resolve_expression_sync(&self, expression: &str) -> TypeResolution {
        // 1. Прямой поиск в repository
        if let Some(raw_type) = self.repository.find_type(expression) {
            return self.create_resolution_from_raw(&raw_type);
        }

        // 2. Парсинг и разрешение составных имен (Справочники.Контрагенты)
        if let Some((base, member)) = self.parse_member_access(expression) {
            return self.resolve_member_access(&base, &member);
        }

        // 3. Union types (пока не реализовано)
        if expression.contains(',') {
            // TODO: resolve_union_type
        }

        TypeResolution::unknown()
    }

    /// Преобразование RawTypeData в TypeResolution (чистая логика)
    fn create_resolution_from_raw(&self, raw_type: &crate::domain::types::RawTypeData) -> TypeResolution {
        let mut resolution = TypeResolution::known(
            crate::domain::types::ConcreteType::Platform(
                crate::domain::types::PlatformType {
                    name: raw_type.name.clone()
                }
            )
        );
        // Копируем фасеты из RawTypeData
        resolution.available_facets = raw_type.facets.clone();
        resolution
    }

    /// Парсинг доступа к членам вида \"Base.Member\"
    fn parse_member_access(&self, expression: &str) -> Option<(String, String)> {
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
    fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
        use crate::domain::types::{
            Certainty, ConcreteType, ConfigurationType, MetadataKind, ResolutionMetadata,
            ResolutionResult, ResolutionSource,
        };

        let (kind, prefix) = match base {
            "Справочники" | "Catalogs" => (MetadataKind::Catalog, "Справочники"),
            "Документы" | "Documents" => (MetadataKind::Document, "Документы"),
            "Перечисления" | "Enums" => (MetadataKind::Enum, "Перечисления"),
            "РегистрыСведений" | "InformationRegisters" => {
                (MetadataKind::Register, "РегистрыСведений")
            }
            _ => {
                let mut resolution = TypeResolution::unknown();
                resolution
                    .metadata
                    .notes
                    .push(format!("Unknown base type: {}", base));
                return resolution;
            }
        };

        TypeResolution {
            certainty: Certainty::Inferred(0.8),
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: member.to_string(),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: Some(format!("{}:{}", prefix, member)),
                line: None,
                column: None,
                notes: vec![format!(
                    "Inferred {} type: {}.{}",
                    match kind {
                        MetadataKind::Catalog => "catalog",
                        MetadataKind::Document => "document",
                        MetadataKind::Enum => "enum",
                        MetadataKind::Register => "information register",
                        _ => "configuration object",
                    },
                    base,
                    member
                )],
            },
            active_facet: Some(crate::domain::types::FacetKind::Manager),
            available_facets: vec![
                crate::domain::types::FacetKind::Manager,
                crate::domain::types::FacetKind::Object,
                crate::domain::types::FacetKind::Reference,
            ],
        }
    }

    // ===== Дополнительные Domain методы =====

    /// Проверить совместимость присваивания типов (Domain логика)
    pub fn is_assignment_compatible(&self, from: &TypeResolution, to: &TypeResolution) -> bool {
        use crate::domain::types::{ResolutionResult, Certainty};

        // Если "to" - Unknown, то любое присваивание допустимо (градуальная типизация)
        if matches!(to.certainty, Certainty::Unknown) {
            return true;
        }

        // Если "from" - Unknown, допускаем присваивание с предупреждением
        if matches!(from.certainty, Certainty::Unknown) {
            return true;
        }

        // Точное совпадение типов
        match (&from.result, &to.result) {
            (ResolutionResult::Concrete(from_type), ResolutionResult::Concrete(to_type)) => {
                // Простое сравнение типов (можно расширить)
                format!("{:?}", from_type) == format!("{:?}", to_type)
            }
            (ResolutionResult::Union(_), _) => {
                // Union type - более сложная логика
                // TODO: проверить, что все члены union совместимы с to
                false
            }
            _ => false,
        }
    }

    /// Сужение типа на основе условия (flow-sensitive анализ)
    /// Например: Если ТипЗнч(x) = Тип("Строка"), то x: Строка
    pub fn narrow_type(
        &self,
        current: &TypeResolution,
        type_check: &str,
    ) -> TypeResolution {
        // TODO: Implement proper type narrowing
        // Сейчас просто возвращаем новый тип
        if let Some(raw_type) = self.repository.find_type(type_check) {
            return self.create_resolution_from_raw(&raw_type);
        }
        current.clone()
    }
}