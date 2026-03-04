//! Fluent builder для добавления методов в SignatureIndex
//!
//! Упрощает создание MethodSignature через цепочку вызовов вместо verbose конструкторов.
//!
//! # Example
//! ```rust,no_run
//! use bsl_repository::signature_index::{MethodBuilder, SignatureIndex};
//! use bsl_types::TypeId;
//!
//! let type_id = TypeId::new("ТабличнаяЧасть");
//! let mut index = SignatureIndex::new();
//!
//! MethodBuilder::for_type(&type_id)
//!     .method("Добавить")
//!     .returns("СтрокаТабличнойЧасти")
//!     .add_to(&mut index);
//!
//! // С параметрами:
//! MethodBuilder::for_type(&type_id)
//!     .method("Выгрузить")
//!     .returns("ТаблицаЗначений")
//!     .param("СписокКолонок", "Строка").optional().desc("Список колонок")
//!     .param("ОтборСтрок", "Структура").optional().desc("Условия отбора")
//!     .add_to(&mut index);
//!
//! // Void метод:
//! MethodBuilder::for_type(&type_id)
//!     .method("Очистить")
//!     .void()
//!     .add_to(&mut index);
//! ```

use super::method::MethodSignature;
use super::types::SignatureSource;
use super::SignatureIndex;
use bsl_types::types::{FacetKind, ParameterInfo};
use bsl_types::{ContextRequirements, TypeId};

/// Builder для создания MethodSignature с fluent API
///
/// Параметры по умолчанию:
/// - source: Platform
/// - context: Universal
/// - return_facet: None
/// - params: required (не optional)
pub struct MethodBuilder {
    type_id: TypeId,
    method_name: String,
    return_type: Option<String>,
    params: Vec<ParameterInfo>,
    return_facet: Option<FacetKind>,
    context: ContextRequirements,
    source: SignatureSource,
}

/// Builder для параметра метода
///
/// Создается через `MethodBuilder::param()`.
/// По умолчанию параметр обязательный (required).
/// Возвращает MethodBuilder через методы настройки (ownership transfer pattern).
pub struct ParamBuilder {
    builder: MethodBuilder,
    param_index: usize,
}

impl MethodBuilder {
    /// Создать builder для указанного типа
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// let builder = MethodBuilder::for_type(&TypeId::new("Массив"));
    /// # let _ = builder;
    /// ```
    pub fn for_type(type_id: &TypeId) -> Self {
        Self {
            type_id: type_id.clone(),
            method_name: String::new(),
            return_type: None,
            params: Vec::new(),
            return_facet: None,
            context: ContextRequirements::Universal,
            source: SignatureSource::Platform,
        }
    }

    /// Установить имя метода
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// let builder = MethodBuilder::for_type(&TypeId::new("Массив"));
    /// builder.method("Добавить");
    /// ```
    pub fn method(mut self, name: &str) -> Self {
        self.method_name = name.to_string();
        self
    }

    /// Установить тип возвращаемого значения
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// let builder = MethodBuilder::for_type(&TypeId::new("Массив")).method("Добавить");
    /// builder.returns("ТаблицаЗначений");
    /// ```
    pub fn returns(mut self, type_name: &str) -> Self {
        self.return_type = Some(type_name.to_string());
        self
    }

    /// Пометить метод как void (без возвращаемого значения)
    ///
    /// Эквивалентно процедуре в 1С.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// let builder = MethodBuilder::for_type(&TypeId::new("Массив")).method("Очистить");
    /// builder.void();
    /// ```
    pub fn void(mut self) -> Self {
        self.return_type = None;
        self
    }

    /// Добавить параметр и вернуть ParamBuilder для настройки
    ///
    /// По умолчанию параметр обязательный (required).
    /// ParamBuilder возвращает MethodBuilder через ownership transfer.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// MethodBuilder::for_type(&TypeId::new("Массив"))
    ///     .method("Вставить")
    ///     .param("Индекс", "Число").required().desc("Индекс элемента");
    /// ```
    pub fn param(mut self, name: &str, type_name: &str) -> ParamBuilder {
        let param = ParameterInfo {
            name: name.to_string(),
            type_name: Some(type_name.to_string()),
            is_optional: false, // По умолчанию required
            default_value: None,
            description: None,
        };
        self.params.push(param);
        let index = self.params.len() - 1;
        ParamBuilder {
            builder: self,
            param_index: index,
        }
    }

    /// Добавить параметр без типа (произвольный тип)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// MethodBuilder::for_type(&TypeId::new("Массив"))
    ///     .method("Добавить")
    ///     .param_any("Значение").required().desc("Любое значение");
    /// ```
    pub fn param_any(mut self, name: &str) -> ParamBuilder {
        let param = ParameterInfo {
            name: name.to_string(),
            type_name: None, // Произвольный тип
            is_optional: false,
            default_value: None,
            description: None,
        };
        self.params.push(param);
        let index = self.params.len() - 1;
        ParamBuilder {
            builder: self,
            param_index: index,
        }
    }

    /// Установить facet возвращаемого типа
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// # use bsl_types::types::FacetKind;
    /// let builder = MethodBuilder::for_type(&TypeId::new("СправочникМенеджер"))
    ///     .method("СоздатьЭлемент");
    /// builder.facet(FacetKind::Object);
    /// ```
    pub fn facet(mut self, facet: FacetKind) -> Self {
        self.return_facet = Some(facet);
        self
    }

