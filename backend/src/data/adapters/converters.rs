//! Functions to convert parsed data into the common `RawTypeData` format.

use bsl_shared::domain::types::{
    RawTypeData, RawDataSource, RawMethodData, RawPropertyData, RawAttributeData,
    RawTabularSectionData, TypeResolution
};
use crate::data::loaders::{SyntaxHelperDatabase, SyntaxNode, TypeInfo, DiscoveredMetadata};

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
    let methods = type_info.structure.methods.iter().map(|name| RawMethodData {
        name: name.clone(),
        ..Default::default()
    }).collect();

    let properties = type_info.structure.properties.iter().map(|name| RawPropertyData {
        name: name.clone(),
        ..Default::default()
    }).collect();

    RawTypeData {
        name: type_info.identity.russian_name.clone(),
        english_name: type_info.identity.english_name.clone(),
        description: type_info.documentation.type_description.clone(),
        category: type_info.identity.category_path.clone(),
        source: RawDataSource::Platform,
        methods,
        properties,
        facets: type_info.metadata.available_facets.clone(),
        ..Default::default()
    }
}

/// Converts a vector of DiscoveredMetadata from config parser into RawTypeData.
pub fn convert_discovered_metadata_to_raw(metadata: &[DiscoveredMetadata]) -> Vec<RawTypeData> {
    metadata.iter().map(|meta| {
        let attributes = meta.attributes.iter().map(|attr| RawAttributeData {
            name: attr.name.clone(),
            attr_type: attr.type_definition.clone(),
        }).collect();
        
        let tabular_sections = meta.tabular_sections.iter().map(|ts| RawTabularSectionData {
            name: ts.name.clone(),
            attributes: ts.attributes.iter().map(|attr| RawAttributeData {
                name: attr.name.clone(),
                attr_type: attr.type_definition.clone(),
            }).collect(),
        }).collect();

        RawTypeData {
            name: meta.name.clone(),
            english_name: meta.synonym.clone().unwrap_or_default(),
            description: format!("Конфигурационный объект: {}", meta.qualified_name),
            category: format!("{:?}", meta.kind),
            source: RawDataSource::Configuration,
            kind: Some(meta.kind),
            attributes,
            tabular_sections,
            ..Default::default()
        }
    }).collect()
}

pub fn convert_resolutions_to_raw(_resolutions: &[TypeResolution]) -> Vec<RawTypeData> {
    vec![]
}