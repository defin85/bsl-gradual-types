//! Functions to convert parsed data into the common `RawTypeData` format.

use crate::data::loaders::{SyntaxHelperDatabase, SyntaxNode, TypeInfo};
use bsl_shared::domain::types::{
    RawDataSource, RawMethodData, RawPropertyData, RawTypeData, TypeResolution,
};

/// Converts a full SyntaxHelperDatabase into a vector of RawTypeData.
pub fn convert_syntax_helper_to_raw(db: &SyntaxHelperDatabase) -> Vec<RawTypeData> {
    db.nodes
        .values()
        .filter_map(|node| {
            if let SyntaxNode::Type(type_info) = node {
                Some(convert_type_info_to_raw(type_info))
            } else {
                None
            }
        })
        .collect()
}

fn convert_type_info_to_raw(type_info: &TypeInfo) -> RawTypeData {
    // Конвертируем методы с сохранением двуязычных имён
    let methods = type_info
        .structure
        .methods
        .iter()
        .map(|(russian, english)| RawMethodData {
            name: russian.clone(),
            english_name: english.clone(),
            ..Default::default()
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
