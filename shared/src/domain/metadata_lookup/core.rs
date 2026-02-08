//! Основные методы TypeMetadataLookup для получения данных типов.

use super::TypeMetadataLookup;
use crate::domain::resolver::GenericStrategy;
use crate::domain::signature_index::MethodSignature;
use crate::domain::types::{
    ConcreteType, FacetKind, GenericType, MetadataKind, PlatformType, RawMethodData, RawParamData,
    RawPropertyData, RawTabularSectionData, RawTypeData, ResolutionResult, TypeResolution,
    FORM_DATA_FORM_TYPE_NOTE_PREFIX, FORM_DATA_SEMANTICS_NOTE,
};

impl TypeMetadataLookup {
    /// Получить полную RawTypeData для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// `Some(RawTypeData)` если тип найден в repository, иначе `None`
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// # let lookup: TypeMetadataLookup = todo!();
    /// let resolution = TypeResolution::unknown();
    /// if let Some(raw_type) = lookup.get_raw_type(&resolution) {
    ///     println!("Категория: {}", raw_type.category);
    ///     println!("Методов: {}", raw_type.methods.len());
    /// }
    /// ```
    pub fn get_raw_type(&self, resolution: &TypeResolution) -> Option<RawTypeData> {
        let type_name = self.extract_type_name(resolution)?;
        self.repository.find_type(&type_name)
    }

    /// Получить методы для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Вектор методов типа или пустой вектор если тип не найден
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// # let lookup: TypeMetadataLookup = todo!();
    /// let resolution = TypeResolution::unknown();
    /// let methods = lookup.get_methods(&resolution);
    /// for method in methods {
    ///     println!("Метод: {} -> {}", method.name, method.return_type);
    /// }
    /// ```
    pub fn get_methods(&self, resolution: &TypeResolution) -> Vec<RawMethodData> {
        // Специальная обработка для Generic типов (СОХРАНИТЬ существующую логику!)
        if let ResolutionResult::Generic(generic_type) = &resolution.result {
            return self.get_methods_for_generic(generic_type);
        }

        // Парсинг generic из type_name() для случаев когда result не Generic
        // Это происходит когда тип создан через explicit("ТабличнаяЧасть<Работы>")
        let type_name = resolution.type_name();
        if type_name.contains('<') {
            if let Some((base_type, params)) = GenericStrategy::parse_syntax(&type_name) {
                let generic_type = GenericType {
                    base_type: base_type.to_string(),
                    type_params: vec![ConcreteType::Platform(PlatformType {
                        name: params.to_string(),
                    })],
                };
                tracing::debug!(
                    "get_methods: parsed generic from string '{}' -> base='{}', param='{}'",
                    type_name,
                    base_type,
                    params
                );
                return self.get_methods_for_generic(&generic_type);
            }
        }

        // Приоритет 1 - Lazy lookup через active_facet (для конфигурационных типов)
        if let Some(facet) = resolution.active_facet {
            if let Some(facet_methods) = self.get_facet_methods(resolution, facet) {
                return facet_methods;
            }
        }

        let sig_methods_to_raw =
            |type_name: &str, sig_methods: Vec<MethodSignature>| -> Vec<RawMethodData> {
                // SignatureIndex не хранит english_name, поэтому подтягиваем его из RawTypeData (если есть).
                let english_map = self
                    .repository
                    .find_type(type_name)
                    .map(|t| {
                        t.methods
                            .into_iter()
                            .filter(|m| !m.english_name.is_empty())
                            .map(|m| (m.name.to_lowercase(), m.english_name))
                            .collect::<std::collections::HashMap<_, _>>()
                    })
                    .unwrap_or_default();

                sig_methods
                    .into_iter()
                    .map(|sig| {
                        let mut raw = Self::method_signature_to_raw(sig);
                        if raw.english_name.is_empty() {
                            if let Some(en) = english_map.get(&raw.name.to_lowercase()) {
                                raw.english_name = en.clone();
                            }
                        }
                        raw
                    })
                    .collect()
            };

        // Приоритет 2 - Нормализованное имя типа через SignatureIndex
        if let Some(name) = self.normalize_type_name(resolution) {
            let sig_methods = self.repository.get_methods_from_signature_index(&name);
            if !sig_methods.is_empty() {
                tracing::trace!(
                    "get_methods('{}') -> found {} methods in signature_index",
                    name,
                    sig_methods.len()
                );
                return sig_methods_to_raw(&name, sig_methods);
            }
        }

        // Приоритет 3 - Извлекаем имя типа для fallback поиска
        if let Some(name) = self.extract_type_name(resolution) {
            // Сначала пробуем SignatureIndex с извлечённым именем
            let sig_methods = self.repository.get_methods_from_signature_index(&name);
            if !sig_methods.is_empty() {
                tracing::trace!(
                    "get_methods('{}') -> found {} methods in signature_index (extracted name)",
                    name,
                    sig_methods.len()
                );
                return sig_methods_to_raw(&name, sig_methods);
            }

            // Fallback: для типов не в SignatureIndex (примитивные, тестовые)
            if let Some(raw) = self.repository.find_type(&name) {
                tracing::trace!(
                    "get_methods('{}') -> fallback to raw types ({} methods)",
                    name,
                    raw.methods.len()
                );
                return raw.methods.clone();
            }

            // Fallback для фасетных типов: извлекаем базовый тип
            // Используем universal функцию для обработки как placeholder, так и конкретных форматов
            if let Some(base_type) =
                crate::domain::facet_utils::extract_base_facet_type_universal(&name)
            {
                // Сначала пробуем SignatureIndex
                let sig_methods = self.repository.get_methods_from_signature_index(base_type);
                if !sig_methods.is_empty() {
                    tracing::trace!(
                        "get_methods('{}') -> found {} methods via base type '{}' in signature_index",
                        name,
                        sig_methods.len(),
                        base_type
                    );
                    return sig_methods
                        .into_iter()
                        .map(Self::method_signature_to_raw)
                        .collect();
                }

                // Затем raw types
                if let Some(raw) = self.repository.find_type(base_type) {
                    tracing::trace!(
                        "get_methods('{}') -> fallback via base type '{}' ({} methods)",
                        name,
                        base_type,
                        raw.methods.len()
                    );
                    return raw.methods.clone();
                }
            }
        }

        vec![]
    }

