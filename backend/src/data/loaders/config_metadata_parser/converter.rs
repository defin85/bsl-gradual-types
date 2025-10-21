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
    pub fn to_raw_type_data(&self) -> RawTypeData {
        let type_name = self.get_full_type_name();

        RawTypeData {
            name: type_name.clone(),
            english_name: self.name.clone(), // TODO: извлечь английское имя из метаданных
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
        }
    }

    /// Получить полное имя типа (Справочники.Контрагенты)
    fn get_full_type_name(&self) -> String {
        if let Some(kind) = self.object_type {
            format!("{}.{}", kind.display_name(), self.name)
        } else {
            // Для неизвестных типов - просто имя
            self.name.clone()
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

        let raw_type = obj.to_raw_type_data();

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

        let raw_type = obj.to_raw_type_data();

        assert_eq!(raw_type.name, "СтранныйОбъект");
        assert_eq!(raw_type.kind, None);
        assert_eq!(raw_type.facets.len(), 0); // Пустой список фасетов
    }
}
