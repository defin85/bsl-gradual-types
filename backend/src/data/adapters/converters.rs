//! Functions to convert parsed data into the common `RawTypeData` format.

use crate::data::loaders::{SyntaxHelperDatabase, SyntaxNode, TypeInfo};
use bsl_shared::domain::types::{
    RawDataSource, RawMethodData, RawParamData, RawPropertyData, RawTypeData, TypeResolution,
};
use tracing::warn;

/// Converts a full SyntaxHelperDatabase into a vector of RawTypeData.
pub fn convert_syntax_helper_to_raw(db: &SyntaxHelperDatabase) -> Vec<RawTypeData> {
    db.nodes
        .values()
        .filter_map(|node| {
            if let SyntaxNode::Type(type_info) = node {
                Some(convert_type_info_to_raw(type_info, db))
            } else {
                None
            }
        })
        .collect()
}

fn convert_type_info_to_raw(type_info: &TypeInfo, db: &SyntaxHelperDatabase) -> RawTypeData {
    // Конвертируем методы с извлечением параметров из db.methods
    let methods = type_info
        .structure
        .methods
        .iter()
        .map(|(russian, english)| {
            // Ищем метод несколькими способами в порядке приоритета:
            // 1. По полному ключу с типом: "method_ТипДанных.МетодИмя"
            // 2. По простому ключу: "method_МетодИмя"
            // 3. Fallback: ищем метод нашего типа с совпадающим именем

            let method_info = {
                let type_qualified_key =
                    format!("method_{}.{}", type_info.identity.russian_name, russian);
                db.methods
                    .get(&type_qualified_key)
                    .or_else(|| {
                        let simple_key = format!("method_{}", russian);
                        db.methods.get(&simple_key)
                    })
                    .or_else(|| {
                        db.methods.values().find(|method| {
                            (method
                                .name
                                .starts_with(&format!("{}.", type_info.identity.russian_name))
                                && method.name.contains(&format!(".{}", russian)))
                                || (method.name.as_str() == russian)
                        })
                    })
            };

            if let Some(method_info) = method_info {
                RawMethodData {
                    name: russian.clone(),
                    english_name: method_info
                        .english_name
                        .clone()
                        .unwrap_or_else(|| english.clone()),
                    return_type: method_info.return_type.clone().unwrap_or_default(),
                    params: method_info
                        .parameters
                        .iter()
                        .map(|p| RawParamData {
                            name: p.name.clone(),
                            param_type: p
                                .type_name
                                .clone()
                                .unwrap_or_else(|| "Произвольный".to_string()),
                            is_optional: p.is_optional,
                            default_value: p.default_value.clone(),
                        })
                        .collect(),
                    description: method_info.description.clone(),
                    is_deprecated: false,
                    is_constructor: method_info.name.starts_with("Новый")
                        || method_info.name.starts_with("New"),
                }
            } else {
                warn!(
                    "⚠️ Method {} not found in database for type {}",
                    russian, type_info.identity.russian_name
                );
                RawMethodData {
                    name: russian.clone(),
                    english_name: english.clone(),
                    ..Default::default()
                }
            }
        })
        .collect();

    // Конвертируем свойства с сохранением двуязычных имён (если будут)
    let properties = type_info
        .structure
        .properties
        .iter()
        .map(|(russian, _english)| RawPropertyData {
            name: russian.clone(),
            ..Default::default()
        })
        .collect();

    RawTypeData {
        name: type_info.identity.russian_name.clone(),
        english_name: type_info.identity.english_name.clone(),
        description: type_info.documentation.type_description.clone(),
        category: type_info.identity.category_path.clone(),
        source: RawDataSource::Platform,
        methods,
        properties,
        facets: type_info.metadata.available_facets.clone(),
        enum_values: type_info.structure.enum_values.clone(),
        ..Default::default()
    }
}

pub fn convert_resolutions_to_raw(_resolutions: &[TypeResolution]) -> Vec<RawTypeData> {
    vec![]
}