    /// Найти сигнатуру метода/функции для вызова (глобальной или объектной)
    pub fn find_method_signature_for_call(
        &self,
        owner_type: Option<&TypeResolution>,
        method_name: &str,
    ) -> Option<MethodSignature> {
        match owner_type {
            Some(resolution) => {
                let owner_name = self
                    .normalize_type_name(resolution)
                    .unwrap_or_else(|| resolution.type_name());
                if owner_name.is_empty() {
                    None
                } else {
                    self.repository
                        .find_method_signature(Some(&owner_name), method_name)
                }
            }
            None => self.repository.find_method_signature(None, method_name),
        }
    }

    /// Получить свойства для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Вектор свойств типа или пустой вектор если тип не найден
    ///
    /// # Алгоритм
    ///
    /// 1. Приоритет 1: Lazy lookup через active_facet (для конфигурационных типов)
    /// 2. Приоритет 2: Fallback на raw type properties (для платформенных типов)
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// # let lookup: TypeMetadataLookup = todo!();
    /// let resolution = TypeResolution::unknown();
    /// let properties = lookup.get_properties(&resolution);
    /// for prop in properties {
    ///     println!("Свойство: {} ({})", prop.name, prop.prop_type);
    /// }
    /// ```
    pub fn get_properties(&self, resolution: &TypeResolution) -> Vec<RawPropertyData> {
        if let Some(enum_props) = self.get_enum_manager_properties(resolution) {
            return enum_props;
        }

        if let Some(form_data_props) = self.get_form_data_properties(resolution) {
            return form_data_props;
        }

        // Приоритет 1 - Lazy lookup через active_facet (для конфигурационных типов)
        if let Some(facet) = resolution.active_facet {
            if let Some(props) = self.get_facet_properties(resolution, facet) {
                return props;
            }
        }

        // Приоритет 2 - Fallback на raw type properties (для платформенных типов)
        self.get_raw_type(resolution)
            .map(|raw| raw.properties)
            .unwrap_or_default()
    }

