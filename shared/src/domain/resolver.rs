//! Domain Layer: Type Resolver
//!
//! Чистая бизнес-логика разрешения типов без Application concerns

use std::sync::Arc;
use crate::domain::repository::TypeRepository;
use crate::domain::types::TypeResolution;
use super::signature_index::SignatureIndex;

/// Результат валидации вызова функции
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Вызов корректен, возвращает тип (если функция возвращает значение)
    Ok(Option<String>),
    /// Отсутствует обязательный параметр
    MissingRequiredParam { param_name: String, param_index: usize },
    /// Слишком много аргументов
    TooManyArgs { expected: usize, actual: usize },
    /// Несоответствие типов аргумента
    TypeMismatch {
        param_name: String,
        expected: String,
        actual: String
    },
    /// Функция/метод не найдены
    NotFound,
}

/// Результат резолвинга конструктора (Milestone 2.21)
#[derive(Debug, Clone, PartialEq)]
pub enum ConstructorResolution {
    /// Успешно резолвлен
    Resolved {
        /// Результирующий тип
        type_name: String,

        /// Facet результата (если есть)
        facet: Option<String>,

        /// Generic параметры (для коллекций)
        /// Массив<Число> → Some(vec!["Число"])
        generic_params: Option<Vec<String>>,

        /// Ошибки валидации параметров
        validation_errors: Vec<String>,
    },

    /// Конструктор не найден
    NotFound {
        type_name: String,
        hint: String,
    },

