//! Functions to convert parsed data into the common `RawTypeData` format.

use crate::data::loaders::{SyntaxHelperDatabase, SyntaxNode, TypeInfo};
use bsl_shared::domain::signature_index::{ContextRequirements, MethodSignature, SignatureSource};
use bsl_shared::domain::types::{
    ParameterInfo as SignatureParam, RawDataSource, RawMethodData, RawParamData, RawPropertyData,
    RawTypeData, TypeResolution,
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

fn contexts_to_requirements(contexts: &[String]) -> Option<ContextRequirements> {
    if contexts.is_empty() {
        return None;
    }

    let mut has_server = false;
    let mut has_client = false;

    for ctx in contexts {
        let lower = ctx.to_lowercase();
        if lower.contains("сервер")
            || lower.contains("server")
            || lower.contains("внешнее соединение")
            || lower.contains("external connection")
        {
            has_server = true;
        }
        if lower.contains("клиент") || lower.contains("client") {
            has_client = true;
        }
    }

    match (has_server, has_client) {
        (true, true) => Some(ContextRequirements::Universal),
        (true, false) => Some(ContextRequirements::ServerOnly),
        (false, true) => Some(ContextRequirements::ClientOnly),
        (false, false) => None,
    }
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
    let (russian, _english) = get_method?;

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
    if rt.is_empty()
        || rt == "T"
        || rt == "Произвольный"
        || rt == "Неопределено"
        || rt.contains(',')
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
                let context_requirements = contexts_to_requirements(&method_info.contexts);

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
                                context_requirements,
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
                    context_requirements,
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

pub fn convert_syntax_helper_global_functions(db: &SyntaxHelperDatabase) -> Vec<MethodSignature> {
    let mut signatures = Vec::new();

    for func in db.global_functions.values() {
        let params: Vec<SignatureParam> = func
            .parameters
            .iter()
            .map(|p| SignatureParam {
                name: p.name.clone(),
                type_name: p.type_name.clone(),
                is_optional: p.is_optional,
                default_value: p.default_value.clone(),
                description: p.description.clone(),
            })
            .collect();

        let return_type = func
            .return_type
            .as_ref()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let description = func
            .description
            .as_ref()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        let return_description = func
            .return_description
            .as_ref()
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty());
        let context_requirements = contexts_to_requirements(&func.contexts).unwrap_or_default();

        let signature = MethodSignature::new(
            func.name.clone(),
            None,
            params.clone(),
            return_type.clone(),
            description.clone(),
            return_description.clone(),
            SignatureSource::Platform,
            None,
            context_requirements,
        );
        signatures.push(signature);

        if let Some(english) = func
            .english_name
            .as_ref()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty() && !name.eq_ignore_ascii_case(&func.name))
        {
            signatures.push(MethodSignature::new(
                english,
                None,
                params.clone(),
                return_type.clone(),
                description.clone(),
                return_description.clone(),
                SignatureSource::Platform,
                None,
                context_requirements,
            ));
        }
    }

    signatures
}

#[cfg(test)]
#[path = "converters/tests.rs"]
mod tests;