    /// Установить требования к контексту выполнения
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::ContextRequirements;
    /// # use bsl_types::TypeId;
    /// let builder = MethodBuilder::for_type(&TypeId::new("Объект"))
    ///     .method("Метод");
    /// builder.context(ContextRequirements::ServerOnly);
    /// ```
    pub fn context(mut self, context: ContextRequirements) -> Self {
        self.context = context;
        self
    }

    /// Установить источник сигнатуры
    ///
    /// По умолчанию: Platform
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::{MethodBuilder, SignatureSource};
    /// # use bsl_types::TypeId;
    /// let builder = MethodBuilder::for_type(&TypeId::new("Объект"))
    ///     .method("Метод");
    /// builder.source(SignatureSource::Configuration);
    /// ```
    pub fn source(mut self, source: SignatureSource) -> Self {
        self.source = source;
        self
    }

    /// Добавить метод в SignatureIndex
    ///
    /// Финальный метод цепочки. Создает MethodSignature и добавляет в индекс.
    ///
    /// # Panics
    /// Если имя метода не установлено (пустая строка).
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::{MethodBuilder, SignatureIndex};
    /// # use bsl_types::TypeId;
    /// # let type_id = TypeId::new("ТабличнаяЧасть");
    /// # let mut index = SignatureIndex::new();
    /// MethodBuilder::for_type(&type_id)
    ///     .method("Добавить")
    ///     .returns("СтрокаТабличнойЧасти")
    ///     .add_to(&mut index);
    /// ```
    pub fn add_to(self, index: &mut SignatureIndex) {
        assert!(
            !self.method_name.is_empty(),
            "Method name must be set before calling add_to()"
        );

        let signature = MethodSignature::new(
            self.method_name,
            Some(self.type_id.to_string()),
            self.params,
            self.return_type,
            None,
            None,
            self.source,
            self.return_facet,
            self.context,
        );

        index.add_platform_method(self.type_id, signature);
    }

    /// Построить MethodSignature без добавления в индекс
    ///
    /// Полезно для тестирования или создания standalone сигнатур.
    ///
    /// # Panics
    /// Если имя метода не установлено (пустая строка).
    pub fn build(self) -> (TypeId, MethodSignature) {
        assert!(
            !self.method_name.is_empty(),
            "Method name must be set before calling build()"
        );

        let signature = MethodSignature::new(
            self.method_name,
            Some(self.type_id.to_string()),
            self.params,
            self.return_type,
            None,
            None,
            self.source,
            self.return_facet,
            self.context,
        );

        (self.type_id, signature)
    }
}

impl ParamBuilder {
    /// Пометить параметр как опциональный
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// MethodBuilder::for_type(&TypeId::new("ТаблицаЗначений"))
    ///     .method("Выгрузить")
    ///     .param("Колонки", "Строка").optional();
    /// ```
    pub fn optional(mut self) -> Self {
        self.builder.params[self.param_index].is_optional = true;
        self
    }

    /// Пометить параметр как обязательный (по умолчанию)
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// MethodBuilder::for_type(&TypeId::new("Массив"))
    ///     .method("Добавить")
    ///     .param("Значение", "Строка").required();
    /// ```
    pub fn required(mut self) -> Self {
        self.builder.params[self.param_index].is_optional = false;
        self
    }

    /// Установить описание параметра
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// MethodBuilder::for_type(&TypeId::new("Массив"))
    ///     .method("Вставить")
    ///     .param("Индекс", "Число").desc("Индекс элемента");
    /// ```
    pub fn desc(mut self, description: &str) -> Self {
        self.builder.params[self.param_index].description = Some(description.to_string());
        self
    }

    /// Установить значение по умолчанию
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bsl_repository::signature_index::MethodBuilder;
    /// # use bsl_types::TypeId;
    /// MethodBuilder::for_type(&TypeId::new("Массив"))
    ///     .method("УстановитьРазмер")
    ///     .param("Размер", "Число").default_value("0");
    /// ```
    pub fn default_value(mut self, value: &str) -> Self {
        self.builder.params[self.param_index].default_value = Some(value.to_string());
        self
    }

    // ==================== Forwarding методы к MethodBuilder ====================

    /// Добавить ещё один параметр
    pub fn param(self, name: &str, type_name: &str) -> ParamBuilder {
        self.builder.param(name, type_name)
    }

    /// Добавить параметр без типа (произвольный тип)
    pub fn param_any(self, name: &str) -> ParamBuilder {
        self.builder.param_any(name)
    }

    /// Установить facet возвращаемого типа
    pub fn facet(self, facet: FacetKind) -> MethodBuilder {
        self.builder.facet(facet)
    }

    /// Установить требования к контексту выполнения
    pub fn context(self, context: ContextRequirements) -> MethodBuilder {
        self.builder.context(context)
    }

    /// Установить источник сигнатуры
    pub fn source(self, source: SignatureSource) -> MethodBuilder {
        self.builder.source(source)
    }

    /// Добавить метод в SignatureIndex (финальный метод)
    pub fn add_to(self, index: &mut SignatureIndex) {
        self.builder.add_to(index);
    }

    /// Построить MethodSignature без добавления в индекс
    pub fn build(self) -> (TypeId, MethodSignature) {
        self.builder.build()
    }
}

#[cfg(test)]
#[path = "method_builder/tests.rs"]
mod tests;
