use super::*;

pub(super) fn add_methods_from_resolution(
    metadata_lookup: &TypeMetadataLookup,
    resolution: &TypeResolution,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let owner_type = resolution.type_name();
    let methods = metadata_lookup.get_methods(resolution);
    for method in methods {
        target.push(Candidate::new(
            CompletionItem::new(method.name, CompletionKind::Method),
            priority,
            Some(owner_type.clone()),
            None,
            None,
        ));
    }
}

pub(super) fn add_properties_from_resolution(
    metadata_lookup: &TypeMetadataLookup,
    resolution: &TypeResolution,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    let owner_type = resolution.type_name();
    let properties = metadata_lookup.get_properties_with_origin(resolution);
    let mut intrinsic_count = 0usize;
    let mut saw_intrinsic = false;
    for (property, origin) in properties {
        let member_identity = resolution
            .find_structural_member(&property.name)
            .map(|member| member.member_id.key.clone());
        let property_priority = if TypeMetadataLookup::is_intrinsic_property_origin(origin) {
            intrinsic_count += 1;
            saw_intrinsic = true;
            // Для form-data pipeline сохраняем provider order в source_priority:
            // shape/repository-before-intrinsic -> intrinsic -> facet/fallback-repository.
            priority.saturating_add(1)
        } else if saw_intrinsic {
            priority.saturating_add(2)
        } else {
            priority
        };

        target.push(Candidate::new(
            CompletionItem::new(property.name, CompletionKind::Property),
            property_priority,
            Some(owner_type.clone()),
            member_identity,
            None,
        ));
    }

    if intrinsic_count > 0 {
        tracing::debug!(
            metric = "completion_form_data_intrinsic_candidates_total",
            owner_type = owner_type,
            count = intrinsic_count,
            "Added intrinsic form-data property candidates"
        );
    }
}

pub(super) async fn resolve_member_owner_type(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    _file_content: &str,
    _line: u32,
    _column: u32,
    _base_name: &str,
) -> Option<TypeResolution> {
    resolve_member_owner_type_sync(analysis, _file_content, _line, _column, _base_name)
}

pub(super) fn resolve_member_owner_type_sync(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    _file_content: &str,
    _line: u32,
    _column: u32,
    _base_name: &str,
) -> Option<TypeResolution> {
    let ctx = analysis?;
    ctx.member_access_owner_type_hint
        .as_ref()
        .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
        .cloned()
}

pub(super) fn resolve_property_access_type(
    resolver: Option<&TypeResolver>,
    metadata_lookup: &TypeMetadataLookup,
    owner: &TypeResolution,
    property_name: &str,
) -> Option<TypeResolution> {
    let owner_type_name = owner.type_name();
    let lowered = property_name.to_lowercase();
    let property = metadata_lookup
        .get_properties(owner)
        .into_iter()
        .find(|item| item.name.to_lowercase() == lowered)?;
    if property.prop_type.trim().is_empty() {
        return None;
    }

    if let Some(resolver) = resolver {
        if property
            .prop_type
            .trim_start()
            .starts_with("ТабличнаяЧасть<")
        {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(cfg)) = &owner.result {
                let tabular_sections = metadata_lookup.get_tabular_sections(owner);
                let lowered = property_name.to_lowercase();
                if let Some(ts) = tabular_sections
                    .iter()
                    .find(|ts| ts.name.to_lowercase() == lowered)
                {
                    let parent_type = if cfg.name.contains('.') {
                        cfg.name.clone()
                    } else {
                        format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
                    };
                    let expr = format!("{}.{}", parent_type, ts.name);
                    let resolved = resolver.resolve_expression_sync(&expr);
                    if !resolved.is_unknown() {
                        return Some(resolved);
                    }
                }
            }
        }
    }

    let resolved_type = substitute_type_name_if_needed(&property.prop_type, &owner_type_name);
    Some(resolve_type_from_string(resolver, &resolved_type))
}

