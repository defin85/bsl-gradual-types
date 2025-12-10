//! Domain Layer: Type Resolver
//!
//! Чистая бизнес-логика разрешения типов без Application concerns

use super::helpers::is_type_compatible;
use super::member_resolution::MemberResolver;
use super::result_types::{ConstructorResolution, ValidationResult, ValidationResultV2};
use super::strategies::{GenericStrategy, IntersectionStrategy, NullableStrategy, UnionStrategy};
use crate::domain::signature_index::SignatureIndex;
use crate::domain::repository::TypeRepository;
use crate::domain::types::TypeResolution;
use std::sync::Arc;

/// Чистый Domain resolver - только бизнес-логика типизации
pub struct TypeResolver {
    repository: Arc<dyn TypeRepository>,
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
    fn create_resolution_from_raw(
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
        let mut resolution = TypeResolution::known(ConcreteType::Platform(
            PlatformType {
                name: raw_type.name.clone(),
            },
        ));
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

    // ===== Milestone 2.20: Function Signature Validation =====

    /// Валидирует вызов функции/метода
    ///
    /// # Параметры
    /// - `type_name` - имя типа для методов (None для глобальных функций)
    /// - `method_name` - имя метода или функции
    /// - `arg_types` - список типов аргументов в вызове
    /// - `signature_index` - индекс с сигнатурами методов и функций
    ///
    /// # Возвращает
    /// `ValidationResult` - результат валидации (Ok, MissingRequiredParam, TooManyArgs, TypeMismatch, NotFound)
    pub fn validate_call(
        &self,
        type_name: Option<&str>,
        method_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ValidationResult {
        // 1. Найти сигнатуру
        let signature = if let Some(type_name) = type_name {
            signature_index.find_method(type_name, method_name)
        } else {
            signature_index.find_global_function(method_name)
        };

        let signature = match signature {
            Some(sig) => sig,
            None => return ValidationResult::NotFound,
        };

        // 2. Проверить количество аргументов
        let required_count = signature.params.iter().filter(|p| !p.is_optional).count();

        if arg_types.len() < required_count {
            // Найти первый отсутствующий обязательный параметр
            let missing_param = signature
                .params
                .iter()
                .enumerate()
                .find(|(i, p)| !p.is_optional && *i >= arg_types.len())
                .unwrap();

            return ValidationResult::MissingRequiredParam {
                param_name: missing_param.1.name.clone(),
                param_index: missing_param.0,
            };
        }

        if arg_types.len() > signature.params.len() {
            return ValidationResult::TooManyArgs {
                expected: signature.params.len(),
                actual: arg_types.len(),
            };
        }

        // 3. Проверить типы аргументов
        for (param, arg_type) in signature.params.iter().zip(arg_types.iter()) {
            if let Some(expected_type) = &param.type_name {
                // Проверяем совместимость типов
                if !is_type_compatible(expected_type, arg_type) {
                    return ValidationResult::TypeMismatch {
                        param_name: param.name.clone(),
                        expected: expected_type.clone(),
                        actual: arg_type.clone(),
                    };
                }
            }
            // Если expected_type = None (Произвольный), то любой тип подходит
        }

        // 4. Вернуть тип возврата
        ValidationResult::Ok(signature.return_type.clone())
    }

    // ===== Assignment Compatibility =====

    /// Проверить совместимость присваивания типов (Domain логика)
    pub fn is_assignment_compatible(&self, from: &TypeResolution, to: &TypeResolution) -> bool {
        use crate::domain::types::{Certainty, ResolutionResult};

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
            // Milestone 2.3: Union type compatibility
            (_, ResolutionResult::Union(_)) => {
                // Присваивание в Union: проверяем совместимость с любым членом
                self.is_assignable_to_union(from, to)
            }
            (ResolutionResult::Union(union_types), ResolutionResult::Concrete(_)) => {
                // Присваивание из Union: все члены должны быть совместимы
                union_types.iter().all(|wt| {
                    let union_member = TypeResolution {
                        certainty: from.certainty,
                        result: ResolutionResult::Concrete(wt.type_.clone()),
                        source: from.source,
                        metadata: from.metadata.clone(),
                        active_facet: from.active_facet,
                        available_facets: from.available_facets.clone(),
                    };
                    self.is_assignment_compatible(&union_member, to)
                })
            }
            _ => false,
        }
    }

    /// Сужение типа на основе условия (flow-sensitive анализ)
    /// Например: Если ТипЗнч(x) = Тип("Строка"), то x: Строка
    ///
    /// Milestone 3.7: Интеграция с NarrowingEngine
    pub fn narrow_type(&self, current: &TypeResolution, type_check: &str) -> TypeResolution {
        use crate::analysis::type_guards::detect_type_guards;

        // Обнаруживаем type guards в условии
        let guards = detect_type_guards(type_check);

        if guards.is_empty() {
            // Fallback: пробуем найти тип напрямую
            if let Some(raw_type) = self.repository.find_type(type_check) {
                return self.create_resolution_from_raw(&raw_type);
            }
            return current.clone();
        }

        // Применяем первый найденный guard
        if let Some(guard) = guards.first() {
            guard.apply_narrowing(current)
        } else {
            current.clone()
        }
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

    /// Type narrowing для Nullable - убрать null из типа после проверки
    pub fn narrow_nullable(&self, nullable_resolution: &TypeResolution) -> TypeResolution {
        NullableStrategy::narrow(nullable_resolution)
    }

    /// Форматирование Nullable типа для отображения
    pub fn format_nullable_type(base_type: &crate::domain::types::ConcreteType) -> String {
        super::helpers::format_nullable_type(base_type)
    }

    // ===== Milestone 2.21: Constructor Resolution =====

    /// Резолвить конструктор
    pub fn resolve_constructor(
        &self,
        type_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ConstructorResolution {
        // 1. Проверка на динамический конструктор
        if type_name.is_empty() || type_name == "?" {
            return ConstructorResolution::Dynamic {
                reason: "Динамический конструктор через строку - тип определяется в runtime"
                    .to_string(),
            };
        }

        // 2. Поиск сигнатуры конструктора
        let constructor = match signature_index.find_constructor(type_name) {
            Some(c) => c,
            None => {
                return ConstructorResolution::NotFound {
                    type_name: type_name.to_string(),
                    hint: format!(
                        "Конструктор для типа '{}' не найден в SignatureIndex",
                        type_name
                    ),
                };
            }
        };

        // 3. Валидация параметров
        let validation_errors = self.validate_constructor_params(&constructor.params, arg_types);

        // 4. Generic inference для коллекций
        let generic_params = if constructor.is_collection {
            self.infer_generic_params(type_name, arg_types, constructor.generic_params_count)
        } else {
            None
        };

        // 5. Формирование результата
        ConstructorResolution::Resolved {
            type_name: type_name.to_string(),
            facet: constructor.facet.clone(),
            generic_params,
            validation_errors,
        }
    }

    /// Валидировать параметры конструктора
    fn validate_constructor_params(
        &self,
        expected_params: &[crate::domain::types::ParameterInfo],
        actual_arg_types: &[String],
    ) -> Vec<String> {
        let mut errors = Vec::new();

        // Проверка количества параметров
        let required_count = expected_params.iter().filter(|p| !p.is_optional).count();

        if actual_arg_types.len() < required_count {
            errors.push(format!(
                "Недостаточно аргументов: ожидается минимум {}, передано {}",
                required_count,
                actual_arg_types.len()
            ));
        }

        if actual_arg_types.len() > expected_params.len() {
            errors.push(format!(
                "Слишком много аргументов: ожидается максимум {}, передано {}",
                expected_params.len(),
                actual_arg_types.len()
            ));
        }

        // Проверка типов параметров
        for (i, (param, arg_type)) in expected_params
            .iter()
            .zip(actual_arg_types.iter())
            .enumerate()
        {
            if let Some(expected_type) = &param.type_name {
                // TODO: добавить более сложную проверку совместимости типов
                // Пока простая проверка на точное соответствие
                if expected_type != "Произвольный" && expected_type != arg_type {
                    errors.push(format!(
                        "Параметр {} '{}': ожидается тип {}, передан {}",
                        i + 1,
                        param.name,
                        expected_type,
                        arg_type
                    ));
                }
            }
        }

        errors
    }

    /// Вывести generic параметры для коллекций
    fn infer_generic_params(
        &self,
        type_name: &str,
        arg_types: &[String],
        generic_count: usize,
    ) -> Option<Vec<String>> {
        if generic_count == 0 {
            return None;
        }

        match type_name {
            "Массив" | "Array" => {
                // Массив может быть создан с начальным размером
                // Новый Массив(10) → Массив<?>
                Some(vec!["?".to_string()])
            }

            "ФиксированныйМассив" | "FixedArray" => {
                // Новый ФиксированныйМассив(ИсходныйМассив)
                if !arg_types.is_empty() {
                    let generic = GenericStrategy::extract_from_type(&arg_types[0])
                        .unwrap_or_else(|| "?".to_string());
                    Some(vec![generic])
                } else {
                    Some(vec!["?".to_string()])
                }
            }

            "Соответствие" | "Map" => {
                // Соответствие<K, V>
                Some(vec!["?".to_string(), "?".to_string()])
            }

            "СписокЗначений" | "ValueList" => {
                // СписокЗначений<T>
                Some(vec!["?".to_string()])
            }

            _ => {
                // Для неизвестных коллекций возвращаем "?" для каждого generic
                Some(vec!["?".to_string(); generic_count])
            }
        }
    }

    // ===== Direction 2: Generic Collections Inference - Integration =====

    /// Резолюция переменной с использованием SymbolTable контекста
    ///
    /// Используется для вывода Generic типов из flow-sensitive анализа.
    pub fn resolve_variable_with_context(
        &self,
        var_name: &str,
        symbol_table: &crate::ir::SymbolTable,
        scope_id: crate::ir::ScopeId,
    ) -> TypeResolution {
        use tracing::{debug, info};

        // Ищем переменную в scope hierarchy
        if let Some(resolution) = symbol_table.get_variable_type(scope_id, var_name) {
            info!(
                "resolve_variable_with_context('{}', scope={:?}): TypeResolution = {:?}",
                var_name, scope_id, resolution
            );

            use crate::domain::types::{Certainty, ResolutionResult};

            match (&resolution.certainty, &resolution.result) {
                (Certainty::Unknown, _) => {
                    debug!("  -> TypeResolution::unknown()");
                    return TypeResolution::unknown();
                }
                (_, ResolutionResult::Generic(gen)) => {
                    let type_params: Vec<String> = gen.type_params.iter()
                        .map(|ct| {
                            let temp = TypeResolution::known(ct.clone());
                            temp.type_name()
                        })
                        .collect();
                    let certainty = match resolution.certainty {
                        Certainty::Known => 1.0,
                        Certainty::Inferred(c) => c,
                        Certainty::Unknown => 0.0,
                    };
                    debug!("  -> Generic: base={}, params={:?}", gen.base_type, type_params);
                    return self.resolve_generic_from_hint(&gen.base_type, &type_params, certainty);
                }
                _ => {
                    let type_name = resolution.type_name();
                    info!("  -> Resolving type_name: '{}'", type_name);
                    let resolved = self.resolve_expression_sync(&type_name);
                    info!("  -> Resolution result: {:?}", resolved.result);
                    return resolved;
                }
            }
        }

        info!(
            "resolve_variable_with_context('{}', scope={:?}): NOT FOUND in SymbolTable",
            var_name, scope_id
        );
        TypeResolution::unknown()
    }

    /// Резолюция Generic типа из TypeResolution
    fn resolve_generic_from_hint(
        &self,
        base_type: &str,
        type_params: &[String],
        certainty: f32,
    ) -> TypeResolution {
        use crate::domain::types::{
            Certainty, ConcreteType, FacetKind, GenericType, ResolutionResult, ResolutionSource,
        };

        // Конвертируем строки типов в ConcreteType
        let concrete_params: Vec<ConcreteType> = type_params
            .iter()
            .filter(|p| *p != "?")
            .filter_map(|p| {
                let resolved = self.resolve_expression_sync(p);
                match resolved.result {
                    ResolutionResult::Concrete(ct) => Some(ct),
                    _ => None,
                }
            })
            .collect();

        // Если после фильтрации не осталось параметров — возвращаем базовый тип без Generic
        if concrete_params.is_empty() {
            return self.resolve_expression_sync(base_type);
        }

        // Создаём GenericType
        let generic_type = GenericType {
            base_type: base_type.to_string(),
            type_params: concrete_params,
        };

        // Определяем уровень certainty
        let certainty_level = if certainty > 0.9 {
            Certainty::Known
        } else if certainty > 0.5 {
            Certainty::Inferred(certainty)
        } else {
            Certainty::Inferred(0.5)
        };

        TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: certainty_level,
            source: ResolutionSource::Inferred,
            metadata: crate::domain::types::ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!(
                    "Generic type inferred from flow-sensitive analysis (certainty: {:.0}%)",
                    certainty * 100.0
                )],
                uncertainty_reason: None,
            },
            active_facet: Some(FacetKind::Collection),
            available_facets: vec![FacetKind::Collection],
        }
    }

