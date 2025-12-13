//! Вывод типов выражений
//!
//! Модуль содержит методы для вывода типов выражений из AST.
//! Включает как простой строковый вывод (`infer_expression_type`),
//! так и полный вывод с TypeResolution (`infer_type_resolution`).

use crate::parsing::bsl::ast::Expression;
use bsl_shared::domain::is_configuration_type_pattern;
use bsl_shared::domain::resolver::GenericStrategy;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{Certainty, ConcreteType, GenericType, PlatformType, ResolutionResult, TypeResolution};

use super::converter::AstToIrConverter;
use super::global_collections::{is_global_collection, lookup_global_collection};

impl AstToIrConverter {
    /// Резолвит конфигурационный тип через TypeResolver, если применимо
    ///
    /// Проверяет, является ли тип конфигурационным (Справочники.X, Документы.X, etc.)
    /// и резолвит его через TypeResolver для получения корректного active_facet.
    ///
    /// # Returns
    /// Some(TypeResolution) если тип конфигурационный и резолвер доступен, иначе None
    fn try_resolve_configuration_type(&self, type_name: &str) -> Option<TypeResolution> {
        if is_configuration_type_pattern(type_name) {
            if let Some(ref resolver) = self.resolver {
                return Some(resolver.resolve_expression_sync(type_name));
            }
        }
        None
    }

