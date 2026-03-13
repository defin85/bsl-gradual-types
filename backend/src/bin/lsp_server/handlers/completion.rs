//! Completion handler for LSP
//!
//! Handles textDocument/completion requests.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::error;

use bsl_backend::application::get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints;
use bsl_backend::application::type_system::{
    build_call_snippet, resolve_method_completion, resolve_type_details,
};
use bsl_backend::application::CompletionStats;
use bsl_backend::system::IndexSnapshot;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
use bsl_shared::domain::types::{MetadataKind, TypeResolution};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::normalize_user_facing_type_name;
use bsl_shared::ir::SemanticProgram;

#[path = "completion/kinds.rs"]
mod kinds;
use kinds::{completion_kind_tag, map_completion_kind};

const COMPLETION_CANDIDATE_ID_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionCandidateId {
    v: u32,
    #[serde(flatten)]
    payload: CompletionCandidateIdPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum CompletionCandidateIdPayload {
    Method {
        owner_type: String,
        name: String,
        sig_hash: Option<String>,
    },
    Property {
        owner_type: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        member_identity: Option<String>,
    },
    Function {
        name: String,
        sig_hash: Option<String>,
        resolve: bool,
    },
    Type {
        name: String,
    },
    Metadata {
        kind: MetadataKind,
        name: String,
    },
    Keyword {
        name: String,
    },
    Other {
        kind: String,
        name: String,
    },
}

pub struct CompletionResponseWithStats {
    pub response: CompletionResponse,
    #[allow(dead_code)]
    pub stats: Option<CompletionStats>,
    pub had_error: bool,
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub async fn handle_completion_v2(
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    member_access_owner_type_hint: Option<TypeResolution>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    file_uri: &Url,
    index_snapshot: &IndexSnapshot,
    snippet_support: bool,
    include_flow_sensitive: bool,
) -> Option<CompletionResponseWithStats> {
    handle_completion_v2_with_trigger_hint(
        file_content,
        file_path,
        ir_program,
        member_access_owner_type_hint,
        deps,
        position,
        file_uri,
        index_snapshot,
        snippet_support,
        include_flow_sensitive,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_completion_v2_with_trigger_hint(
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    member_access_owner_type_hint: Option<TypeResolution>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    file_uri: &Url,
    index_snapshot: &IndexSnapshot,
    snippet_support: bool,
    include_flow_sensitive: bool,
    trigger_char_hint: Option<char>,
) -> Option<CompletionResponseWithStats> {
    handle_completion_v2_with_trigger_hint_and_owner_hints(
        file_content,
        file_path,
        ir_program,
        member_access_owner_type_hint.into_iter().collect(),
        deps,
        position,
        file_uri,
        index_snapshot,
        snippet_support,
        include_flow_sensitive,
        trigger_char_hint,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_completion_v2_with_trigger_hint_and_owner_hints(
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    member_access_owner_type_hints: Vec<TypeResolution>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    file_uri: &Url,
    index_snapshot: &IndexSnapshot,
    snippet_support: bool,
    include_flow_sensitive: bool,
    trigger_char_hint: Option<char>,
) -> Option<CompletionResponseWithStats> {
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let member_access_request = completion_request_targets_member_access(
        file_content.as_ref(),
        position,
        trigger_char_hint,
    );
    let member_access_owner_type_hints = member_access_owner_type_hints
        .into_iter()
        .filter(|hint| !hint.is_unknown() && !hint.is_dynamic())
        .collect::<Vec<_>>();

    // Default LSP delivery must not reconstruct owner type from IR when the shared
    // exact owner hint is unavailable for a member-access request.
    if member_access_request && member_access_owner_type_hints.is_empty() {
        return Some(CompletionResponseWithStats {
            response: CompletionResponse::List(CompletionList {
                is_incomplete: false,
                items: vec![],
            }),
            stats: None,
            had_error: false,
        });
    }

    let completion = get_completion_with_semantic_program_snapshot_with_trigger_hint_and_owner_hints(
        file_content.as_ref(),
        position.line,
        position.character,
        Some(file_uri.as_str()),
        index_snapshot,
        &metadata_lookup,
        file_path.as_ref(),
        resolver.as_ref(),
        ir_program,
        member_access_owner_type_hints,
        include_flow_sensitive,
        trigger_char_hint,
    )
    .await;

    match completion {
        Ok(result) => {
            let lsp_completions: Vec<CompletionItem> = result
                .items
                .into_iter()
                .map(|candidate| {
                    to_lsp_completion(
                        candidate.item,
                        candidate.owner_type,
                        candidate.member_identity,
                        candidate.origin_sources,
                        snippet_support,
                        Some(deps.as_ref()),
                    )
                })
                .collect();
            Some(CompletionResponseWithStats {
                response: CompletionResponse::List(CompletionList {
                    is_incomplete: result.is_incomplete,
                    items: lsp_completions,
                }),
                stats: Some(result.stats),
                had_error: false,
            })
        }
        Err(e) => {
            error!("Failed to get completions (v2): {}", e);
            Some(CompletionResponseWithStats {
                response: CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: vec![],
                }),
                stats: None,
                had_error: true,
            })
        }
    }
}

fn completion_request_targets_member_access(
    text: &str,
    position: Position,
    trigger_char_hint: Option<char>,
) -> bool {
    if trigger_char_hint == Some('.') {
        return true;
    }

    let Some(line_text) = text.lines().nth(position.line as usize) else {
        return false;
    };
    let column_index =
        bsl_analysis_v2::utf16_to_byte_offset(line_text, position.character).min(line_text.len());
    let line_prefix = line_text.get(..column_index).unwrap_or(line_text);
    let line_prefix = if line_text
        .get(column_index..)
        .and_then(|tail| tail.chars().next())
        == Some('.')
    {
        format!("{line_prefix}.")
    } else {
        line_prefix.to_string()
    };

    let trimmed = line_prefix.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty() || after_dot.chars().all(is_completion_identifier_char)
}

fn is_completion_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

pub async fn handle_completion_resolve(
    mut item: CompletionItem,
    deps: Option<Arc<bsl_analysis_v2::SemanticDeps>>,
    snippet_support: bool,
) -> CompletionItem {
    if item.detail.is_some() && item.documentation.is_some() && !snippet_support {
        return item;
    }

    let deps = match deps {
        Some(deps) => deps,
        None => return item,
    };

    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let Some(candidate_id) = parse_candidate_id(&item) else {
        return item;
    };
    let resolved = resolve_by_candidate_id(
        &candidate_id,
        deps.as_ref(),
        &metadata_lookup,
        snippet_support,
    );

    if let Some((detail, documentation, insert_text)) = resolved {
        item.detail = detail.map(|value| normalize_user_facing_type_name(&value));
        if let Some(doc) = documentation {
            item.documentation = Some(Documentation::String(normalize_user_facing_type_name(&doc)));
        }
        if let Some(snippet) = insert_text {
            if snippet_support {
                item.insert_text = Some(snippet);
                item.insert_text_format = Some(InsertTextFormat::SNIPPET);
            }
        }
    }

    item
}

fn to_lsp_completion(
    item: bsl_shared::domain::CompletionItem,
    owner_type: Option<String>,
    member_identity: Option<String>,
    origin_sources: Vec<u8>,
    snippet_support: bool,
    deps: Option<&bsl_analysis_v2::SemanticDeps>,
) -> CompletionItem {
    let kind_tag = completion_kind_tag(&item);
    let candidate_id = build_candidate_id(
        kind_tag,
        &item,
        owner_type.as_deref(),
        member_identity.as_deref(),
        &origin_sources,
        deps,
    );
    let kind = map_completion_kind(item.kind);
    let mut insert_text = item.insert_text;
    let contains_snippet = insert_text
        .as_deref()
        .map(|text| text.contains("${"))
        .unwrap_or(false);
    if !snippet_support && contains_snippet {
        insert_text = Some(item.label.clone());
    }
    let insert_text_format = if snippet_support && contains_snippet {
        Some(InsertTextFormat::SNIPPET)
    } else {
        None
    };

    let mut data = serde_json::Map::new();
    data.insert("kind".to_string(), json!(kind_tag));
    data.insert("owner_type".to_string(), json!(owner_type));
    data.insert("origin_sources".to_string(), json!(origin_sources));
    data.insert("candidate_id".to_string(), json!(candidate_id));
    if let Some(member_identity) = member_identity {
        data.insert("member_identity".to_string(), json!(member_identity));
    }

    CompletionItem {
        label: item.label,
        kind,
        detail: None,
        documentation: None,
        insert_text,
        insert_text_format,
        filter_text: item.filter_text,
        sort_text: item.sort_text,
        data: Some(serde_json::Value::Object(data)),
        ..Default::default()
    }
}

fn build_candidate_id(
    kind_tag: &str,
    item: &bsl_shared::domain::CompletionItem,
    owner_type: Option<&str>,
    member_identity: Option<&str>,
    origin_sources: &[u8],
    deps: Option<&bsl_analysis_v2::SemanticDeps>,
) -> CompletionCandidateId {
    fn normalize_owner_type(owner: &str) -> String {
        bsl_shared::domain::type_id::TypeId::new(owner)
            .without_generic_params()
            .display()
            .to_string()
    }

    let payload = if kind_tag.starts_with("metadata.") {
        CompletionCandidateIdPayload::Metadata {
            kind: item.kind.metadata_kind().unwrap_or(MetadataKind::Unknown),
            name: item.label.clone(),
        }
    } else {
        match kind_tag {
            "method" => match owner_type {
                Some(owner) => {
                    let owner = normalize_owner_type(owner);
                    let sig_hash = deps
                        .and_then(|deps| deps.signature_index.find_method(&owner, &item.label))
                        .map(method_signature_hash);

                    CompletionCandidateIdPayload::Method {
                        owner_type: owner,
                        name: item.label.clone(),
                        sig_hash,
                    }
                }
                None => CompletionCandidateIdPayload::Other {
                    kind: kind_tag.to_string(),
                    name: item.label.clone(),
                },
            },
            "property" => match owner_type {
                Some(owner) => CompletionCandidateIdPayload::Property {
                    owner_type: normalize_owner_type(owner),
                    name: item.label.clone(),
                    member_identity: member_identity.map(str::to_string),
                },
                None => CompletionCandidateIdPayload::Other {
                    kind: kind_tag.to_string(),
                    name: item.label.clone(),
                },
            },
            "function" => {
                // Для `CompletionKind::Function` считаем, что:
                // - `source_priority=0` → локальные/файловые символы (не resolve'им по SignatureIndex)
                // - иначе → модульные/глобальные функции (resolve'им, если есть сигнатура)
                //
                // Важно: `origin_sources` может быть merge'нут ранжированием/дедупом,
                // поэтому берём "лучший" источник как `min(origin_sources)`.
                let best_source = origin_sources.iter().copied().min();
                let resolve = best_source.map(|source| source != 0).unwrap_or(false);
                let sig_hash = if resolve {
                    deps.and_then(|deps| deps.signature_index.find_global_function(&item.label))
                        .map(method_signature_hash)
                } else {
                    None
                };

                CompletionCandidateIdPayload::Function {
                    name: item.label.clone(),
                    sig_hash,
                    resolve,
                }
            }
            "type" => CompletionCandidateIdPayload::Type {
                name: item.label.clone(),
            },
            "keyword" => CompletionCandidateIdPayload::Keyword {
                name: item.label.clone(),
            },
            _ => CompletionCandidateIdPayload::Other {
                kind: kind_tag.to_string(),
                name: item.label.clone(),
            },
        }
    };

    CompletionCandidateId {
        v: COMPLETION_CANDIDATE_ID_VERSION,
        payload,
    }
}

fn method_signature_hash(signature: &MethodSignature) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"completion-sig-v1|");

    let source_code = match signature.source {
        SignatureSource::Platform => b"p",
        SignatureSource::Configuration => b"c",
        SignatureSource::UserCode => b"u",
    };
    hasher.update(source_code);
    hasher.update(b"|");

    if let Some(owner_type) = signature.owner_type.as_deref() {
        hasher.update(owner_type.to_lowercase().as_bytes());
    }
    hasher.update(b"|");
    hasher.update(signature.name.to_lowercase().as_bytes());
    hasher.update(b"|");
    if let Some(return_type) = signature.return_type.as_deref() {
        hasher.update(return_type.to_lowercase().as_bytes());
    }
    hasher.update(b"|");

    for param in &signature.params {
        hasher.update(param.name.to_lowercase().as_bytes());
        hasher.update(b":");
        if let Some(type_name) = param.type_name.as_deref() {
            hasher.update(type_name.to_lowercase().as_bytes());
        }
        hasher.update(b":");
        hasher.update(if param.is_optional { b"1" } else { b"0" });
        hasher.update(b"|");
    }

    hasher.finalize().to_hex().to_string()
}

fn parse_candidate_id(item: &CompletionItem) -> Option<CompletionCandidateId> {
    let data = item.data.as_ref()?;
    let value = data.get("candidate_id")?.clone();
    serde_json::from_value(value).ok()
}

fn resolve_by_candidate_id(
    candidate_id: &CompletionCandidateId,
    deps: &bsl_analysis_v2::SemanticDeps,
    metadata_lookup: &TypeMetadataLookup,
    snippet_support: bool,
) -> Option<(Option<String>, Option<String>, Option<String>)> {
    if candidate_id.v != COMPLETION_CANDIDATE_ID_VERSION {
        return None;
    }

    match &candidate_id.payload {
        CompletionCandidateIdPayload::Method {
            owner_type,
            name,
            sig_hash,
        } => resolve_method_by_candidate_id(
            owner_type,
            name,
            sig_hash.as_deref(),
            deps,
            metadata_lookup,
            snippet_support,
        ),
        CompletionCandidateIdPayload::Property {
            owner_type, name, ..
        } => resolve_property_by_candidate_id(owner_type, name, metadata_lookup),
        CompletionCandidateIdPayload::Function {
            name,
            sig_hash,
            resolve,
        } => {
            if !resolve {
                return None;
            }
            resolve_function_by_candidate_id(name, sig_hash.as_deref(), deps, snippet_support)
        }
        CompletionCandidateIdPayload::Type { name } => resolve_type_details(name, metadata_lookup)
            .map(|(detail, documentation)| (detail, documentation, None)),
        CompletionCandidateIdPayload::Metadata { kind, name } => {
            resolve_metadata_by_candidate_id(*kind, name, metadata_lookup)
        }
        CompletionCandidateIdPayload::Keyword { .. } => None,
        CompletionCandidateIdPayload::Other { .. } => None,
    }
}

fn resolve_method_by_candidate_id(
    owner_type: &str,
    name: &str,
    sig_hash: Option<&str>,
    deps: &bsl_analysis_v2::SemanticDeps,
    metadata_lookup: &TypeMetadataLookup,
    snippet_support: bool,
) -> Option<(Option<String>, Option<String>, Option<String>)> {
    let owner_type = bsl_shared::domain::type_id::TypeId::new(owner_type)
        .without_generic_params()
        .display()
        .to_string();

    let signature = sig_hash
        .and_then(|expected| {
            deps.signature_index
                .find_methods(&owner_type, name)
                .into_iter()
                .find(|signature| method_signature_hash(signature) == expected)
                .cloned()
        })
        .or_else(|| deps.signature_index.find_method(&owner_type, name).cloned());

    if let Some(signature) = signature {
        let detail = signature
            .return_type
            .clone()
            .filter(|value| !value.is_empty());
        let documentation = signature.description.clone();
        let params: Vec<(String, bool)> = signature
            .params
            .iter()
            .map(|param| (param.name.clone(), param.is_optional))
            .collect();
        let insert_text = if snippet_support {
            build_call_snippet(&signature.name, &params)
        } else {
            None
        };

        return Some((detail, documentation, insert_text));
    }

    resolve_method_completion(&owner_type, name, metadata_lookup, snippet_support)
        .map(|details| (details.detail, details.documentation, details.insert_text))
}

fn resolve_function_by_candidate_id(
    name: &str,
    _sig_hash: Option<&str>,
    deps: &bsl_analysis_v2::SemanticDeps,
    snippet_support: bool,
) -> Option<(Option<String>, Option<String>, Option<String>)> {
    let signature = deps.signature_index.find_global_function(name).cloned()?;

    let detail = signature
        .return_type
        .clone()
        .filter(|value| !value.is_empty());
    let documentation = signature.description.clone();
    let params: Vec<(String, bool)> = signature
        .params
        .iter()
        .map(|param| (param.name.clone(), param.is_optional))
        .collect();
    let insert_text = if snippet_support {
        build_call_snippet(&signature.name, &params)
    } else {
        None
    };

    Some((detail, documentation, insert_text))
}

fn resolve_property_by_candidate_id(
    owner_type: &str,
    name: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<(Option<String>, Option<String>, Option<String>)> {
    let owner_type = bsl_shared::domain::type_id::TypeId::new(owner_type)
        .without_generic_params()
        .display()
        .to_string();
    let resolution = TypeResolution::explicit(&owner_type);
    let lowered = name.to_lowercase();
    let property = metadata_lookup
        .get_properties(&resolution)
        .into_iter()
        .find(|prop| prop.name.to_lowercase() == lowered)?;

    let detail = if property.prop_type.is_empty() {
        None
    } else {
        Some(property.prop_type)
    };
    Some((detail, None, None))
}

fn resolve_metadata_by_candidate_id(
    kind: MetadataKind,
    name: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<(Option<String>, Option<String>, Option<String>)> {
    let type_name = format!("{}.{}", kind.to_prefix(), name);
    let documentation = resolve_type_details(&type_name, metadata_lookup).and_then(|(_, doc)| doc);
    Some((
        Some(kind.to_russian_name().to_string()),
        documentation,
        None,
    ))
}

#[cfg(test)]
#[path = "completion/tests.rs"]
mod tests;
