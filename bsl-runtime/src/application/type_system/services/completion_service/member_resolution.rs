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

#[cfg(test)]
pub(super) async fn resolve_member_owner_type(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    _file_content: &str,
    _line: u32,
    _column: u32,
    _base_name: &str,
) -> Option<TypeResolution> {
    resolve_member_owner_type_sync(analysis, _file_content, _line, _column, _base_name)
}

#[cfg(test)]
pub(super) fn resolve_member_owner_type_sync(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    _base_name: &str,
) -> Option<TypeResolution> {
    let ctx = analysis?;
    ctx.member_access_owner_type_hint
        .as_ref()
        .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
        .cloned()
        .or_else(|| resolve_member_access_owner_type_from_ir(analysis, file_content, line, column))
}

pub(super) fn resolve_member_owner_types_sync(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    _base_name: &str,
) -> Vec<TypeResolution> {
    let Some(ctx) = analysis else {
        return Vec::new();
    };

    if let Some(owner_hint) = ctx
        .member_access_owner_type_hint
        .as_ref()
        .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
        .cloned()
    {
        return vec![owner_hint];
    }

    resolve_member_access_owner_types_from_ir(analysis, file_content, line, column)
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