    /// Вывод типа выражения (простая эвристика)
    pub(crate) fn infer_expression_type(&self, expr: &Expression) -> String {
        match expr {
            Expression::Number { .. } => "Число".to_string(),
            Expression::String { .. } => "Строка".to_string(),
            Expression::Boolean { .. } => "Булево".to_string(),
            Expression::Date { .. } => "Дата".to_string(),
            Expression::Identifier { name, .. } => {
                // MILESTONE 3.11 Phase 2: tree-sitter может парсить "Справочники.X" как один Identifier
                // Проверяем и трансформируем в Manager facet
                if let Some(dot_pos) = name.find('.') {
                    let collection = &name[..dot_pos];
                    let object_name = &name[dot_pos + 1..];

                    // Используем lookup_global_collection для унифицированного поиска
                    if let Some(info) = lookup_global_collection(collection) {
                        return format!("{}.{}", info.item_manager_type, object_name);
                    }

                    // Не глобальная коллекция - поиск переменной
                    return self.lookup_variable_type(name)
                        .unwrap_or_else(|| name.clone());
                }

                // Поиск переменной в текущем scope
                self.lookup_variable_type(name)
                    .unwrap_or_else(|| name.clone())
            }
            Expression::New { type_name, .. } => type_name.clone(),
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let base_type = self.infer_expression_type(object);

                // MILESTONE 3.11 Phase 2: PropertyAccess для глобальных коллекций -> Manager facet
                // Используем lookup_global_collection для унифицированного поиска
                if let Some(info) = lookup_global_collection(&base_type) {
                    format!("{}.{}", info.item_manager_type, property)
                } else {
                    format!("{}.{}", base_type, property)
                }
            }
            Expression::Call { function, .. } => {
                // Тип результата вызова функции
                match function.as_ref() {
                    // 1. Метод объекта: object.Method()
                    Expression::PropertyAccess { object, property, .. } => {
                        // MILESTONE 5.7: Используем infer_type_resolution для корректной резолюции
                        // цепочек вызовов типа Ссылка.ТабличнаяЧасть.Метод()
                        // infer_type_resolution использует resolve_member_type, который корректно
                        // резолвит .Работы -> ТабличнаяЧасть<Работы>
                        let object_resolution = self.infer_type_resolution(object);
                        let result = self.resolve_method_return_type(&object_resolution, property);

                        if result.is_unknown() {
                            // MILESTONE 3.11 FIX: Метод не найден - возвращаем Dynamic
                            return "Dynamic".to_string();
                        }

                        result.type_name()
                    },

                    // 2. Глобальная функция: ТипЗнч()
                    Expression::Identifier { name: func_name, .. } => {
                        // SignatureIndex для платформенных функций
                        if let Some(sig) = self.signature_index.find_global_function(func_name) {
                            return sig.return_type.clone().unwrap_or_else(|| "Неопределено".to_string());
                        }

                        // Fallback: пользовательские функции из SymbolTable
                        // Phase 3: return_type теперь Option<TypeResolution>
                        if let Some(sig) = self.symbol_table.find_function(func_name) {
                            return sig
                                .return_type
                                .as_ref()
                                .map(|r| r.type_name())
                                .unwrap_or_else(|| "Dynamic".to_string());
                        }

                        "Dynamic".to_string()
                    },

                    _ => "Dynamic".to_string()
                }
            }
            _ => "Dynamic".to_string(),
        }
    }

    /// Поиск типа переменной в scope hierarchy
    pub(crate) fn lookup_variable_type(&self, name: &str) -> Option<String> {
        // Используем публичный API вместо прямого доступа к scopes
        self.symbol_table
            .lookup_variable_in_hierarchy(self.current_scope, name)
            .map(|(_, resolution)| resolution.type_name())
    }

    /// Phase 3: Вывод типа с полной информацией TypeResolution
    ///
    /// В отличие от `infer_expression_type()`, возвращает TypeResolution
    /// с Certainty и ResolutionSource для точного отслеживания происхождения типа.
    ///
    /// # Milestone 3.17: TypeResolver DI
    /// Использует TypeResolver для резолюции конфигурационных типов с корректным active_facet.
    /// Это критично для валидации методов фасетных типов (СправочникМенеджер.СоздатьЭлемент()).
    pub(crate) fn infer_type_resolution(&self, expr: &Expression) -> TypeResolution {
        match expr {
            // Примитивные литералы - высокая уверенность
            Expression::Number { .. } => TypeResolution::primitive("Число"),
            Expression::String { .. } => TypeResolution::primitive("Строка"),
            Expression::Boolean { .. } => TypeResolution::primitive("Булево"),
            Expression::Date { .. } => TypeResolution::primitive("Дата"),

            // Идентификаторы - поиск в SymbolTable
            Expression::Identifier { name, .. } => {
                // Проверяем на Неопределено/Null (парсятся как идентификаторы)
                let name_lower = name.to_lowercase();
                if name_lower == "неопределено" || name_lower == "undefined" {
                    return TypeResolution::primitive("Неопределено");
                }
                if name_lower == "null" {
                    return TypeResolution::primitive("Null");
                }

                // MILESTONE 5.3: Глобальные коллекции (Справочники, Документы и т.д.)
                // возвращают своё имя как тип для корректной работы с PropertyAccess
                // Используем is_global_collection для унифицированного поиска
                if is_global_collection(name).is_some() {
                    return TypeResolution::inferred(name, 1.0);
                }

                // Сначала ищем в SymbolTable
                if let Some(resolution) = self
                    .symbol_table
                    .get_variable_type(self.current_scope, name)
                {
                    // Milestone 3.17: Если active_facet отсутствует, но это конфигурационный тип,
                    // пробуем обогатить через TypeResolver
                    if resolution.active_facet.is_none() {
                        let type_name = resolution.type_name();
                        if is_configuration_type_pattern(&type_name) {
                            if let Some(ref resolver) = self.resolver {
                                let enriched = resolver.resolve_expression_sync(&type_name);
                                if enriched.active_facet.is_some() {
                                    return enriched;
                                }
                            }
                        }
                    }
                    return resolution.clone();
                }

                // Переменная не найдена в scope - возвращаем undeclared
                // MILESTONE 5.1: Это важно для диагностики необъявленных переменных
                TypeResolution::undeclared_variable(name)
            }

            // Новые конструкции (Новый Тип())
            Expression::New { type_name, .. } => {
                // Очищаем скобки если tree-sitter включил их
                let clean_type_name = type_name.trim().trim_end_matches("()").trim();
                TypeResolution::explicit(clean_type_name)
            }

            // Доступ к свойству (object.property) - критично для конфигурационных типов
            // MILESTONE 3.17: Используем TypeResolver для установки active_facet
            Expression::PropertyAccess { object, property, .. } => {
                let base = self.infer_type_resolution(object);

                // Phase 4: Если base - undeclared variable, пробрасываем эту информацию
                // Это позволяет детектировать `необъявленная.Свойство` как ошибку
                if let Some(var_name) = base.is_undeclared_variable() {
                    return TypeResolution::undeclared_variable(var_name);
                }

                // MILESTONE 3.18: Сначала пробуем resolve_member_type для получения типа свойства
                // Это критично для цепочки вызовов: Ссылка.ТабличнаяЧасть.Метод()
                // где .ТабличнаяЧасть должен вернуть ТабличнаяЧасть<X>, а не строку
                if !base.is_unknown() {
                    let member_type = self.resolve_member_type(&base, property);
                    if !member_type.is_unknown() {
                        tracing::info!(
                            "PropertyAccess via resolve_member_type: base='{}', property='{}' => '{}'",
                            base.type_name(), property, member_type.type_name()
                        );
                        return member_type;
                    }
                }

                let type_str = format!("{}.{}", base.type_name(), property);

                // Проверяем, является ли это конфигурационным типом (Справочники.X, Документы.X, etc.)
                let is_config = is_configuration_type_pattern(&type_str);
                let has_resolver = self.resolver.is_some();
                tracing::info!(
                    "PropertyAccess: type_str='{}', is_config={}, has_resolver={}",
                    type_str, is_config, has_resolver
                );

                if is_config {
                    // Используем TypeResolver для корректной резолюции с active_facet
                    if let Some(ref resolver) = self.resolver {
                        let resolution = resolver.resolve_expression_sync(&type_str);
                        tracing::info!(
                            "Resolver result: type='{}', active_facet={:?}",
                            resolution.type_name(), resolution.active_facet
                        );
                        return resolution;
                    }
                }

                // Fallback для обычных типов
                TypeResolution::inferred(&type_str, 0.7)
            }

            // Вызов функции/метода
            Expression::Call { function, .. } => {
                // Phase 4: Проверяем function expression на undeclared
                // Если это прямой вызов необъявленной переменной (например "необъявленная()"),
                // а не вызов метода на ней ("объект.Метод()"), возвращаем undeclared.
                // Случай "необъявленная.Метод()" обрабатывается рекурсивно через PropertyAccess,
                // который также возвращает undeclared для необъявленного base.
                let func_type = self.infer_type_resolution(function);
                if let Some(var_name) = func_type.is_undeclared_variable() {
                    return TypeResolution::undeclared_variable(var_name);
                }

                // Для вызовов методов используем resolve_method_return_type
                // который ищет метод в SignatureIndex и возвращает Known если найден
                if let Expression::PropertyAccess { object, property, .. } = function.as_ref() {
                    let object_type = self.infer_type_resolution(object);
                    let method_result = self.resolve_method_return_type(&object_type, property);

                    // Если метод найден в SignatureIndex - certainty будет Known или Inferred с >0 уверенностью
                    // Inferred(0.0) эквивалентен Unknown и должен быть пропущен
                    let is_meaningful = match method_result.certainty {
                        Certainty::Known => true,
                        Certainty::Inferred(c) if c > 0.0 => true,
                        _ => false,
                    };

                    if is_meaningful {
                        let type_name = method_result.type_name();
                        // Для конфигурационных типов - дополнительный резолвинг через TypeResolver
                        if let Some(resolved) = self.try_resolve_configuration_type(&type_name) {
                            return resolved;
                        }
                        return method_result;
                    }
                }

                // Fallback для глобальных функций или ненайденных методов
                let type_str = self.infer_expression_type(expr);

                // Milestone 3.X: Если результат вызова - конфигурационный тип,
                // используем TypeResolver для корректной резолюции с active_facet
                if let Some(resolved) = self.try_resolve_configuration_type(&type_str) {
                    return resolved;
                }

                TypeResolution::inferred(&type_str, 0.6)
            }

            // Бинарные/унарные операции
            Expression::Binary { .. } | Expression::Unary { .. } => {
                let type_str = self.infer_expression_type(expr);
                TypeResolution::inferred(&type_str, 0.8)
            }

            // Остальные случаи - используем fallback
            _ => {
                let type_str = self.infer_expression_type(expr);
                if type_str.is_empty() || type_str == "Unknown" || type_str == "Dynamic" {
                    TypeResolution::unknown()
                } else {
                    TypeResolution::inferred(&type_str, 0.5)
                }
            }
        }
    }

    /// Попытка вывести Generic тип из вызова метода коллекции
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// МассивСтрок = Новый Массив();      // Generic<Массив, [?]>
    /// МассивСтрок.Добавить("текст");     // -> Generic<Массив, ["Строка"]>
    /// ```
    pub(crate) fn try_infer_generic_from_method_call(
        &mut self,
        receiver: &str,
        method_name: &str,
        arguments: &[String],
    ) {
        use tracing::debug;

        // Получаем текущий тип receiver из SymbolTable
        let current_resolution = match self
            .symbol_table
            .get_variable_type(self.current_scope, receiver)
        {
            Some(resolution) => resolution,
            None => {
                debug!(
                    "try_infer_generic: переменная {} не найдена в scope",
                    receiver
                );
                return;
            }
        };

        // Проверяем, что это Generic тип
        let base_type = match &current_resolution.result {
            ResolutionResult::Generic(gen) => gen.base_type.clone(),
            _ => {
                debug!(
                    "try_infer_generic: {} не Generic тип, пропускаем inference",
                    receiver
                );
                return;
            }
        };

        // Проверяем, есть ли GenericInfo для базового типа
        let type_data = match self.repository.find_type(&base_type) {
            Some(data) => data,
            None => {
                debug!(
                    "try_infer_generic: тип {} не найден в TypeRepository",
                    base_type
                );
                return;
            }
        };

        let generic_info = match &type_data.generic_info {
            Some(info) => info,
            None => {
                debug!("try_infer_generic: тип {} не имеет GenericInfo", base_type);
                return;
            }
        };

        // Ищем метод в inference_methods
        for inference_method in &generic_info.inference_methods {
            if method_name != inference_method.method_name {
                continue;
            }

            debug!(
                "try_infer_generic: найден inference метод {}.{}",
                base_type, method_name
            );

            // Для каждого параметра, который определяет Generic тип
            for (i, &param_idx) in inference_method.param_indices.iter().enumerate() {
                if let Some(arg_type) = arguments.get(param_idx) {
                    // Получаем индекс Generic параметра (0 для T в Массив<T>, 0 и 1 для K,V в Соответствие<K,V>)
                    let type_param_idx = inference_method
                        .inferred_type_params
                        .get(i)
                        .copied()
                        .unwrap_or(0);

                    // Обновляем Generic параметр
                    let success = self.symbol_table.update_generic_param(
                        self.current_scope,
                        receiver,
                        type_param_idx,
                        arg_type.clone(),
                    );

                    if success {
                        debug!(
                            "Generic inference: {}.{}() -> type_param[{}] = {}",
                            receiver, method_name, type_param_idx, arg_type
                        );
                    } else {
                        debug!(
                            "Generic inference failed: не удалось обновить {} type_param[{}]",
                            receiver, type_param_idx
                        );
                    }
                }
            }
        }
    }

    /// Резолвит тип свойства для MemberAccess
    ///
    /// # Milestone 3.18: Интеграция TypeMetadataLookup
    ///
    /// Использует TypeMetadataLookup.get_properties() для корректного резолвинга
    /// свойств с учётом active_facet. Это критично для:
    /// - Object/Reference фасетов: платформенные + конфигурационные свойства + табличные части
    /// - Manager фасета: shows_properties() = false → пустой список
    ///
    /// # Стратегия поиска
    ///
    /// 1. Новая логика: TypeMetadataLookup.get_properties() с учётом active_facet
    /// 2. Legacy fallback: прямой поиск в TypeRepository (для типов без active_facet)
    ///
    /// # Arguments
    /// * `object_type` - Тип объекта (из infer_type_resolution)
    /// * `member_name` - Имя свойства
    ///
    /// # Returns
    /// TypeResolution для свойства или unknown() если свойство не найдено
    pub(crate) fn resolve_member_type(&self, object_type: &TypeResolution, member_name: &str) -> TypeResolution {
        let type_name = object_type.type_name();

        // Пустой или unknown тип - не можем резолвить
        if type_name.is_empty() || type_name == "?" {
            return TypeResolution::unknown();
        }

        let member_name_lower = member_name.to_lowercase();

        // НОВАЯ ЛОГИКА: Используем TypeMetadataLookup для получения свойств с учётом active_facet
        // Корректно обрабатывает:
        // - Manager facet: пустой список (shows_properties() = false)
        // - Object/Reference facet: платформенные + конфигурационные свойства + табличные части
        let properties = self.metadata_lookup.get_properties(object_type);

        for prop in &properties {
            if prop.name.to_lowercase() == member_name_lower
                && !prop.prop_type.is_empty() {
                    return TypeResolution::explicit(&prop.prop_type);
                }
        }

        // Обработка Generic типов: ТабличнаяЧасть<Работы> → базовый тип + параметры
        if type_name.contains('<') {
            if let Some((base_type, params)) = GenericStrategy::parse_syntax(&type_name) {
                // Создаём GenericType для получения методов
                let generic_type = GenericType {
                    base_type: base_type.to_string(),
                    type_params: vec![ConcreteType::Platform(PlatformType {
                        name: params.to_string(),
                    })],
                };

                // Получаем методы с подстановкой типовых параметров
                let methods = self.metadata_lookup.get_methods_for_generic(&generic_type);

                for method in &methods {
                    if method.name.to_lowercase() == member_name_lower {
                        if !method.return_type.is_empty() {
                            return TypeResolution::explicit(&method.return_type);
                        }
                        // Метод найден, но без return_type
                        return TypeResolution::explicit("?");
                    }
                }

                // Также ищем свойства в базовом типе (ТабличнаяЧасть без параметров)
                if let Some(type_data) = self.repository.find_type(base_type) {
                    for prop in &type_data.properties {
                        if prop.name.to_lowercase() == member_name_lower
                            && !prop.prop_type.is_empty() {
                                return TypeResolution::explicit(&prop.prop_type);
                            }
                    }
                }
            }
        }

        // LEGACY FALLBACK: Для типов без active_facet (платформенные типы)
        if properties.is_empty() {
            // 1. Ищем в TypeRepository напрямую
            if let Some(type_data) = self.repository.find_type(&type_name) {
                for prop in &type_data.properties {
                    if prop.name.to_lowercase() == member_name_lower
                        && !prop.prop_type.is_empty() {
                            return TypeResolution::explicit(&prop.prop_type);
                        }
                }
            }

            // 2. Для фасетных типов по имени (СправочникОбъект.Контрагенты)
            if let Some(base_type) = SignatureIndex::extract_base_facet_type(&type_name) {
                if let Some(type_data) = self.repository.find_type(base_type) {
                    for prop in &type_data.properties {
                        if prop.name.to_lowercase() == member_name_lower
                            && !prop.prop_type.is_empty() {
                                return TypeResolution::explicit(&prop.prop_type);
                            }
                    }
                }
            }
        }

        // Fallback: свойство не найдено
        TypeResolution::unknown()
    }

    /// Резолвит return type метода для FunctionCall
    ///
    /// # Milestone 5.6: Вычисление result_type для цепочек вызовов
    ///
    /// Использует SignatureIndex для поиска сигнатуры метода и вычисления
    /// возвращаемого типа с учётом:
    /// - Фасетных типов (подстановка имени объекта метаданных)
    /// - Generic типов (ТабличнаяЧасть<Работы>.Выгрузить() -> ТаблицаЗначений)
    ///
    /// # Arguments
    /// * `object_type` - Тип объекта (из result_type дочернего узла или infer_type_resolution)
    /// * `method_name` - Имя метода
    ///
    /// # Returns
    /// TypeResolution для возвращаемого типа или unknown() если метод не найден
    pub(crate) fn resolve_method_return_type(&self, object_type: &TypeResolution, method_name: &str) -> TypeResolution {
        let type_name = object_type.type_name();

        // Пустой или unknown тип - не можем резолвить
        if type_name.is_empty() || type_name == "?" {
            return TypeResolution::unknown();
        }

        // Убираем Generic параметры для поиска: "ТабличнаяЧасть<Работы>" -> "ТабличнаяЧасть"
        let clean_type = if let Some(idx) = type_name.find('<') {
            &type_name[..idx]
        } else {
            &type_name
        };

        // Для фасетных типов извлекаем базовый тип для поиска в SignatureIndex
        // "СправочникМенеджер.Контрагенты" -> "СправочникМенеджер"
        let search_type = if let Some(base_type) = SignatureIndex::extract_base_facet_type(clean_type) {
            base_type
        } else {
            clean_type
        };

        // Поиск метода в SignatureIndex
        if let Some(method) = self.signature_index.find_method(search_type, method_name) {
            if let Some(return_type) = &method.return_type {
                // Извлекаем имя объекта метаданных из исходного типа для подстановки
                // "СправочникМенеджер.Контрагенты" -> "Контрагенты"
                if let Some(metadata_name) = SignatureIndex::extract_metadata_name(&type_name) {
                    // Подставляем имя в return_type
                    // "СправочникСсылка" + "Контрагенты" -> "СправочникСсылка.Контрагенты"
                    let substituted = SignatureIndex::substitute_type_name(return_type, metadata_name);
                    return TypeResolution::explicit(&substituted);
                }

                // Fallback: возвращаем return_type как есть
                return TypeResolution::explicit(return_type);
            }

            // Метод найден, но без return_type
            return TypeResolution::explicit("Неопределено");
        }

        // Метод не найден - unknown
        TypeResolution::unknown()
    }
}