pub(super) fn resolve_method_call_return_type(
    resolver: Option<&TypeResolver>,
    metadata_lookup: &TypeMetadataLookup,
    owner: &TypeResolution,
    method_name: &str,
) -> Option<TypeResolution> {
    let owner_type_name = owner.type_name();

    if matches!(owner.result, ResolutionResult::Generic(_)) {
        if let ResolutionResult::Generic(generic) = &owner.result {
            let base = generic.base_type.to_lowercase();
            let method = method_name.to_lowercase();
            if base == "табличнаячасть"
                && matches!(
                    method.as_str(),
                    "добавить" | "вставить" | "получить" | "найти"
                )
            {
                if let Some(concrete) = generic.type_params.first() {
                    if !matches!(concrete, ConcreteType::Special(SpecialType::Undefined)) {
                        if let Some(resolver) = resolver {
                            return Some(resolve_concrete_type(resolver, concrete));
                        }
                    }
                }
            }
        }

        let lowered = method_name.to_lowercase();
        let method = metadata_lookup
            .get_methods(owner)
            .into_iter()
            .find(|item| item.name.to_lowercase() == lowered)?;
        if method.return_type.trim().is_empty() {
            return None;
        }
        let resolved_type = substitute_type_name_if_needed(&method.return_type, &owner_type_name);
        return Some(resolve_type_from_string(resolver, &resolved_type));
    }

    let signature = metadata_lookup.find_method_signature_for_call(Some(owner), method_name);
    if let Some(signature) = signature {
        let return_type = signature.return_type.as_deref().unwrap_or("Неопределено");

        if return_type == "T" {
            if let ResolutionResult::Generic(generic) = &owner.result {
                if let Some(concrete) = generic.type_params.first() {
                    if !matches!(concrete, ConcreteType::Special(SpecialType::Undefined)) {
                        if let Some(resolver) = resolver {
                            return Some(resolve_concrete_type(resolver, concrete));
                        }
                    }
                }
            }
        }

        let resolved_type = substitute_type_name_if_needed(return_type, &owner_type_name);
        return Some(resolve_type_from_string(resolver, &resolved_type));
    }

    let lowered = method_name.to_lowercase();
    let method = metadata_lookup
        .get_methods(owner)
        .into_iter()
        .find(|item| item.name.to_lowercase() == lowered)?;
    if method.return_type.trim().is_empty() {
        return None;
    }

    let resolved_type = substitute_type_name_if_needed(&method.return_type, &owner_type_name);
    Some(resolve_type_from_string(resolver, &resolved_type))
}

pub(super) fn resolve_concrete_type(
    resolver: &TypeResolver,
    concrete: &ConcreteType,
) -> TypeResolution {
    let type_name = match concrete {
        ConcreteType::Primitive(pt) => pt.display_name().to_string(),
        ConcreteType::Platform(pt) => pt.name.clone(),
        ConcreteType::Special(s) => s.display_name().to_string(),
        ConcreteType::GlobalFunction(func) => func.name.clone(),
        ConcreteType::TabularRow(row) => row.get_full_name(),
        ConcreteType::Configuration(cfg) => {
            if let Some(facet) = cfg.facet {
                format!("{}.{}", cfg.kind.faceted_type_prefix(&facet), cfg.name)
            } else {
                format!("{}.{}", cfg.kind.to_prefix(), cfg.name)
            }
        }
    };
    resolver.resolve_expression_sync(&type_name)
}

pub(super) fn substitute_type_name_if_needed(type_name: &str, owner_type: &str) -> String {
    let Some(metadata_name) = SignatureIndex::extract_metadata_name(owner_type) else {
        return type_name.to_string();
    };
    SignatureIndex::substitute_type_name(type_name, metadata_name)
}

pub(super) fn resolve_type_from_string(
    resolver: Option<&TypeResolver>,
    type_name: &str,
) -> TypeResolution {
    let type_name = type_name.trim();
    if type_name.is_empty() {
        return TypeResolution::unknown();
    }
    resolver
        .map(|resolver| resolver.resolve_expression_sync(type_name))
        .unwrap_or_else(|| TypeResolution::explicit(type_name))
}

#[derive(Debug, Clone)]
pub(super) struct CompletionScopePosition {
    pub(super) byte_offset: u32,
    pub(super) scope_rank: HashMap<ScopeId, usize>,
}

#[derive(Debug, Clone)]
pub(super) struct LocalSymbolCandidate {
    pub(super) name: String,
    pub(super) scope_id: ScopeId,
    pub(super) span_start: u32,
}

