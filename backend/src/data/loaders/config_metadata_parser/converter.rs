//! Конвертер UniversalMetadataObject -> RawTypeData для TypeRepository

use super::types::UniversalMetadataObject;
use bsl_shared::domain::types::{
    RawAttributeData, RawDataSource, RawTabularSectionData, RawTypeData,
};

impl UniversalMetadataObject {
    /// Конвертирует UniversalMetadataObject в RawTypeData для загрузки в TypeRepository
    ///
    /// Создаёт полное имя типа (например, "Справочники.Контрагенты")
    /// и заполняет все поля RawTypeData.
    ///
    /// # Параметры
    /// - `prefix` - Опциональный префикс расширения (например, "Тест_")
    ///              Применяется ТОЛЬКО к имени объекта метаданных.
    ///              Для основной конфигурации передавайте `None`.
    ///
    /// # Примеры
    /// ```ignore
    /// // Без префикса (основная конфигурация)
    /// obj.to_raw_type_data(None) // → "Справочники.Контрагенты"
    ///
    /// // С префиксом (расширение)
    /// obj.to_raw_type_data(Some("Тест_")) // → "Справочники.Тест_Контрагенты"
    /// ```
    pub fn to_raw_type_data(&self, prefix: Option<&str>) -> RawTypeData {
        let type_name = self.get_full_type_name(prefix);

        RawTypeData {
            name: type_name.clone(),
            english_name: Self::apply_prefix(prefix, &self.name), // Префикс и к english name
            description: self.synonym.clone().unwrap_or_default(),
            category: self.get_category(),
            source: RawDataSource::Configuration,
            methods: Vec::new(), // Методы извлекаются отдельно из модулей
            properties: self.convert_attributes_to_properties(),
            facets: self.facets.clone(),
            kind: self.object_type,
            attributes: self.convert_attributes(),
            tabular_sections: self.convert_tabular_sections(),
            enum_values: Vec::new(), // Для перечислений будет заполнено отдельно
            generic_info: None, // Конфигурационные типы не имеют Generic метаданных (пока)
        }
    }

    /// Применяет префикс к имени объекта
    ///
    /// # Примеры
    /// ```ignore
    /// apply_prefix(None, "Контрагенты") // → "Контрагенты"
    /// apply_prefix(Some(""), "Контрагенты") // → "Контрагенты"
    /// apply_prefix(Some("Тест_"), "Контрагенты") // → "Тест_Контрагенты"
    /// ```
    fn apply_prefix(prefix: Option<&str>, name: &str) -> String {
        match prefix {
            Some(p) if !p.is_empty() => format!("{}{}", p, name),
            _ => name.to_string(),
        }
    }

    /// Получить полное имя типа (Справочники.Контрагенты или Справочники.Тест_Контрагенты)
    fn get_full_type_name(&self, prefix: Option<&str>) -> String {
        let object_name = Self::apply_prefix(prefix, &self.name);

        if let Some(kind) = self.object_type {
            format!("{}.{}", kind.display_name(), object_name)
        } else {
            // Для неизвестных типов - просто имя с префиксом
            object_name
        }
    }

    /// Получить категорию объекта для TypeRepository
    fn get_category(&self) -> String {
        if let Some(kind) = self.object_type {
            kind.display_name().to_string()
        } else {
            "Неизвестные".to_string()
        }
    }

    /// Конвертировать атрибуты в свойства для RawTypeData
    fn convert_attributes_to_properties(&self) -> Vec<bsl_shared::domain::types::RawPropertyData> {
        self.attributes
            .iter()
            .map(|attr| bsl_shared::domain::types::RawPropertyData {
                name: attr.name.clone(),
                prop_type: attr.type_name.clone(),
                is_readonly: false, // TODO: определить из метаданных
            })
            .collect()
    }

    /// Конвертировать атрибуты в RawAttributeData
    fn convert_attributes(&self) -> Vec<RawAttributeData> {
        self.attributes
            .iter()
            .map(|attr| RawAttributeData {
                name: attr.name.clone(),
                attr_type: attr.type_name.clone(),
            })
            .collect()
    }

    /// Конвертировать табличные части в RawTabularSectionData
    fn convert_tabular_sections(&self) -> Vec<RawTabularSectionData> {
        self.tabular_sections
            .iter()
            .map(|ts| RawTabularSectionData {
                name: ts.name.clone(),
                attributes: ts
                    .attributes
                    .iter()
                    .map(|attr| RawAttributeData {
                        name: attr.name.clone(),
                        attr_type: attr.type_name.clone(),
                    })
                    .collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::AttributeInfo;
    use bsl_shared::domain::types::MetadataKind;

    #[test]
    fn test_convert_catalog_to_raw_type() {
        let mut obj = UniversalMetadataObject::new(
            "Catalog".to_string(),
            "Контрагенты".to_string(),
            "12345678-1234-1234-1234-123456789012".to_string(),
        );
        obj.synonym = Some("Контрагенты".to_string());
        obj.attributes.push(AttributeInfo {
            name: "ИНН".to_string(),
            type_name: "Строка".to_string(),
            synonym: Some("ИНН".to_string()),
        });

        let raw_type = obj.to_raw_type_data(None);

        assert_eq!(raw_type.name, "Справочники.Контрагенты");
        assert_eq!(raw_type.kind, Some(MetadataKind::Catalog));
        assert_eq!(raw_type.attributes.len(), 1);
        assert_eq!(raw_type.attributes[0].name, "ИНН");
        assert_eq!(raw_type.facets.len(), 5); // Manager, Object, Reference, Selection, List
    }

    #[test]
    fn test_convert_unknown_type() {
        let obj = UniversalMetadataObject::new(
            "UnknownType".to_string(),
            "СтранныйОбъект".to_string(),
            "12345678-1234-1234-1234-123456789012".to_string(),
        );

        let raw_type = obj.to_raw_type_data(None);

        assert_eq!(raw_type.name, "СтранныйОбъект");
        assert_eq!(raw_type.kind, None);
        assert_eq!(raw_type.facets.len(), 0); // Пустой список фасетов
    }

    #[test]
    fn test_convert_with_extension_prefix() {
        let obj = UniversalMetadataObject::new(
            "Catalog".to_string(),
            "Контрагенты".to_string(),
            "12345678-1234-1234-1234-123456789012".to_string(),
        );

        // Без префикса (основная конфигурация)
        let raw_type_no_prefix = obj.to_raw_type_data(None);
        assert_eq!(raw_type_no_prefix.name, "Справочники.Контрагенты");
        assert_eq!(raw_type_no_prefix.english_name, "Контрагенты");

        // С префиксом (расширение)
        let raw_type_with_prefix = obj.to_raw_type_data(Some("Тест_"));
        assert_eq!(raw_type_with_prefix.name, "Справочники.Тест_Контрагенты");
        assert_eq!(raw_type_with_prefix.english_name, "Тест_Контрагенты");

        // Пустой префикс (эквивалент None)
        let raw_type_empty_prefix = obj.to_raw_type_data(Some(""));
        assert_eq!(raw_type_empty_prefix.name, "Справочники.Контрагенты");
    }
}
