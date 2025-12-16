//! Functions to convert parsed data into the common `RawTypeData` format.

use crate::data::loaders::{SyntaxHelperDatabase, SyntaxNode, TypeInfo};
use bsl_shared::domain::types::{
    RawDataSource, RawMethodData, RawParamData, RawPropertyData, RawTypeData, TypeResolution,
};
use std::collections::HashSet;
use tracing::warn;

/// Converts a full SyntaxHelperDatabase into a vector of RawTypeData.
pub fn convert_syntax_helper_to_raw(db: &SyntaxHelperDatabase) -> Vec<RawTypeData> {
    let known_type_names: HashSet<String> = db
        .nodes
        .values()
        .filter_map(|node| match node {
            SyntaxNode::Type(t) => Some(t.identity.russian_name.clone()),
            _ => None,
        })
        .collect();

    db.nodes
        .values()
        .filter_map(|node| {
            if let SyntaxNode::Type(type_info) = node {
                Some(convert_type_info_to_raw(type_info, db, &known_type_names))
            } else {
                None
            }
        })
        .collect()
}

fn derive_collection_item_type(
    type_info: &TypeInfo,
    db: &SyntaxHelperDatabase,
    known_type_names: &HashSet<String>,
) -> Option<String> {
    if type_info.structure.collection_element.is_some() {
        return type_info.structure.collection_element.clone();
    }

    // Fallback: пытаемся вывести item type по return type метода "Получить" (Get).
    // Используем только если return type выглядит как реальный тип (существует в базе типов),
    // чтобы не подхватывать фразы вроде "строка табличного поля".
    let get_method = type_info
        .structure
        .methods
        .iter()
        .find(|(ru, en)| ru == "Получить" || en == "Get");
    let Some((russian, _english)) = get_method else {
        return None;
    };

    let method_info = {
        let type_qualified_key = format!("method_{}.{}", type_info.identity.russian_name, russian);
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
    }?;

    let rt = method_info.return_type.clone()?;
    let rt = rt.trim();
    if rt.is_empty() || rt == "T" || rt == "Произвольный" || rt == "Неопределено" || rt.contains(',')
    {
        return None;
    }
    if known_type_names.contains(rt) {
        return Some(rt.to_string());
    }

    None
}

