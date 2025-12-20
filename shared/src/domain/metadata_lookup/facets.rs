//! Логика работы с фасетами типов (Manager, Object, Reference, Selection, List).
//!
//! Фасетная система типов позволяет представлять один объект метаданных 1С
//! через несколько фасетов: Manager, Object, Reference, Selection, List.

use super::TypeMetadataLookup;
use crate::domain::types::{
    ConcreteType, FacetKind, MetadataKind, RawMethodData, RawPropertyData, ResolutionResult,
    TypeResolution,
};

impl TypeMetadataLookup {
    /// Определяет имя платформенного типа на основе вида метаданных и активного фасета
    ///
    /// # Mapping таблица:
    ///
    /// | MetadataKind | FacetKind  | Platform Type Name     |
    /// |-------------|------------|------------------------|
    /// | Document    | Manager    | ДокументМенеджер       |
    /// | Document    | Object     | ДокументОбъект         |
    /// | Document    | Reference  | ДокументСсылка         |
    /// | Document    | Selection  | ДокументВыборка        |
    /// | Document    | List       | ДокументСписок         |
    /// | Catalog     | Manager    | СправочникМенеджер     |
    /// | Catalog     | Object     | СправочникОбъект       |
    /// | Catalog     | Reference  | СправочникСсылка       |
    /// | Catalog     | Selection  | СправочникВыборка      |
    /// | Catalog     | List       | СправочникСписок       |
    ///
    /// # Возвращает
    ///
    /// * `Some(&'static str)` - имя платформенного типа для поддерживаемой комбинации
    /// * `None` - для неподдерживаемых комбинаций (Enums, Registers пока не реализованы)
    ///
    pub(crate) fn get_platform_facet_type(
        kind: MetadataKind,
        facet: FacetKind,
    ) -> Option<&'static str> {
        use FacetKind::*;
        use MetadataKind::*;