    fn get_enum_manager_properties(
        &self,
        resolution: &TypeResolution,
    ) -> Option<Vec<RawPropertyData>> {
        let enum_name = match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg))
                if cfg.kind == MetadataKind::Enum
                    && matches!(resolution.active_facet, None | Some(FacetKind::Manager)) =>
            {
                cfg.name.as_str()
            }
            _ => return None,
        };
        let enum_ref_type = Self::get_platform_facet_type(MetadataKind::Enum, FacetKind::Reference)
            .map(|platform_type| {
                crate::domain::facet_utils::substitute_type_name(platform_type, enum_name)
            })?;
        let raw = self
            .repository
            .find_type(&format!("Перечисления.{}", enum_name))?;

        Some(
            raw.enum_values
                .iter()
                .map(|value| RawPropertyData {
                    name: value.clone(),
                    prop_type: enum_ref_type.clone(),
                    is_readonly: true,
                })
                .collect(),
        )
    }

    /// Получить табличные части для TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Вектор табличных частей типа или пустой вектор если тип не найден
    /// или не имеет табличных частей (платформенные типы)
    ///
    /// # Алгоритм
    ///
    /// 1. Извлекает имя типа из resolution
    /// 2. Находит RawTypeData в repository
    /// 3. Возвращает табличные части из RawTypeData
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// # let lookup: TypeMetadataLookup = todo!();
    /// let resolution = TypeResolution::unknown();
    /// let tabular_sections = lookup.get_tabular_sections(&resolution);
    /// for ts in tabular_sections {
    ///     println!("Табличная часть: {} ({} колонок)", ts.name, ts.attributes.len());
    /// }
    /// ```
    pub fn get_tabular_sections(&self, resolution: &TypeResolution) -> Vec<RawTabularSectionData> {
        // Табличные части актуальны только для Object/Reference фасетов
        if let Some(facet) = resolution.active_facet {
            if !matches!(facet, FacetKind::Object | FacetKind::Reference) {
                return vec![];
            }
        }

        // Получаем RawTypeData и извлекаем табличные части
        self.get_raw_type(resolution)
            .map(|raw| raw.tabular_sections.clone())
            .unwrap_or_default()
    }

    /// Проверить существование метода или свойства у типа
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    /// * `member_name` - имя метода или свойства для проверки
    ///
    /// # Возвращает
    ///
    /// `true` если метод/свойство существует, `false` если нет или тип не найден
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// # let lookup: TypeMetadataLookup = todo!();
    /// let resolution = TypeResolution::unknown();
    ///
    /// // Проверяем существующий метод
    /// assert!(lookup.has_member(&resolution, "Добавить"));
    ///
    /// // Проверяем несуществующий метод
    /// assert!(!lookup.has_member(&resolution, "НеСуществующийМетод"));
    /// ```
    ///
    /// # Использование для валидации
    ///
    /// ```rust,no_run
    /// # use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    /// # use bsl_shared::domain::types::TypeResolution;
    /// # use bsl_shared::domain::validators::TypeErrorKind;
    /// # let metadata_lookup: TypeMetadataLookup = todo!();
    /// # let resolution = TypeResolution::unknown();
    /// // В TypeValidator
    /// let maybe_error = if !metadata_lookup.has_member(&resolution, "Записать") {
    ///     Some(TypeErrorKind::NonExistentProperty {
    ///         object_type: format!("{:?}", resolution.result),
    ///         property_name: "Записать".to_string(),
    ///         variable_name: None,
    ///     })
    /// } else {
    ///     None
    /// };
    /// # let _ = maybe_error;
    /// ```
    pub fn has_member(&self, resolution: &TypeResolution, member_name: &str) -> bool {
        if self
            .get_methods(resolution)
            .iter()
            .any(|m| m.name == member_name || m.english_name == member_name)
        {
            return true;
        }

        if self
            .get_properties(resolution)
            .iter()
            .any(|p| p.name == member_name)
        {
            return true;
        }

        let Some(raw) = self.get_raw_type(resolution) else {
            return false;
        };

        // Проверяем значения перечисления
        if raw.enum_values.iter().any(|v| v == member_name) {
            return true;
        }

        if let Some(enum_name) = resolution.type_name().strip_prefix("ПеречислениеМенеджер.")
        {
            if let Some(raw_enum) = self
                .repository
                .find_type(&format!("Перечисления.{}", enum_name))
            {
                if raw_enum.enum_values.iter().any(|v| v == member_name) {
                    return true;
                }
            }
        }

        false
    }

    fn get_form_data_properties(
        &self,
        resolution: &TypeResolution,
    ) -> Option<Vec<RawPropertyData>> {
        if !Self::has_contextual_note(resolution, FORM_DATA_SEMANTICS_NOTE) {
            return None;
        }

        let mut merged: Vec<RawPropertyData> = Vec::new();
        let mut seen = std::collections::HashSet::<String>::new();
        let mut push_unique = |prop: RawPropertyData| {
            let key = prop.name.to_lowercase();
            if seen.insert(key) {
                merged.push(prop);
            }
        };

        // 1) form shape
        if let Some(form_type_name) =
            Self::contextual_note_value(resolution, FORM_DATA_FORM_TYPE_NOTE_PREFIX)
        {
            if let Some(form_type) = self.repository.find_type(form_type_name) {
                for prop in form_type.properties {
                    push_unique(prop);
                }
            }
        }

        // 2) guaranteed applied-object members
        if let Some(object_props) = self.get_facet_properties(resolution, FacetKind::Object) {
            for prop in object_props {
                push_unique(prop);
            }
        }

        // 3) applied facet fallback
        if let Some(facet) = resolution.active_facet {
            if let Some(fallback_props) = self.get_facet_properties(resolution, facet) {
                for prop in fallback_props {
                    push_unique(prop);
                }
            }
        } else if let Some(raw) = self.get_raw_type(resolution) {
            for prop in raw.properties {
                push_unique(prop);
            }
        }

        Some(merged)
    }

    fn has_contextual_note(resolution: &TypeResolution, note: &str) -> bool {
        resolution.metadata.notes.iter().any(|item| item == note)
    }

    fn contextual_note_value<'a>(resolution: &'a TypeResolution, prefix: &str) -> Option<&'a str> {
        resolution
            .metadata
            .notes
            .iter()
            .find_map(|note| note.strip_prefix(prefix))
    }

    /// Получить описание типа
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Описание типа или пустую строку если тип не найден
    pub fn get_description(&self, resolution: &TypeResolution) -> String {
        self.get_raw_type(resolution)
            .map(|raw| raw.description)
            .unwrap_or_default()
    }

    /// Получить категорию типа
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// Категорию типа или пустую строку если тип не найден
    pub fn get_category(&self, resolution: &TypeResolution) -> String {
        self.get_raw_type(resolution)
            .map(|raw| raw.category)
            .unwrap_or_default()
    }

    /// Извлечь имя типа из TypeResolution
    ///
    /// # Параметры
    ///
    /// * `resolution` - результат статического анализа типа
    ///
    /// # Возвращает
    ///
    /// `Some(String)` с именем типа, `None` если тип не поддерживается
    ///
    /// # Поддерживаемые типы
    ///
    /// - **Platform** типы: `Массив`, `ТаблицаЗначений`, `Строка`
    /// - **Configuration** типы: `Справочники.Контрагенты`, `Документы.Заказ`
    /// - **Primitive** и **Special** типы поддерживаются, если загружены из Syntax Helper
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn extract_type_name(&self, resolution: &TypeResolution) -> Option<String> {
        match &resolution.result {
            ResolutionResult::Concrete(concrete) => match concrete {
                ConcreteType::Platform(platform) => {
                    // Для платформенных типов используем имя напрямую
                    Some(platform.name.clone())
                }
                ConcreteType::Configuration(config) => {
                    // Если name уже содержит префикс (точку), возвращаем как есть
                    // Это предотвращает двойной префикс: "Документы.Документы.ЗаказНаряды"
                    if config.name.contains('.') {
                        Some(config.name.clone())
                    } else {
                        Some(format!("{}.{}", config.kind.to_prefix(), config.name))
                    }
                }
                ConcreteType::Primitive(prim) => Some(prim.display_name().to_string()),
                ConcreteType::Special(special) => Some(special.display_name().to_string()),
                // GlobalFunction может иметь документацию
                ConcreteType::GlobalFunction(func) => Some(func.name.clone()),
                ConcreteType::TabularRow(tr) => Some(tr.get_full_name()),
            },
            // Union и Dynamic типы не имеют прямого соответствия в RawTypeData
            ResolutionResult::Union(_) | ResolutionResult::Dynamic => None,
            // Intersection - берём первый тип
            ResolutionResult::Intersection(types) => types.first().and_then(|t| {
                self.extract_type_name(&TypeResolution {
                    result: ResolutionResult::Concrete(t.clone()),
                    ..resolution.clone()
                })
            }),
            // Generic - используем базовый тип
            ResolutionResult::Generic(gen) => Some(gen.base_type.clone()),
            // Nullable - распаковываем внутренний тип
            ResolutionResult::Nullable(inner) => self.extract_type_name(&TypeResolution {
                result: ResolutionResult::Concrete(inner.as_ref().clone()),
                ..resolution.clone()
            }),
        }
    }

    /// Нормализует имя типа для поиска в SignatureIndex
    ///
    /// Учитывает active_facet для построения имени платформенного типа.
    ///
    /// # Возвращает
    /// * `Some(String)` - нормализованное имя типа для SignatureIndex
    /// * `None` - если тип не поддерживается
    pub(crate) fn normalize_type_name(&self, resolution: &TypeResolution) -> Option<String> {
        // 1. Если есть active_facet -> строим platform facet type name
        if let Some(facet) = resolution.active_facet {
            if let Some(metadata_kind) = self.extract_metadata_kind(resolution) {
                if let Some(platform_name) = Self::get_platform_facet_type(metadata_kind, facet) {
                    return Some(platform_name.to_string());
                }
            }
        }

        // 2. Fallback на extract_type_name
        self.extract_type_name(resolution)
    }

    /// Конвертирует MethodSignature из signature_index в RawMethodData
    ///
    /// Это необходимо для обратной совместимости с существующим API,
    /// который возвращает Vec<RawMethodData>.
    pub(crate) fn method_signature_to_raw(sig: MethodSignature) -> RawMethodData {
        RawMethodData {
            name: sig.name,
            english_name: String::new(), // SignatureIndex не хранит english_name
            return_type: sig.return_type.unwrap_or_default(),
            params: sig
                .params
                .into_iter()
                .map(|p| RawParamData {
                    name: p.name,
                    param_type: p.type_name.unwrap_or_default(),
                    is_optional: p.is_optional,
                    default_value: p.default_value,
                })
                .collect(),
            description: sig.description,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: Some(sig.context_requirements),
            return_facet: sig.return_facet,
        }
    }
}