    // ===== Milestone 3.13: Object-Based Type Comparison =====

    /// Объектное сравнение типов (v2 версия)
    pub fn is_type_compatible_v2(
        &self,
        expected: &str,
        actual: &str,
    ) -> crate::domain::types::TypeCompatibility {
        let expected_resolution = self.resolve_expression_sync(expected);
        let actual_resolution = self.resolve_expression_sync(actual);

        actual_resolution.is_compatible_with(&expected_resolution)
    }

    /// Валидация вызова с объектным сравнением типов (v2 версия)
    pub fn validate_call_v2(
        &self,
        type_name: Option<&str>,
        method_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ValidationResultV2 {
        // 1. Найти сигнатуру
        let signature = if let Some(type_name) = type_name {
            signature_index.find_method(type_name, method_name)
        } else {
            signature_index.find_global_function(method_name)
        };

        let signature = match signature {
            Some(sig) => sig,
            None => return ValidationResultV2::NotFound,
        };

        // 2. Проверка количества параметров
        let required_count = signature.params.iter().filter(|p| !p.is_optional).count();

        if arg_types.len() < required_count {
            return ValidationResultV2::MissingRequiredParam {
                param_name: signature.params[arg_types.len()].name.clone(),
                param_index: arg_types.len(),
            };
        }

        if arg_types.len() > signature.params.len() {
            return ValidationResultV2::TooManyArgs {
                expected: signature.params.len(),
                actual: arg_types.len(),
            };
        }

        // 3. Проверяем типы параметров с объектным сравнением
        for (i, (param, arg_type)) in signature.params.iter().zip(arg_types.iter()).enumerate() {
            if let Some(expected_type) = &param.type_name {
                let compat = self.is_type_compatible_v2(expected_type, arg_type);
                if !compat.is_compatible() {
                    return ValidationResultV2::TypeMismatch {
                        param_name: param.name.clone(),
                        param_index: i,
                        expected: expected_type.clone(),
                        actual: arg_type.clone(),
                        reason: compat.reason(),
                    };
                }
            }
        }

        // 4. Вернуть тип возврата с подстановкой имени объекта для фасетных типов
        let return_type = if let Some(ref rt) = signature.return_type {
            if let Some(type_name) = type_name {
                if let Some(metadata_name) = SignatureIndex::extract_metadata_name(type_name) {
                    Some(SignatureIndex::substitute_type_name(rt, metadata_name))
                } else {
                    Some(rt.clone())
                }
            } else {
                Some(rt.clone())
            }
        } else {
            None
        };
        ValidationResultV2::Ok(return_type)
    }
}