fn convert_type_info_to_raw(
    type_info: &TypeInfo,
    db: &SyntaxHelperDatabase,
    known_type_names: &HashSet<String>,
) -> RawTypeData {
    // Конвертируем методы с извлечением параметров из db.methods
    let methods = type_info
        .structure
        .methods
        .iter()
        .flat_map(|(russian, english)| {
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
                let english_name = method_info
                    .english_name
                    .clone()
                    .unwrap_or_else(|| english.clone());

                // Если в документации есть варианты синтаксиса — разворачиваем в overload'ы.
                if !method_info.overloads.is_empty() {
                    return method_info
                        .overloads
                        .iter()
                        .map(|ov| {
                            let description = match (&method_info.description, &ov.variant_name) {
                                (Some(base), Some(variant)) if !variant.is_empty() => {
                                    Some(format!("{} (вариант: {})", base, variant))
                                }
                                (Some(base), _) => Some(base.clone()),
                                (None, Some(variant)) if !variant.is_empty() => {
                                    Some(format!("Вариант: {}", variant))
                                }
                                (None, _) => None,
                            };

                            RawMethodData {
                                name: russian.clone(),
                                english_name: english_name.clone(),
                                return_type: method_info.return_type.clone().unwrap_or_default(),
                                params: ov
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
                                description,
                                is_deprecated: false,
                                is_constructor: method_info.name.starts_with("Новый")
                                    || method_info.name.starts_with("New"),
                                context_requirements: None, // TODO: Извлечь из Syntax Helper
                                return_facet: None, // TODO: Извлечь из Syntax Helper
                            }
                        })
                        .collect::<Vec<_>>();
                }

                vec![RawMethodData {
                    name: russian.clone(),
                    english_name,
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
                    context_requirements: None, // TODO: Извлечь из Syntax Helper
                    return_facet: None, // TODO: Извлечь из Syntax Helper
                }]
            } else {
                warn!(
                    "⚠️ Method {} not found in database for type {}",
                    russian, type_info.identity.russian_name
                );
                vec![RawMethodData {
                    name: russian.clone(),
                    english_name: english.clone(),
                    ..Default::default()
                }]
            }
        })
        .collect();

    // Конвертируем свойства, подтягивая тип/readonly из db.properties (если есть)
    let properties = type_info
        .structure
        .properties
        .iter()
        .map(|(russian, _english)| {
            let property_info = {
                let type_qualified_key =
                    format!("property_{}.{}", type_info.identity.russian_name, russian);
                db.properties
                    .get(&type_qualified_key)
                    .or_else(|| {
                        let simple_key = format!("property_{}", russian);
                        db.properties.get(&simple_key)
                    })
                    .or_else(|| {
                        db.properties.values().find(|prop| {
                            prop.name
                                .starts_with(&format!("{}.", type_info.identity.russian_name))
                                && prop.name.ends_with(&format!(".{}", russian))
                        })
                    })
            };

            if let Some(property_info) = property_info {
                RawPropertyData {
                    name: russian.clone(),
                    prop_type: property_info.property_type.clone().unwrap_or_default(),
                    is_readonly: property_info.is_readonly,
                }
            } else {
                warn!(
                    "⚠️ Property {} not found in database for type {}",
                    russian, type_info.identity.russian_name
                );
                RawPropertyData {
                    name: russian.clone(),
                    ..Default::default()
                }
            }
        })
        .collect();

    RawTypeData {
        name: type_info.identity.russian_name.clone(),
        english_name: type_info.identity.english_name.clone(),
        description: type_info.documentation.type_description.clone(),
        category: type_info.identity.category_path.clone(),
        source: RawDataSource::Platform,
        collection_item_type: derive_collection_item_type(type_info, db, known_type_names),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::syntax_helper::{DocumentParser, SyntaxHelperDatabase, SyntaxNode};
    use crate::data::loaders::syntax_helper::types::{
        MethodInfo, TypeDocumentation, TypeIdentity, TypeInfo, TypeMetadata, TypeStructure,
    };
    use scraper::Html;
    use std::path::Path;

    #[test]
    fn test_form_data_collection_collection_item_type_is_exported_to_raw() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let html_path = root.join(
            "examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog1649/catalog1614/FormDataCollection.html",
        );

        let content =
            std::fs::read_to_string(&html_path).expect("failed to read FormDataCollection.html");
        let document = Html::parse_document(&content);

        let parser = DocumentParser::new();
        let type_info = parser
            .parse_type(&html_path, &document)
            .expect("failed to parse type info");

        assert_eq!(
            type_info.structure.collection_element.as_deref(),
            Some("ДанныеФормыЭлементКоллекции"),
            "Должны извлечь item type из реального HTML"
        );

        let mut db = SyntaxHelperDatabase::default();
        db.nodes
            .insert("k".to_string(), SyntaxNode::Type(type_info));

        let raw = convert_syntax_helper_to_raw(&db);
        let form_data_collection = raw
            .iter()
            .find(|t| t.name == "ДанныеФормыКоллекция")
            .expect("RawTypeData for ДанныеФормыКоллекция should be present");

        assert_eq!(
            form_data_collection.collection_item_type.as_deref(),
            Some("ДанныеФормыЭлементКоллекции")
        );
    }

    #[test]
    fn test_fallback_collection_item_type_from_get_return_type() {
        let type_info = TypeInfo {
            identity: TypeIdentity {
                russian_name: "ТестКоллекция".to_string(),
                english_name: "TestCollection".to_string(),
                catalog_path: "objects/x/TestCollection.html".to_string(),
                aliases: Vec::new(),
                category_path: "Тесты".to_string(),
            },
            documentation: TypeDocumentation {
                category_description: None,
                type_description: "Коллекция без блока элементов".to_string(),
                examples: Vec::new(),
                availability: Vec::new(),
                since_version: "0".to_string(),
            },
            structure: TypeStructure {
                collection_element: None,
                methods: vec![("Получить".to_string(), "Get".to_string())],
                properties: Vec::new(),
                constructors: Vec::new(),
                iterable: true,
                indexable: true,
                enum_values: Vec::new(),
            },
            metadata: TypeMetadata {
                available_facets: Vec::new(),
                default_facet: None,
                serializable: false,
                exchangeable: false,
                xdto_namespace: None,
                xdto_type: None,
            },
        };

        let element_type_info = TypeInfo {
            identity: TypeIdentity {
                russian_name: "ЭлементТеста".to_string(),
                english_name: "TestItem".to_string(),
                catalog_path: "objects/x/TestItem.html".to_string(),
                aliases: Vec::new(),
                category_path: "Тесты".to_string(),
            },
            documentation: TypeDocumentation {
                category_description: None,
                type_description: String::new(),
                examples: Vec::new(),
                availability: Vec::new(),
                since_version: "0".to_string(),
            },
            structure: TypeStructure {
                collection_element: None,
                methods: Vec::new(),
                properties: Vec::new(),
                constructors: Vec::new(),
                iterable: false,
                indexable: false,
                enum_values: Vec::new(),
            },
            metadata: TypeMetadata {
                available_facets: Vec::new(),
                default_facet: None,
                serializable: false,
                exchangeable: false,
                xdto_namespace: None,
                xdto_type: None,
            },
        };

        let mut db = SyntaxHelperDatabase::default();
        db.nodes
            .insert("c".to_string(), SyntaxNode::Type(type_info));
        db.nodes
            .insert("i".to_string(), SyntaxNode::Type(element_type_info));
        db.methods.insert(
            "method_ТестКоллекция.Получить".to_string(),
            MethodInfo {
                name: "ТестКоллекция.Получить".to_string(),
                english_name: Some("Get".to_string()),
                description: None,
                overloads: Vec::new(),
                parameters: Vec::new(),
                return_type: Some("ЭлементТеста".to_string()),
                return_description: None,
            },
        );

        let raw = convert_syntax_helper_to_raw(&db);
        let coll = raw
            .iter()
            .find(|t| t.name == "ТестКоллекция")
            .expect("RawTypeData for ТестКоллекция should be present");

        assert_eq!(coll.collection_item_type.as_deref(), Some("ЭлементТеста"));
    }
}
