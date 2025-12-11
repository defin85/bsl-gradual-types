//! Domain Layer: Type Resolver
//!
//! Чистая бизнес-логика разрешения типов без Application concerns

use super::helpers::is_type_compatible;
use super::member_resolution::MemberResolver;
use super::strategies::{GenericStrategy, IntersectionStrategy, NullableStrategy, UnionStrategy};
use crate::domain::repository::TypeRepository;
use crate::domain::types::TypeResolution;
use std::sync::Arc;

/// Чистый Domain resolver - только бизнес-логика типизации
pub struct TypeResolver {
    pub(crate) repository: Arc<dyn TypeRepository>,
}

impl TypeResolver {
    pub fn new(repository: Arc<dyn TypeRepository>) -> Self {
        Self { repository }
    }

    // Note: is_configuration_loaded() is handled in MemberResolver

    /// Синхронное разрешение выражения (чистая Domain логика)
    pub fn resolve_expression_sync(&self, expression: &str) -> TypeResolution {
        // 1. Прямой поиск в repository
        if let Some(raw_type) = self.repository.find_type(expression) {
            return self.create_resolution_from_raw(&raw_type);
        }

        // 2. Парсинг и разрешение составных имен (Справочники.Контрагенты)
        if let Some((base, member)) = MemberResolver::parse_member_access(expression) {
            return self.resolve_member_access(&base, &member);
        }

        // 3. Union types: "Строка | Число" (Milestone 2.3)
        if expression.contains('|') {
            return self.resolve_union(expression);
        }

        // 4. Intersection types: "TypeA & TypeB" (Milestone 2.3 Task 2)
        if expression.contains('&') {
            return self.resolve_intersection(expression);
        }

        // 5. Generic types: "Массив<Строка>" (Milestone 2.3 Task 3)
        if expression.contains('<') && expression.contains('>') {
            return self.resolve_generic(expression);
        }

        // 6. Nullable types: "Строка?" (Milestone 2.3 Task 4)
        if expression.ends_with('?') {
            return self.resolve_nullable(expression);
        }

        // 7. Fallback для примитивных типов (когда repository пуст)
        if let Some(primitive_type) = self.try_resolve_primitive(expression) {
            return TypeResolution::known(primitive_type);
        }

        TypeResolution::unknown()
    }

    /// Преобразование RawTypeData в TypeResolution (чистая логика)
    ///
    /// Для конфигурационных типов (Справочники.X, Документы.X) создаёт
    /// ConcreteType::Configuration с правильным MetadataKind для корректного
    /// lookup методов через get_facet_methods().
    pub(crate) fn create_resolution_from_raw(
        &self,
        raw_type: &crate::domain::types::RawTypeData,
    ) -> TypeResolution {
        use crate::domain::metadata_constants::get_base_type_info;
        use crate::domain::types::{ConfigurationType, ConcreteType, PlatformType};

        // Проверяем, является ли это конфигурационным типом (Справочники.X, Документы.X)
        if let Some(dot_pos) = raw_type.name.find('.') {
            let prefix = &raw_type.name[..dot_pos];
            let object_name = &raw_type.name[dot_pos + 1..];

            // Если prefix — коллекция метаданных, создаём ConfigurationType
            if let Some((metadata_kind, facet)) = get_base_type_info(prefix) {
                let mut resolution = TypeResolution::known(ConcreteType::Configuration(
                    ConfigurationType {
                        kind: metadata_kind,
                        name: object_name.to_string(),
                        facet: Some(facet),
                        attributes: vec![],
                        tabular_sections: vec![],
                    },
                ));
                resolution.available_facets = raw_type.facets.clone();
                resolution.active_facet = Some(facet);
                return resolution;
            }
        }

        // Fallback: обычный платформенный тип
        let mut resolution = TypeResolution::known(ConcreteType::Platform(PlatformType {
            name: raw_type.name.clone(),
        }));
        resolution.available_facets = raw_type.facets.clone();

        resolution
    }

    /// Попытка распознать примитивный тип по имени (fallback для пустого repository)
    fn try_resolve_primitive(&self, type_name: &str) -> Option<crate::domain::types::ConcreteType> {
        use crate::domain::types::{ConcreteType, PrimitiveType};

        match type_name {
            "Строка" | "String" => Some(ConcreteType::string()),
            "Число" | "Number" => Some(ConcreteType::number()),
            "Булево" | "Boolean" => Some(ConcreteType::boolean()),
            "Дата" | "Date" => Some(ConcreteType::Primitive(PrimitiveType::Date)),
            "Null" => Some(ConcreteType::null()),
            "Неопределено" | "Undefined" => Some(ConcreteType::undefined()),
            _ => None,
        }
    }

    // ===== Member Resolution =====

    /// Разрешение доступа к членам конфигурации
    fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
        let resolver = MemberResolver::new(&self.repository);
        resolver.resolve(base, member)
    }

    // ===== Milestone 2.3: Union Types Integration =====

    /// Разрешение Union типа из строки: "Строка | Число | Null"
    pub fn resolve_union(&self, union_str: &str) -> TypeResolution {
        UnionStrategy::resolve(union_str, self)
    }

    /// Проверка совместимости присваивания для Union типов
    pub fn is_assignable_to_union(
        &self,
        value: &TypeResolution,
        union_resolution: &TypeResolution,
    ) -> bool {
        UnionStrategy::is_assignable_to_union(value, union_resolution, self)
    }

    /// Форматирование Union типа для отображения
    pub fn format_union_type(union_types: &[crate::domain::types::WeightedType]) -> String {
        super::helpers::format_union_type(union_types)
    }

    // ===== Milestone 2.3 Task 2: Intersection Types Integration =====

    /// Разрешение Intersection типа из строки: "TypeA & TypeB"
    pub fn resolve_intersection(&self, intersection_str: &str) -> TypeResolution {
        IntersectionStrategy::resolve(intersection_str, self)
    }

    /// Проверка совместимости типов для Intersection
    pub fn are_compatible_for_intersection(
        &self,
        type_a: &TypeResolution,
        type_b: &TypeResolution,
    ) -> bool {
        IntersectionStrategy::are_compatible(type_a, type_b)
    }

    /// Форматирование Intersection типа для отображения
    pub fn format_intersection_type(
        intersection_types: &[crate::domain::types::ConcreteType],
    ) -> String {
        super::helpers::format_intersection_type(intersection_types)
    }

    // ===== Milestone 2.3 Task 3: Generic Types Integration =====

    /// Разрешение Generic типа из строки: "Массив<Строка>", "Соответствие<Строка, Число>"
    pub fn resolve_generic(&self, generic_str: &str) -> TypeResolution {
        GenericStrategy::resolve(generic_str, self)
    }

    /// Форматирование Generic типа для отображения
    pub fn format_generic_type(generic: &crate::domain::types::GenericType) -> String {
        super::helpers::format_generic_type(generic)
    }

    // ===== Milestone 2.3 Task 4: Nullable Types Integration =====

    /// Разрешение Nullable типа из строки: "Строка?"
    pub fn resolve_nullable(&self, nullable_str: &str) -> TypeResolution {
        NullableStrategy::resolve(nullable_str, self)
    }

    /// Форматирование Nullable типа для отображения
    pub fn format_nullable_type(base_type: &crate::domain::types::ConcreteType) -> String {
        super::helpers::format_nullable_type(base_type)
    }

    // ===== Helpers for submodules =====

    /// Проверка совместимости типов (helper для валидации)
    pub(crate) fn check_type_compatible(expected: &str, actual: &str) -> bool {
        is_type_compatible(expected, actual)
    }
}