        // ВАЖНО: имена типов должны содержать placeholder как в Syntax Helper
        // Например: "СправочникМенеджер.<Имя справочника>" вместо "СправочникМенеджер"
        match (kind, facet) {
            // Documents mapping
            (Document, Manager) => Some("ДокументМенеджер.<Имя документа>"),
            (Document, Object) => Some("ДокументОбъект.<Имя документа>"),
            (Document, Reference) => Some("ДокументСсылка.<Имя документа>"),
            (Document, Selection) => Some("ДокументВыборка.<Имя документа>"),
            (Document, List) => Some("ДокументСписок.<Имя документа>"),

            // Catalogs mapping
            (Catalog, Manager) => Some("СправочникМенеджер.<Имя справочника>"),
            (Catalog, Object) => Some("СправочникОбъект.<Имя справочника>"),
            (Catalog, Reference) => Some("СправочникСсылка.<Имя справочника>"),
            (Catalog, Selection) => Some("СправочникВыборка.<Имя справочника>"),
            (Catalog, List) => Some("СправочникСписок.<Имя справочника>"),

            // Enums mapping
            (Enum, Manager) => Some("ПеречислениеМенеджер.<Имя перечисления>"),
            (Enum, Reference) => Some("ПеречислениеСсылка.<Имя перечисления>"),

            // Information Registers mapping
            (InformationRegister, Manager) => {
                Some("РегистрСведенийМенеджер.<Имя регистра сведений>")
            }
            (InformationRegister, Collection) => {
                Some("РегистрСведенийНаборЗаписей.<Имя регистра сведений>")
            }
            (InformationRegister, Selection) => {
                Some("РегистрСведенийВыборка.<Имя регистра сведений>")
            }

            // Accumulation Registers mapping
            (AccumulationRegister, Manager) => {
                Some("РегистрНакопленияМенеджер.<Имя регистра накопления>")
            }
            (AccumulationRegister, Collection) => {
                Some("РегистрНакопленияНаборЗаписей.<Имя регистра накопления>")
            }
            (AccumulationRegister, Selection) => {
                Some("РегистрНакопленияВыборка.<Имя регистра накопления>")
            }

            // Неподдерживаемые комбинации
            _ => None,
        }
    }

    /// Извлекает MetadataKind из TypeResolution
    ///
    /// # Возвращает
    ///
    /// * `Some(MetadataKind)` - для конфигурационных типов (Документы, Справочники)
    /// * `None` - для примитивных и других не-конфигурационных типов
    ///
    pub(crate) fn extract_metadata_kind(
        &self,
        resolution: &TypeResolution,
    ) -> Option<MetadataKind> {
        match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) => Some(cfg.kind),
            _ => None,
        }
    }

    /// Выполняет lazy lookup методов для конкретного фасета
    ///
    /// # Алгоритм
    ///
    /// 1. Извлекает MetadataKind из resolution
    /// 2. Определяет имя платформенного типа через mapping
    /// 3. Ищет платформенный тип в репозитории
    /// 4. Возвращает его методы
    ///
    /// # Edge cases
    ///
    /// - Если resolution не содержит ConfigurationType -> None
    /// - Если mapping не найден для комбинации -> None
    /// - Если платформенный тип не загружен -> None
    /// - Если методы пусты -> Some(vec![])
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// // Документы.ЗаказНаряды + Manager фасет
    /// let methods = lookup.get_facet_methods(&resolution, FacetKind::Manager);
    /// // -> Ищет "ДокументМенеджер" -> Возвращает 12 методов
    /// ```
    ///
    pub(crate) fn get_facet_methods(
        &self,
        resolution: &TypeResolution,
        facet: FacetKind,
    ) -> Option<Vec<RawMethodData>> {
        // 1. Извлекаем MetadataKind
        let metadata_kind = self.extract_metadata_kind(resolution)?;

        // 2. Получаем имя платформенного типа через mapping
        let platform_type_name = Self::get_platform_facet_type(metadata_kind, facet)?;

        // 3. ПРИОРИТЕТ: Сначала ищем в signature_index (обогащённые данные)
        let sig_methods = self
            .repository
            .get_methods_from_signature_index(platform_type_name);
        if !sig_methods.is_empty() {
            tracing::trace!(
                "get_facet_methods('{}') -> found {} methods in signature_index",
                platform_type_name,
                sig_methods.len()
            );
            return Some(
                sig_methods
                    .into_iter()
                    .map(Self::method_signature_to_raw)
                    .collect(),
            );
        }

        // Fallback: ищем в raw types
        // Сначала пробуем точное имя с placeholder ("ДокументМенеджер.<Имя документа>")
        if let Some(platform_type) = self.repository.find_type(platform_type_name) {
            tracing::trace!(
                "get_facet_methods('{}') -> fallback to raw types ({} methods)",
                platform_type_name,
                platform_type.methods.len()
            );
            return Some(platform_type.methods.clone());
        }

        // Если не найдено, пробуем извлечь базовый тип ("ДокументМенеджер")
        // Используем universal функцию для обработки как placeholder, так и конкретных форматов
        if let Some(base_type_name) =
            crate::domain::facet_utils::extract_base_facet_type_universal(platform_type_name)
        {
            // Сначала пробуем SignatureIndex с базовым типом
            let sig_methods = self
                .repository
                .get_methods_from_signature_index(base_type_name);
            if !sig_methods.is_empty() {
                tracing::trace!(
                    "get_facet_methods('{}') -> found {} methods via base type '{}' in signature_index",
                    platform_type_name,
                    sig_methods.len(),
                    base_type_name
                );
                return Some(
                    sig_methods
                        .into_iter()
                        .map(Self::method_signature_to_raw)
                        .collect(),
                );
            }

            // Затем fallback на raw types
            if let Some(platform_type) = self.repository.find_type(base_type_name) {
                tracing::trace!(
                    "get_facet_methods('{}') -> fallback to raw types via base type '{}' ({} methods)",
                    platform_type_name,
                    base_type_name,
                    platform_type.methods.len()
                );
                return Some(platform_type.methods.clone());
            }
        }

        // Тип не найден ни с placeholder, ни без него
        None
    }

    /// Выполняет lazy lookup свойств для конкретного фасета
    ///
    /// # Алгоритм
    ///
    /// 1. Проверяем shows_properties() для фасета
    /// 2. Получаем платформенные свойства через get_platform_facet_type()
    /// 3. Добавляем конфигурационные свойства (реквизиты) для Object/Reference
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// // Справочники.Контрагенты + Object фасет
    /// let props = lookup.get_facet_properties(&resolution, FacetKind::Object);
    /// // -> Ищет "СправочникОбъект" -> Возвращает платформенные + конфигурационные свойства
    ///
    /// // Справочники.Контрагенты + Manager фасет
    /// let props = lookup.get_facet_properties(&resolution, FacetKind::Manager);
    /// // -> Manager не показывает свойства -> Возвращает Some(vec![])
    /// ```
    pub(crate) fn get_facet_properties(
        &self,
        resolution: &TypeResolution,
        facet: FacetKind,
    ) -> Option<Vec<RawPropertyData>> {
        // 1. Проверяем, показывает ли фасет свойства
        if !facet.shows_properties() {
            if matches!(facet, FacetKind::Manager)
                && matches!(self.extract_metadata_kind(resolution), Some(MetadataKind::Enum))
            {
                if let Some(config_type) = self.get_raw_type(resolution) {
                    let enum_name = config_type
                        .name
                        .strip_prefix("Перечисления.")
                        .unwrap_or(config_type.name.as_str());
                    let enum_ref_type = format!("ПеречислениеСсылка.{}", enum_name);

                    let props = config_type
                        .enum_values
                        .iter()
                        .map(|value| RawPropertyData {
                            name: value.clone(),
                            prop_type: enum_ref_type.clone(),
                            is_readonly: true,
                        })
                        .collect();
                    return Some(props);
                }
            }

            return Some(vec![]); // Пустой список для Manager/Selection/List
        }

        let mut combined_properties = Vec::new();

        // 2. Получаем платформенные свойства через mapping
        if let Some(metadata_kind) = self.extract_metadata_kind(resolution) {
            if let Some(platform_type_name) = Self::get_platform_facet_type(metadata_kind, facet) {
                // Сначала пробуем точное имя с placeholder
                if let Some(platform_type) = self.repository.find_type(platform_type_name) {
                    combined_properties.extend(platform_type.properties.clone());
                }
                // Fallback на базовый тип (без placeholder)
                else if let Some(base_name) =
                    crate::domain::facet_utils::extract_base_facet_type_universal(
                        platform_type_name,
                    )
                {
                    if let Some(platform_type) = self.repository.find_type(base_name) {
                        combined_properties.extend(platform_type.properties.clone());
                    }
                }
            }
        }

        // 3. Добавляем конфигурационные свойства (реквизиты) для Object/Reference
        if matches!(facet, FacetKind::Object | FacetKind::Reference) {
            if let Some(config_type) = self.get_raw_type(resolution) {
                let is_readonly = facet.properties_are_readonly();
                for prop in config_type.properties.iter() {
                    // Избегаем дубликатов (платформенные имеют приоритет)
                    if !combined_properties.iter().any(|p| p.name == prop.name) {
                        let mut new_prop = prop.clone();
                        if is_readonly {
                            new_prop.is_readonly = true;
                        }
                        combined_properties.push(new_prop);
                    }
                }

                // 3b. Табличные части как свойства для Object/Reference
                // Примечание: реквизиты добавляются раньше (шаг 3a), поэтому при
                // конфликте имён реквизит имеет приоритет. В 1С такой конфликт
                // невозможен - конфигуратор запрещает одинаковые имена.
                for ts in config_type.tabular_sections.iter() {
                    if !combined_properties.iter().any(|p| p.name == ts.name) {
                        combined_properties.push(RawPropertyData {
                            name: ts.name.clone(),
                            prop_type: format!("ТабличнаяЧасть<{}>", ts.name),
                            // readonly: табличную часть нельзя заменить целиком
                            // (Док.ТЧ = Другая), но можно изменять содержимое
                            is_readonly: true,
                        });
                    }
                }
            }
        }

        Some(combined_properties)
    }
}