    /// Динамический конструктор с неизвестным типом
    Dynamic {
        /// Невозможно определить тип статически
        reason: String,
    },
}

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

    /// Разрешение доступа к членам конфигурации
    fn resolve_member_access(&self, base: &str, member: &str) -> TypeResolution {
        use crate::domain::types::{
            Certainty, ConcreteType, ConfigurationType, MetadataKind, ResolutionMetadata,
            ResolutionResult, ResolutionSource,
        };

        // ✅ УЛУЧШЕНИЕ: Поддержка вложенных точек (Документы.ЗаказНаряды.Работы)
        // Если member содержит точку, рекурсивно резолвим
        if member.contains('.') {
            // Пробуем склеить base.первая_часть_member и резолвить как тип
            if let Some((first_part, rest)) = member.split_once('.') {
                let potential_type = format!("{}.{}", base, first_part);

                // Проверяем, существует ли такой тип в репозитории
                if let Some(raw_type) = self.repository.find_type(&potential_type) {
                    // Тип найден — проверяем, не табличная ли часть
                    if let Some(tabular_section) = raw_type.tabular_sections.iter().find(|ts| ts.name == rest) {
                        return self.resolve_tabular_section_access(
                            potential_type,
                            tabular_section.clone(),
                        );
                    }
                    // Иначе пробуем рекурсивно
                    return self.resolve_member_access(&potential_type, rest);
                }
            }
        }

        // Проверяем, не является ли base конфигурационным типом с табличными частями
        if let Some(raw_type) = self.repository.find_type(base) {
            // ✅ НОВОЕ: Проверка табличных частей
            if let Some(tabular_section) = raw_type.tabular_sections.iter().find(|ts| ts.name == member) {
                return self.resolve_tabular_section_access(
                    base.to_string(),
                    tabular_section.clone(),
                );
            }
        }

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

        // ✅ ИСПРАВЛЕНИЕ: Проверяем наличие метаданных для честного certainty
        // Формируем имя типа для поиска в repository
        let type_name = format!("{}.{}", prefix, member);
        let has_metadata = self.repository.find_type(&type_name).is_some();

        // Определяем уровень уверенности:
        // - Known (100%) - тип найден в метаданных конфигурации
        // - Inferred (50%) - только синтаксис распарсили, метаданных нет
        let (certainty, source) = if has_metadata {
            (Certainty::Known, ResolutionSource::Static)
        } else {
            (Certainty::Inferred(0.5), ResolutionSource::Inferred)
        };

        TypeResolution {
            certainty,
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: member.to_string(),
                attributes: vec![],
                tabular_sections: vec![],
            })),
            source,
            metadata: ResolutionMetadata {
                file: Some(format!("{}:{}", prefix, member)),
                line: None,
                column: None,
                notes: vec![if has_metadata {
                    format!(
                        "Found {} type in metadata: {}.{}",
                        match kind {
                            MetadataKind::Catalog => "catalog",
                            MetadataKind::Document => "document",
                            MetadataKind::Enum => "enum",
                            MetadataKind::Register => "information register",
                            _ => "configuration object",
                        },
                        base,
                        member
                    )
                } else {
                    format!(
                        "Inferred {} type from syntax: {}.{} (metadata not available)",
                        match kind {
                            MetadataKind::Catalog => "catalog",
                            MetadataKind::Document => "document",
                            MetadataKind::Enum => "enum",
                            MetadataKind::Register => "information register",
                            _ => "configuration object",
                        },
                        base,
                        member
                    )
                }],
            },
            active_facet: Some(crate::domain::types::FacetKind::Manager),
            available_facets: vec![
                crate::domain::types::FacetKind::Manager,
                crate::domain::types::FacetKind::Object,
                crate::domain::types::FacetKind::Reference,
            ],
        }
    }

    /// Резолвит доступ к табличной части конфигурационного объекта
    ///
    /// # Пример
    /// ```bsl
    /// Документ = Документы.ЗаказНаряды.СоздатьДокумент();
    /// Работы = Документ.Работы;  // ← вызывает resolve_tabular_section_access()
    /// ```
    ///
    /// # Возвращает
    /// `TypeResolution` с `ResolutionResult::Generic`:
    /// - base_type: "ТабличнаяЧасть"
    /// - type_params: [ConcreteType::TabularRow(СтрокаРаботы)]
    fn resolve_tabular_section_access(
        &self,
        parent_type: String,
        tabular_section: crate::domain::types::RawTabularSectionData,
    ) -> TypeResolution {
        use crate::domain::types::{
            TabularRowType, GenericType, ConcreteType, Certainty,
            ResolutionSource, FacetKind, ResolutionResult, ResolutionMetadata,
        };

        tracing::debug!(
            "🔍 Резолюция табличной части: {}.{}",
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
            "  ✅ Создан TabularRowType: {} с {} атрибутами",
            row_type.get_full_name(),
            row_type.attributes.len()
        );

        // 2. Оборачиваем в Generic тип: ТабличнаяЧасть<СтрокаРаботы>
        let generic_type = GenericType {
            base_type: "ТабличнаяЧасть".to_string(),
            type_params: vec![ConcreteType::TabularRow(row_type)],
        };

        tracing::debug!(
            "  🎯 Создан Generic тип: ТабличнаяЧасть<{}>",
            tabular_section.name
        );

        // 3. Возвращаем резолюцию с высокой уверенностью
        TypeResolution {
            result: ResolutionResult::Generic(generic_type),
            certainty: Certainty::Known,  // 100% - данные из метаданных
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
            },
            active_facet: Some(FacetKind::Collection),  // Табличная часть - это коллекция
            available_facets: vec![FacetKind::Collection],
        }
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
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// let result = resolver.validate_call(
    ///     Some("Массив"),
    ///     "Добавить",
    ///     &["Строка".to_string()],
    ///     &signature_index,
    /// );
    /// assert_eq!(result, ValidationResult::Ok(None));
    /// ```
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
        let required_count = signature.params.iter()
            .filter(|p| !p.is_optional)
            .count();

        if arg_types.len() < required_count {
            // Найти первый отсутствующий обязательный параметр
            let missing_param = signature.params
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

        // 3. Проверить типы аргументов (базовая проверка)
        // TODO: добавить более сложную проверку совместимости типов позже

        // 4. Вернуть тип возврата
        ValidationResult::Ok(signature.return_type.clone())
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

    // ===== Milestone 2.3: Union Types Integration =====

    /// Разрешение Union типа из строки: "Строка | Число | Null"
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// # use std::sync::Arc;
    /// # use bsl_shared::domain::resolver::TypeResolver;
    /// # use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let resolver = TypeResolver::new(repo);
    /// let union = resolver.resolve_union("Строка | Число");
    /// // Возвращает: Union(String, Number) с нормализацией
    /// ```
    pub fn resolve_union(&self, union_str: &str) -> TypeResolution {
        use crate::domain::types::{ResolutionResult, WeightedType, Certainty, ResolutionSource};

        // Парсим Union: "Строка | Число | Null"
        let type_names: Vec<&str> = union_str
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if type_names.is_empty() {
            return TypeResolution::unknown();
        }

        // Если один тип — возвращаем его напрямую
        if type_names.len() == 1 {
            return self.resolve_expression_sync(type_names[0]);
        }

        // Разрешаем каждый тип и собираем в WeightedType
        let mut weighted_types = Vec::new();
        let weight_per_type = 1.0 / type_names.len() as f32;

        for type_name in type_names {
            let resolved = self.resolve_expression_sync(type_name);

            // Извлекаем ConcreteType из TypeResolution
            let concrete_type = match resolved.result {
                crate::domain::types::ResolutionResult::Concrete(ct) => ct,
                _ => continue, // Пропускаем Unknown/Dynamic и сложные типы (Union, Generic и т.д.)
            };

            weighted_types.push(WeightedType::with_weight(concrete_type, weight_per_type));
        }

        // Нормализация через ResolutionResult::normalize_union
        let normalized_result = ResolutionResult::normalize_union(weighted_types);

        TypeResolution {
            certainty: Certainty::Known,
            result: normalized_result,
            source: ResolutionSource::Static,
            metadata: crate::domain::types::ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!("Union type: {}", union_str)],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Проверка совместимости присваивания для Union типов
    ///
    /// Значение совместимо с Union, если оно совместимо с любым из типов Union
    pub fn is_assignable_to_union(
        &self,
        value: &TypeResolution,
        union_resolution: &TypeResolution,
    ) -> bool {
        use crate::domain::types::ResolutionResult;

        if let ResolutionResult::Union(union_types) = &union_resolution.result {
            // Проверяем, совместимо ли значение с любым из типов Union
            for weighted in union_types {
                let union_member = TypeResolution {
                    certainty: union_resolution.certainty,
                    result: ResolutionResult::Concrete(weighted.type_.clone()),
                    source: union_resolution.source,
                    metadata: union_resolution.metadata.clone(),
                    active_facet: union_resolution.active_facet,
                    available_facets: union_resolution.available_facets.clone(),
                };

                if self.is_assignment_compatible(value, &union_member) {
                    return true;
                }
            }
            false
        } else {
            // Не Union тип — используем обычную проверку
            self.is_assignment_compatible(value, union_resolution)
        }
    }

    /// Форматирование Union типа для отображения
    pub fn format_union_type(union_types: &[crate::domain::types::WeightedType]) -> String {
        union_types
            .iter()
            .map(|wt| {
                let type_name = match &wt.type_ {
                    crate::domain::types::ConcreteType::Primitive(p) => format!("{:?}", p),
                    crate::domain::types::ConcreteType::Platform(pt) => pt.name.clone(),
                    crate::domain::types::ConcreteType::Configuration(ct) => ct.name.clone(),
                    crate::domain::types::ConcreteType::Special(st) => format!("{:?}", st),
                    crate::domain::types::ConcreteType::GlobalFunction(gf) => gf.name.clone(),
                    crate::domain::types::ConcreteType::TabularRow(tr) => tr.get_full_name(),
                };

                // Если вес не равен 1.0, показываем его
                if (wt.weight - 1.0).abs() > 0.01 {
                    format!("{}({:.0}%)", type_name, wt.weight * 100.0)
                } else {
                    type_name
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    // ===== Milestone 2.3 Task 2: Intersection Types Integration =====

    /// Разрешение Intersection типа из строки: "TypeA & TypeB"
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// # use std::sync::Arc;
    /// # use bsl_shared::domain::resolver::TypeResolver;
    /// # use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let resolver = TypeResolver::new(repo);
    /// let intersection = resolver.resolve_intersection("СправочникОбъект & ИмеетКод");
    /// // Возвращает: Intersection(СправочникОбъект, ИмеетКод)
    /// ```
    pub fn resolve_intersection(&self, intersection_str: &str) -> TypeResolution {
        use crate::domain::types::{ResolutionResult, Certainty, ResolutionSource};

        // Парсим Intersection: "TypeA & TypeB"
        let type_names: Vec<&str> = intersection_str
            .split('&')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if type_names.is_empty() {
            return TypeResolution::unknown();
        }

        // Если один тип — возвращаем его напрямую
        if type_names.len() == 1 {
            return self.resolve_expression_sync(type_names[0]);
        }

        // Разрешаем каждый тип и собираем в ConcreteType
        let mut concrete_types = Vec::new();

        for type_name in type_names {
            let resolved = self.resolve_expression_sync(type_name);

            // Извлекаем ConcreteType из TypeResolution
            let concrete_type = match resolved.result {
                ResolutionResult::Concrete(ct) => ct,
                _ => continue, // Пропускаем Unknown/Dynamic и сложные типы
            };

            concrete_types.push(concrete_type);
        }

        // Если не удалось разрешить ни один тип
        if concrete_types.is_empty() {
            return TypeResolution::unknown();
        }

        // Создаем Intersection через normalize
        let normalized_result = ResolutionResult::intersection(concrete_types);

        TypeResolution {
            certainty: Certainty::Known,
            result: normalized_result,
            source: ResolutionSource::Static,
            metadata: crate::domain::types::ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!("Intersection type: {}", intersection_str)],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Проверка совместимости типов для Intersection
    ///
    /// Проверяет, можно ли объединить два типа в Intersection
    pub fn are_compatible_for_intersection(
        &self,
        type_a: &TypeResolution,
        type_b: &TypeResolution,
    ) -> bool {
        use crate::domain::types::{ResolutionResult, ConcreteType};

        match (&type_a.result, &type_b.result) {
            // Оба типа должны быть Concrete для проверки
            (ResolutionResult::Concrete(a), ResolutionResult::Concrete(b)) => {
                match (a, b) {
                    // Primitive типы несовместимы между собой
                    (ConcreteType::Primitive(_), ConcreteType::Primitive(_)) => {
                        // Разные примитивы несовместимы
                        format!("{:?}", a) == format!("{:?}", b)
                    }

                    // Platform типы совместимы, если не одинаковые
                    (ConcreteType::Platform(pa), ConcreteType::Platform(pb)) => {
                        pa.name != pb.name
                    }

                    // Configuration типы совместимы, если не одинаковые
                    (ConcreteType::Configuration(ca), ConcreteType::Configuration(cb)) => {
                        ca.name != cb.name
                    }

                    // Special типы (Null, Undefined) несовместимы с примитивами
                    (ConcreteType::Special(_), ConcreteType::Primitive(_))
                    | (ConcreteType::Primitive(_), ConcreteType::Special(_)) => false,

                    // Остальные комбинации совместимы
                    _ => true,
                }
            }

            // Dynamic совместим с любым типом
            (ResolutionResult::Dynamic, _) | (_, ResolutionResult::Dynamic) => true,

            // Остальные случаи несовместимы
            _ => false,
        }
    }

    /// Форматирование Intersection типа для отображения
    pub fn format_intersection_type(intersection_types: &[crate::domain::types::ConcreteType]) -> String {
        use crate::domain::types::ConcreteType;

        intersection_types
            .iter()
            .map(|ct| match ct {
                ConcreteType::Primitive(p) => format!("{:?}", p),
                ConcreteType::Platform(pt) => pt.name.clone(),
                ConcreteType::Configuration(ct) => ct.name.clone(),
                ConcreteType::Special(st) => format!("{:?}", st),
                ConcreteType::GlobalFunction(gf) => gf.name.clone(),
                ConcreteType::TabularRow(tr) => tr.get_full_name(),
            })
            .collect::<Vec<_>>()
            .join(" & ")
    }

    // ===== Milestone 2.3 Task 3: Generic Types Integration =====

    /// Разрешение Generic типа из строки: "Массив<Строка>", "Соответствие<Строка, Число>"
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// # use std::sync::Arc;
    /// # use bsl_shared::domain::resolver::TypeResolver;
    /// # use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let resolver = TypeResolver::new(repo);
    /// let generic = resolver.resolve_generic("Массив<Строка>");
    /// // Возвращает: Generic(Массив, [Строка])
    /// ```
    pub fn resolve_generic(&self, generic_str: &str) -> TypeResolution {
        use crate::domain::types::{ResolutionResult, GenericType, Certainty, ResolutionSource};

        // Парсим Generic: "Массив<Строка>" или "Соответствие<Строка, Число>"
        if let Some((base_type, params_str)) = self.parse_generic_syntax(generic_str) {
            // Парсим параметры типа
            let type_params: Vec<&str> = params_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if type_params.is_empty() {
                return TypeResolution::unknown();
            }

            // Разрешаем каждый параметр типа
            let mut concrete_params = Vec::new();
            for param in type_params {
                let resolved = self.resolve_expression_sync(param);

                let concrete_type = match resolved.result {
                    ResolutionResult::Concrete(ct) => ct,
                    _ => continue, // Пропускаем неразрешенные типы
                };

                concrete_params.push(concrete_type);
            }

            if concrete_params.is_empty() {
                return TypeResolution::unknown();
            }

            // Создаем GenericType
            let generic_type = GenericType {
                base_type: base_type.to_string(),
                type_params: concrete_params,
            };

            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Generic(generic_type),
                source: ResolutionSource::Static,
                metadata: crate::domain::types::ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![format!("Generic type: {}", generic_str)],
                },
                active_facet: None,
                available_facets: vec![],
            }
        } else {
            TypeResolution::unknown()
        }
    }

    /// Парсинг Generic синтаксиса: "Массив<Строка>" → ("Массив", "Строка")
    fn parse_generic_syntax<'a>(&self, expr: &'a str) -> Option<(&'a str, &'a str)> {
        let open_bracket = expr.find('<')?;
        let close_bracket = expr.rfind('>')?;

        if close_bracket <= open_bracket {
            return None;
        }

        let base_type = expr[..open_bracket].trim();
        let params = expr[open_bracket + 1..close_bracket].trim();

        if base_type.is_empty() || params.is_empty() {
            return None;
        }

        Some((base_type, params))
    }

    /// Форматирование Generic типа для отображения
    pub fn format_generic_type(generic: &crate::domain::types::GenericType) -> String {
        use crate::domain::types::ConcreteType;

        let params = generic
            .type_params
            .iter()
            .map(|ct| match ct {
                ConcreteType::Primitive(p) => format!("{:?}", p),
                ConcreteType::Platform(pt) => pt.name.clone(),
                ConcreteType::Configuration(ct) => ct.name.clone(),
                ConcreteType::Special(st) => format!("{:?}", st),
                ConcreteType::TabularRow(tr) => tr.get_full_name(),
                ConcreteType::GlobalFunction(gf) => gf.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("{}<{}>", generic.base_type, params)
    }

    // ===== Milestone 2.3 Task 4: Nullable Types Integration =====

    /// Разрешение Nullable типа из строки: "Строка?"
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// # use std::sync::Arc;
    /// # use bsl_shared::domain::resolver::TypeResolver;
    /// # use bsl_shared::domain::repository::InMemoryTypeRepository;
    /// let repo = Arc::new(InMemoryTypeRepository::new());
    /// let resolver = TypeResolver::new(repo);
    /// let nullable = resolver.resolve_nullable("Строка?");
    /// // Возвращает: Nullable(Строка)
    /// ```
    pub fn resolve_nullable(&self, nullable_str: &str) -> TypeResolution {
        use crate::domain::types::{ResolutionResult, Certainty, ResolutionSource};

        // Парсим Nullable: "Строка?"
        if let Some(base_type_str) = nullable_str.strip_suffix('?') {
            let base_type_str = base_type_str.trim();

            if base_type_str.is_empty() {
                return TypeResolution::unknown();
            }

            // Разрешаем базовый тип
            let base_resolved = self.resolve_expression_sync(base_type_str);

            let base_concrete = match base_resolved.result {
                ResolutionResult::Concrete(ct) => ct,
                _ => return TypeResolution::unknown(), // Nullable применим только к Concrete типам
            };

            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::nullable(base_concrete),
                source: ResolutionSource::Static,
                metadata: crate::domain::types::ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![format!("Nullable type: {}", nullable_str)],
                },
                active_facet: None,
                available_facets: vec![],
            }
        } else {
            TypeResolution::unknown()
        }
    }

    /// Type narrowing для Nullable - убрать null из типа после проверки
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// Перем x: Строка?;
    /// Если x <> Неопределено Тогда
    ///     // Здесь x: Строка (не nullable)
    ///     x.Длина; // Безопасно
    /// КонецЕсли;
    /// ```
    pub fn narrow_nullable(&self, nullable_resolution: &TypeResolution) -> TypeResolution {
        use crate::domain::types::{ResolutionResult, Certainty};

        match &nullable_resolution.result {
            ResolutionResult::Nullable(base_type) => {
                // Убираем Nullable обертку, возвращаем базовый тип
                TypeResolution {
                    certainty: Certainty::Known,
                    result: ResolutionResult::Concrete((**base_type).clone()),
                    source: nullable_resolution.source,
                    metadata: crate::domain::types::ResolutionMetadata {
                        file: nullable_resolution.metadata.file.clone(),
                        line: nullable_resolution.metadata.line,
                        column: nullable_resolution.metadata.column,
                        notes: vec!["Type narrowed from nullable".to_string()],
                    },
                    active_facet: nullable_resolution.active_facet,
                    available_facets: nullable_resolution.available_facets.clone(),
                }
            }
            _ => nullable_resolution.clone(), // Не nullable - возвращаем как есть
        }
    }

    /// Форматирование Nullable типа для отображения
    pub fn format_nullable_type(base_type: &crate::domain::types::ConcreteType) -> String {
        use crate::domain::types::ConcreteType;

        let type_name = match base_type {
            ConcreteType::Primitive(p) => format!("{:?}", p),
            ConcreteType::Platform(pt) => pt.name.clone(),
            ConcreteType::Configuration(ct) => ct.name.clone(),
            ConcreteType::Special(st) => format!("{:?}", st),
            ConcreteType::TabularRow(tr) => tr.get_full_name(),
            ConcreteType::GlobalFunction(gf) => gf.name.clone(),
        };

        format!("{}?", type_name)
    }

    // ===== Milestone 2.21: Constructor Resolution =====

    /// Резолвить конструктор
    ///
    /// # Параметры
    /// - `type_name` - имя типа для конструктора ("Массив", "ТаблицаЗначений", etc.)
    /// - `arg_types` - список типов аргументов конструктора
    /// - `signature_index` - индекс с сигнатурами конструкторов
    ///
    /// # Возвращает
    /// `ConstructorResolution` - результат резолвинга (Resolved, NotFound, Dynamic)
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// // Новый Массив() → Массив<?>
    /// // Новый Массив(10) → Массив<?>
    /// // Новый ТаблицаЗначений() → ТаблицаЗначений
    /// // Новый("СправочникСсылка.Номенклатура") → Dynamic
    /// ```
    pub fn resolve_constructor(
        &self,
        type_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ConstructorResolution {
        // 1. Проверка на динамический конструктор
        if type_name.is_empty() || type_name == "?" {
            return ConstructorResolution::Dynamic {
                reason: "Динамический конструктор через строку - тип определяется в runtime".to_string(),
            };
        }

        // 2. Поиск сигнатуры конструктора
        let constructor = match signature_index.find_constructor(type_name) {
            Some(c) => c,
            None => {
                return ConstructorResolution::NotFound {
                    type_name: type_name.to_string(),
                    hint: format!("Конструктор для типа '{}' не найден в SignatureIndex", type_name),
                };
            }
        };

        // 3. Валидация параметров
        let validation_errors = self.validate_constructor_params(
            &constructor.params,
            arg_types
        );

        // 4. Generic inference для коллекций
        let generic_params = if constructor.is_collection {
            self.infer_generic_params(
                type_name,
                arg_types,
                constructor.generic_params_count
            )
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
        expected_params: &[super::types::ParameterInfo],
        actual_arg_types: &[String],
    ) -> Vec<String> {
        let mut errors = Vec::new();

        // Проверка количества параметров
        let required_count = expected_params.iter()
            .filter(|p| !p.is_optional)
            .count();

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
        for (i, (param, arg_type)) in expected_params.iter()
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
    ///
    /// # Generic Inference для коллекций
    /// - Массив: T выводится из типов добавляемых элементов (позже через flow analysis)
    /// - Соответствие: K, V выводятся из использования
    /// - Пока возвращаем "?" для неизвестных типов
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
                // Generic тип будет выведен позже из .Добавить()
                Some(vec!["?".to_string()])
            }

            "ФиксированныйМассив" | "FixedArray" => {
                // Новый ФиксированныйМассив(ИсходныйМассив)
                // Копирует тип из исходного массива
                if !arg_types.is_empty() {
                    // Пытаемся извлечь generic из первого аргумента
                    // Если arg_types[0] = "Массив<Число>", извлекаем "Число"
                    let generic = self.extract_generic_from_type(&arg_types[0])
                        .unwrap_or_else(|| "?".to_string());
                    Some(vec![generic])
                } else {
                    Some(vec!["?".to_string()])
                }
            }

            "Соответствие" | "Map" => {
                // Соответствие<K, V>
                // Generic типы выводятся из использования .Вставить(ключ, значение)
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

    /// Извлечь generic тип из строки типа
    /// "Массив<Число>" → Some("Число")
    /// "Массив" → None
    fn extract_generic_from_type(&self, type_str: &str) -> Option<String> {
        if let Some(start) = type_str.find('<') {
            if let Some(end) = type_str.rfind('>') {
                if end > start {
                    return Some(type_str[start + 1..end].trim().to_string());
                }
            }
        }
        None
    }

    // ===== Direction 2: Generic Collections Inference - Integration =====

    /// Резолюция переменной с использованием SymbolTable контекста
    ///
    /// Используется для вывода Generic типов из flow-sensitive анализа.
    /// Метод получает TypeHint из SymbolTable (через Inline Scope Analysis)
    /// и преобразует его в TypeResolution для hover/diagnostics.
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// МассивСтрок = Новый Массив();
    /// МассивСтрок.Добавить("текст");  // ← SymbolTable содержит TypeHint::Generic { "Массив", ["Строка"], 1.0 }
    /// // resolve_variable_with_context("МассивСтрок", symbol_table, scope_id)
    /// // → TypeResolution::Generic(Массив<Строка>)
    /// ```
    pub fn resolve_variable_with_context(
        &self,
        var_name: &str,
        symbol_table: &crate::ir::SymbolTable,
        scope_id: crate::ir::ScopeId,
    ) -> TypeResolution {
        use crate::ir::TypeHint;

        // Ищем переменную в scope hierarchy
        if let Some(hint) = symbol_table.get_variable_type(scope_id, var_name) {
            match hint {
                TypeHint::Generic {
                    base_type,
                    type_params,
                    certainty,
                } => {
                    // Резолвим Generic тип из hint
                    return self.resolve_generic_from_hint(&base_type, &type_params, certainty);
                }
                TypeHint::Explicit(type_name) | TypeHint::Inferred(type_name) => {
                    // Обычная резолюция по имени типа
                    return self.resolve_expression_sync(&type_name);
                }
                TypeHint::Unknown => {
                    // Тип неизвестен
                    return TypeResolution::unknown();
                }
            }
        }

        // Переменная не найдена в SymbolTable
        TypeResolution::unknown()
    }

    /// Резолюция Generic типа из TypeHint
    ///
    /// Преобразует TypeHint::Generic (IR layer) в TypeResolution::Generic (Domain layer).
    /// Используется после flow-sensitive анализа для получения детальной информации о типе.
    ///
    /// # Параметры
    /// - `base_type` - базовый тип коллекции ("Массив", "Соответствие", etc.)
    /// - `type_params` - список типов параметров (["Строка"], ["Строка", "Число"])
    /// - `certainty` - уверенность в выведенных типах (0.0-1.0)
    ///
    /// # Возвращает
    /// TypeResolution с ResolutionResult::Generic и соответствующим Certainty
    fn resolve_generic_from_hint(
        &self,
        base_type: &str,
        type_params: &[String],
        certainty: f32,
    ) -> TypeResolution {
        use crate::domain::types::{
            Certainty, ConcreteType, GenericType, ResolutionResult, ResolutionSource,
        };

        // Конвертируем строки типов в ConcreteType
        let concrete_params: Vec<ConcreteType> = type_params
            .iter()
            .filter(|p| *p != "?") // Пропускаем неизвестные параметры
            .filter_map(|p| {
                // Резолвим каждый параметр в ConcreteType
                let resolved = self.resolve_expression_sync(p);
                match resolved.result {
                    ResolutionResult::Concrete(ct) => Some(ct),
                    _ => None, // Пропускаем не-Concrete типы (Unknown, Dynamic, etc.)
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
            source: ResolutionSource::Inferred, // Generic inference всегда из flow-sensitive анализа
            metadata: crate::domain::types::ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!(
                    "Generic type inferred from flow-sensitive analysis (certainty: {:.0}%)",
                    certainty * 100.0
                )],
            },
            active_facet: Some(crate::domain::types::FacetKind::Collection),
            available_facets: vec![crate::domain::types::FacetKind::Collection],
        }
    }
}
// Milestone 2.20: Function Signature Validation tests
#[cfg(test)]
mod resolver_validation_tests;

// Milestone 2.3: Union Types tests
#[cfg(test)]
mod resolver_union_tests;

// Milestone 2.3 Task 2: Intersection Types tests
#[cfg(test)]
mod resolver_intersection_tests;

// Milestone 2.3 Task 3: Generic Types tests
#[cfg(test)]
mod resolver_generic_tests;

// Milestone 2.3 Task 4: Nullable Types tests
#[cfg(test)]
mod resolver_nullable_tests;

// Generic Tabular Sections: Resolver tests
#[cfg(test)]
mod resolver_tabular_tests;

// Milestone 2.21: Constructor Resolution tests
#[cfg(test)]
mod resolver_constructor_tests;