pub(super) const IMPLICIT_CONTEXT_SYMBOL_KEYS: [&str; 6] = [
    "этотобъект",
    "этаформа",
    "форма",
    "объект",
    "элементы",
    "параметры",
];

pub fn resolve_type_details(
    type_name: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<(Option<String>, Option<String>)> {
    let resolution = TypeResolution::explicit(type_name);
    let raw = metadata_lookup.get_raw_type(&resolution)?;

    let detail = if raw.category.is_empty() {
        None
    } else {
        Some(raw.category)
    };
    let documentation = if raw.description.is_empty() {
        None
    } else {
        Some(raw.description)
    };

    Some((detail, documentation))
}

#[derive(Debug, Clone)]
pub struct CompletionResolveDetails {
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

pub fn resolve_method_completion(
    owner_type: &str,
    method_name: &str,
    metadata_lookup: &TypeMetadataLookup,
    snippet_support: bool,
) -> Option<CompletionResolveDetails> {
    let resolution = TypeResolution::explicit(owner_type);
    let methods = metadata_lookup.get_methods(&resolution);
    let lowered = method_name.to_lowercase();
    let method = methods
        .into_iter()
        .find(|item| item.name.to_lowercase() == lowered)?;

    let detail = if method.return_type.is_empty() {
        None
    } else {
        Some(method.return_type.clone())
    };
    let documentation = method.description.clone();
    let insert_text = if snippet_support {
        build_method_snippet(&method)
    } else {
        None
    };

    Some(CompletionResolveDetails {
        detail,
        documentation,
        insert_text,
    })
}

pub fn build_call_snippet(name: &str, params: &[(String, bool)]) -> Option<String> {
    if params.is_empty() {
        return None;
    }

    fn normalize_param_label(param_name: &str) -> String {
        let trimmed = param_name.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // 1C Syntax Helper часто использует плейсхолдеры в угловых скобках:
        // "<Имя>", "<Тип>", "<Заголовок>" и т.п. (в т.ч. HTML-encoded).
        // Для сниппета нам нужен читабельный label без скобок.
        let unwrapped = trimmed
            .strip_prefix('<')
            .and_then(|s| s.strip_suffix('>'))
            .or_else(|| {
                trimmed
                    .strip_prefix("&lt;")
                    .and_then(|s| s.strip_suffix("&gt;"))
            })
            .unwrap_or(trimmed);

        unwrapped.trim().to_string()
    }

    let mut required: Vec<(String, bool)> = Vec::new();
    let mut optional: Vec<(String, bool)> = Vec::new();
    for (param_name, is_optional) in params {
        if *is_optional {
            optional.push((param_name.clone(), true));
        } else {
            required.push((param_name.clone(), false));
        }
    }

    let mut parts = Vec::with_capacity(params.len());
    let mut index = 1;
    for (param_name, is_optional) in required.into_iter().chain(optional) {
        let normalized = normalize_param_label(&param_name);
        let label = if normalized.is_empty() {
            format!("param{}", index)
        } else {
            normalized
        };

        // Даже если параметр необязательный, показываем его имя в плейсхолдере:
        // это важно для UX (Tab по аргументам в сниппете).
        //
        // Клиент может удалять/пропускать необязательные параметры вручную,
        // но пустые плейсхолдеры ухудшают подсказки и навигацию.
        let placeholder = if is_optional && param_name.trim().is_empty() {
            // Если имя реально пустое - оставляем пустым плейсхолдером.
            format!("${{{}:}}", index)
        } else {
            format!("${{{}:{}}}", index, escape_snippet_text(&label))
        };
        parts.push(placeholder);
        index += 1;
    }

    let name = escape_snippet_text(name);
    Some(format!("{}({})$0", name, parts.join(", ")))
}

fn build_method_snippet(method: &bsl_shared::domain::types::RawMethodData) -> Option<String> {
    let mut params: Vec<(String, bool)> = Vec::with_capacity(method.params.len());
    for param in &method.params {
        params.push((param.name.clone(), param.is_optional));
    }
    build_call_snippet(&method.name, &params)
}

fn escape_snippet_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '$' => escaped.push_str("\\$"),
            '}' => escaped.push_str("\\}"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
