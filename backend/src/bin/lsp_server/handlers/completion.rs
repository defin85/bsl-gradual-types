//! Completion handler for LSP
//!
//! Handles textDocument/completion requests.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::error;

use bsl_backend::application::type_system::{
    build_call_snippet, resolve_method_completion, resolve_type_details,
};
use bsl_backend::application::CompletionStats;
use bsl_backend::application::{
    get_completion_with_semantic_program_snapshot, get_completion_with_semantic_program_snapshot_v2,
};
use bsl_backend::system::IndexSnapshot;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::{MethodSignature, SignatureSource};
use bsl_shared::domain::types::{MetadataKind, TypeResolution};
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::ir::SemanticProgram;

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
pub async fn handle_completion_v2(
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    file_uri: &Url,
    index_snapshot: &IndexSnapshot,
    snippet_support: bool,
) -> Option<CompletionResponseWithStats> {
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let completion = match parse_result {
        Some(parse_result) => {
            get_completion_with_semantic_program_snapshot_v2(
                file_content.as_ref(),
                position.line,
                position.character,
                Some(file_uri.as_str()),
                index_snapshot,
                &metadata_lookup,
                file_path.as_ref(),
                resolver.as_ref(),
                ir_program,
                parse_result,
            )
            .await
        }
        None => {
            get_completion_with_semantic_program_snapshot(
                file_content.as_ref(),
                position.line,
                position.character,
                Some(file_uri.as_str()),
                index_snapshot,
                &metadata_lookup,
                file_path.as_ref(),
                resolver.as_ref(),
                ir_program,
            )
            .await
        }
    };

    match completion {
        Ok(result) => {
            let lsp_completions: Vec<CompletionItem> = result
                .items
                .into_iter()
                .map(|candidate| {
                    to_lsp_completion(
                        candidate.item,
                        candidate.owner_type,
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

    let resolved = if let Some(candidate_id) = parse_candidate_id(&item) {
        resolve_by_candidate_id(
            &candidate_id,
            deps.as_ref(),
            &metadata_lookup,
            snippet_support,
        )
    } else {
        let (kind, owner_type) = parse_completion_data(&item);
        resolve_legacy(
            &item,
            deps.as_ref(),
            &metadata_lookup,
            snippet_support,
            kind.as_deref(),
            owner_type.as_deref(),
        )
    };

    if let Some((detail, documentation, insert_text)) = resolved {
        item.detail = detail;
        if let Some(doc) = documentation {
            item.documentation = Some(Documentation::String(doc));
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
    origin_sources: Vec<u8>,
    snippet_support: bool,
    deps: Option<&bsl_analysis_v2::SemanticDeps>,
) -> CompletionItem {
    let kind_tag = completion_kind_tag(&item);
    let candidate_id = build_candidate_id(
        kind_tag,
        &item,
        owner_type.as_deref(),
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

    let data = Some(json!({
        "kind": kind_tag,
        "owner_type": owner_type,
        "origin_sources": origin_sources,
        "candidate_id": candidate_id,
    }));

    CompletionItem {
        label: item.label,
        kind,
        detail: None,
        documentation: None,
        insert_text,
        insert_text_format,
        filter_text: item.filter_text,
        sort_text: item.sort_text,
        data,
        ..Default::default()
    }
}

fn build_candidate_id(
    kind_tag: &str,
    item: &bsl_shared::domain::CompletionItem,
    owner_type: Option<&str>,
    origin_sources: &[u8],
    deps: Option<&bsl_analysis_v2::SemanticDeps>,
) -> CompletionCandidateId {
    let payload = if kind_tag.starts_with("metadata.") {
        CompletionCandidateIdPayload::Metadata {
            kind: item.kind.metadata_kind().unwrap_or(MetadataKind::Unknown),
            name: item.label.clone(),
        }
    } else {
        match kind_tag {
            "method" => match owner_type {
                Some(owner) => {
                    let sig_hash = deps
                        .and_then(|deps| deps.signature_index.find_method(owner, &item.label))
                        .map(method_signature_hash);

                    CompletionCandidateIdPayload::Method {
                        owner_type: owner.to_string(),
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
                    owner_type: owner.to_string(),
                    name: item.label.clone(),
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
        CompletionCandidateIdPayload::Property { owner_type, name } => {
            resolve_property_by_candidate_id(owner_type, name, metadata_lookup)
        }
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
    let signature = sig_hash
        .and_then(|expected| {
            deps.signature_index
                .find_methods(owner_type, name)
                .into_iter()
                .find(|signature| method_signature_hash(signature) == expected)
                .cloned()
        })
        .or_else(|| deps.signature_index.find_method(owner_type, name).cloned());

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

    resolve_method_completion(owner_type, name, metadata_lookup, snippet_support)
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
    let resolution = TypeResolution::explicit(owner_type);
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

fn resolve_legacy(
    item: &CompletionItem,
    deps: &bsl_analysis_v2::SemanticDeps,
    metadata_lookup: &TypeMetadataLookup,
    snippet_support: bool,
    kind: Option<&str>,
    owner_type: Option<&str>,
) -> Option<(Option<String>, Option<String>, Option<String>)> {
    match (kind, owner_type) {
        (Some("method"), Some(owner)) => {
            if let Some(signature) = deps
                .repository
                .find_method_signature(Some(owner), &item.label)
            {
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
            } else {
                resolve_method_completion(owner, &item.label, metadata_lookup, snippet_support)
                    .map(|details| (details.detail, details.documentation, details.insert_text))
            }
        }
        (Some("property"), Some(owner)) => {
            resolve_property_by_candidate_id(owner, &item.label, metadata_lookup)
        }
        (Some("function"), _) => deps
            .repository
            .find_method_signature(None, &item.label)
            .map(|signature| {
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

                (detail, documentation, insert_text)
            }),
        (Some("type"), _) => resolve_type_details(&item.label, metadata_lookup)
            .map(|(detail, documentation)| (detail, documentation, None)),
        _ => resolve_type_details(&item.label, metadata_lookup)
            .map(|(detail, documentation)| (detail, documentation, None)),
    }
}

fn completion_kind_tag(item: &bsl_shared::domain::CompletionItem) -> &'static str {
    use bsl_shared::domain::CompletionKind::*;

    match item.kind {
        Method => "method",
        Property => "property",
        Function => "function",
        Keyword => "keyword",
        Type | Class | Struct => "type",
        _ => metadata_completion_kind_tag(item).unwrap_or("other"),
    }
}

fn metadata_completion_kind_tag(item: &bsl_shared::domain::CompletionItem) -> Option<&'static str> {
    use bsl_shared::domain::CompletionKind::*;

    let is_metadata_item = match item.kind {
        Catalog
        | Document
        | MetadataUnknown
        | Report
        | DataProcessor
        | Register
        | InformationRegister
        | AccumulationRegister
        | AccountingRegister
        | CalculationRegister
        | ChartOfAccounts
        | ChartOfCharacteristicTypes
        | ChartOfCalculationTypes
        | BusinessProcess
        | Task
        | ExchangePlan
        | CommonModule
        | Role
        | Subsystem
        | Language => true,
        Enum | Constant => item.detail.is_some(),
        _ => false,
    };

    if !is_metadata_item {
        return None;
    }

    Some(match item.kind {
        Catalog => "metadata.catalog",
        Document => "metadata.document",
        MetadataUnknown => "metadata.unknown",
        Report => "metadata.report",
        DataProcessor => "metadata.data_processor",
        Register => "metadata.register",
        InformationRegister => "metadata.information_register",
        AccumulationRegister => "metadata.accumulation_register",
        AccountingRegister => "metadata.accounting_register",
        CalculationRegister => "metadata.calculation_register",
        ChartOfAccounts => "metadata.chart_of_accounts",
        ChartOfCharacteristicTypes => "metadata.chart_of_characteristic_types",
        ChartOfCalculationTypes => "metadata.chart_of_calculation_types",
        BusinessProcess => "metadata.business_process",
        Task => "metadata.task",
        ExchangePlan => "metadata.exchange_plan",
        Constant => "metadata.constant",
        CommonModule => "metadata.common_module",
        Role => "metadata.role",
        Subsystem => "metadata.subsystem",
        Language => "metadata.language",
        Enum => "metadata.enum",
        _ => return None,
    })
}

fn map_completion_kind(kind: bsl_shared::domain::CompletionKind) -> Option<CompletionItemKind> {
    use bsl_shared::domain::CompletionKind::*;
    Some(match kind {
        Method => CompletionItemKind::METHOD,
        Function => CompletionItemKind::FUNCTION,
        Constructor => CompletionItemKind::CONSTRUCTOR,
        Field => CompletionItemKind::FIELD,
        Variable => CompletionItemKind::VARIABLE,
        Class | Type => CompletionItemKind::CLASS,
        Interface => CompletionItemKind::INTERFACE,
        Module | Global => CompletionItemKind::MODULE,
        Property => CompletionItemKind::PROPERTY,
        Unit => CompletionItemKind::UNIT,
        Value => CompletionItemKind::VALUE,
        Enum => CompletionItemKind::ENUM,
        EnumMember => CompletionItemKind::ENUM_MEMBER,
        Keyword => CompletionItemKind::KEYWORD,
        Snippet => CompletionItemKind::SNIPPET,
        Color => CompletionItemKind::COLOR,
        File => CompletionItemKind::FILE,
        Reference => CompletionItemKind::REFERENCE,
        Folder => CompletionItemKind::FOLDER,
        Constant => CompletionItemKind::CONSTANT,
        Struct => CompletionItemKind::STRUCT,
        Event => CompletionItemKind::EVENT,
        Operator => CompletionItemKind::OPERATOR,
        TypeParameter => CompletionItemKind::TYPE_PARAMETER,
        Text => CompletionItemKind::TEXT,
        Catalog => CompletionItemKind::CLASS,
        Document => CompletionItemKind::FILE,
        MetadataUnknown => CompletionItemKind::TEXT,
        Report => CompletionItemKind::SNIPPET,
        DataProcessor => CompletionItemKind::CONSTRUCTOR,
        Register => CompletionItemKind::STRUCT,
        InformationRegister => CompletionItemKind::EVENT,
        AccumulationRegister => CompletionItemKind::UNIT,
        AccountingRegister => CompletionItemKind::VALUE,
        CalculationRegister => CompletionItemKind::OPERATOR,
        ChartOfAccounts => CompletionItemKind::ENUM_MEMBER,
        ChartOfCharacteristicTypes => CompletionItemKind::TYPE_PARAMETER,
        ChartOfCalculationTypes => CompletionItemKind::INTERFACE,
        BusinessProcess => CompletionItemKind::FIELD,
        Task => CompletionItemKind::PROPERTY,
        ExchangePlan => CompletionItemKind::REFERENCE,
        CommonModule => CompletionItemKind::MODULE,
        Role => CompletionItemKind::COLOR,
        Subsystem => CompletionItemKind::FOLDER,
        Language => CompletionItemKind::KEYWORD,
    })
}

fn parse_completion_data(item: &CompletionItem) -> (Option<String>, Option<String>) {
    let data = match item.data.as_ref() {
        Some(value) => value,
        None => return (None, None),
    };

    let kind = data
        .get("kind")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let owner_type = data
        .get("owner_type")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    (kind, owner_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use bsl_backend::system::IntellisenseIndexStore;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::signature_index::{
        ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
    };
    use bsl_shared::domain::types::{ParameterInfo, RawDataSource, RawPropertyData, RawTypeData};
    use bsl_shared::TypeRepository;
    use bsl_shared::TypeResolver;
    use tower_lsp::lsp_types::Url;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    fn golden_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("golden")
            .join(name)
    }

    fn read_fixture(name: &str) -> String {
        fs::read_to_string(fixture_path(name)).expect("fixture read")
    }

    fn find_position(content: &str, marker: &str) -> Position {
        let byte_index = content.find(marker).expect("marker not found");
        let before = &content[..byte_index + marker.len()];
        let line = before.lines().count() - 1;
        let last_line = before.lines().last().unwrap_or("");
        let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>();
        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    struct TestEnv {
        index: Arc<IntellisenseIndexStore>,
        deps: Arc<bsl_analysis_v2::SemanticDeps>,
    }

    fn create_test_env() -> TestEnv {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        let raw_type = RawTypeData {
            name: "Массив".to_string(),
            source: RawDataSource::Platform,
            properties: vec![RawPropertyData {
                name: "Длина".to_string(),
                prop_type: "Число".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        };
        let metadata_type = RawTypeData {
            name: "Документы.ТестДок".to_string(),
            description: "Описание тестового документа".to_string(),
            source: RawDataSource::Configuration,
            ..Default::default()
        };
        repository_impl
            .load_types(vec![raw_type, metadata_type])
            .expect("load types");

        let mut index = SignatureIndex::new();
        let method = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![
                ParameterInfo {
                    name: "Элемент".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                },
                ParameterInfo {
                    name: "Позиция".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: true,
                    default_value: None,
                    description: None,
                },
            ],
            Some("Булево".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            Default::default(),
        );
        index.add_platform_method(bsl_shared::domain::type_id::TypeId::new("Массив"), method);
        index.add_constructor(
            bsl_shared::domain::type_id::TypeId::new("Массив"),
            ConstructorSignature {
                type_name: "Массив".to_string(),
                params: vec![ParameterInfo {
                    name: "Размер".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: true,
                    default_value: None,
                    description: None,
                }],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );

        let global_function = MethodSignature::new(
            "Дубль".to_string(),
            None,
            vec![ParameterInfo {
                name: "Значение".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            Some("Число".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            Default::default(),
        );
        index.add_global_function(
            bsl_shared::domain::type_id::TypeId::new("Дубль"),
            global_function,
        );
        repository_impl.set_signature_index(index);

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let index = Arc::new(IntellisenseIndexStore::new("test", "test"));
        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(resolver),
            repository,
        });

        TestEnv { index, deps }
    }

    fn snapshot_path(name: &str) -> PathBuf {
        golden_path(name)
    }

    fn assert_snapshot(name: &str, value: &serde_json::Value) {
        let path = snapshot_path(name);
        let json = serde_json::to_string_pretty(value).expect("snapshot json");
        if std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1") {
            fs::create_dir_all(path.parent().expect("golden dir")).expect("create golden dir");
            fs::write(&path, json).expect("write golden");
            return;
        }
        let expected = fs::read_to_string(&path).expect("read golden");
        assert_eq!(expected, json);
    }

    fn completion_kind(kind: Option<CompletionItemKind>) -> Option<&'static str> {
        match kind {
            Some(CompletionItemKind::METHOD) => Some("METHOD"),
            Some(CompletionItemKind::FUNCTION) => Some("FUNCTION"),
            Some(CompletionItemKind::CLASS) => Some("CLASS"),
            Some(CompletionItemKind::KEYWORD) => Some("KEYWORD"),
            Some(CompletionItemKind::PROPERTY) => Some("PROPERTY"),
            _ => None,
        }
    }

    fn insert_text_format(format: Option<InsertTextFormat>) -> Option<&'static str> {
        match format {
            Some(InsertTextFormat::SNIPPET) => Some("SNIPPET"),
            Some(InsertTextFormat::PLAIN_TEXT) => Some("PLAIN_TEXT"),
            _ => None,
        }
    }

    fn completion_items_snapshot(items: &[CompletionItem]) -> serde_json::Value {
        serde_json::Value::Array(
            items
                .iter()
                .map(|item| {
                    serde_json::json!({
                        "label": item.label,
                        "kind": completion_kind(item.kind),
                        "sortText": item.sort_text,
                        "filterText": item.filter_text,
                        "insertText": item.insert_text,
                        "insertTextFormat": insert_text_format(item.insert_text_format),
                        "data": item.data,
                    })
                })
                .collect(),
        )
    }

    fn extract_items(response: CompletionResponse) -> Vec<CompletionItem> {
        match response {
            CompletionResponse::List(list) => list.items,
            CompletionResponse::Array(list) => list,
        }
    }

    #[test]
    fn metadata_completion_kinds_have_unique_lsp_kinds() {
        use bsl_shared::domain::CompletionKind::*;

        let metadata_kinds = [
            MetadataUnknown,
            Catalog,
            Document,
            Register,
            Report,
            DataProcessor,
            Enum,
            ChartOfAccounts,
            ChartOfCharacteristicTypes,
            ChartOfCalculationTypes,
            InformationRegister,
            AccumulationRegister,
            AccountingRegister,
            CalculationRegister,
            BusinessProcess,
            Task,
            ExchangePlan,
            Constant,
            CommonModule,
            Role,
            Subsystem,
            Language,
        ];

        let mut seen: Vec<CompletionItemKind> = Vec::new();
        for kind in metadata_kinds {
            let mapped = map_completion_kind(kind).expect("metadata kind should map");
            assert!(
                !seen.contains(&mapped),
                "Duplicate LSP kind mapping for metadata completion kind: {:?} -> {:?}",
                kind,
                mapped
            );
            seen.push(mapped);
        }
    }

    #[test]
    fn metadata_completion_items_have_granular_kind_in_data() {
        let item = bsl_shared::domain::CompletionItem::with_details(
            "Регистр".to_string(),
            bsl_shared::domain::CompletionKind::InformationRegister,
            Some("Регистр сведений".to_string()),
            None,
        );
        let lsp_item = to_lsp_completion(item, None, vec![], false, None);
        let kind = lsp_item
            .data
            .as_ref()
            .and_then(|value| value.get("kind"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        assert_eq!(kind, "metadata.information_register");
    }

    #[test]
    fn method_completion_items_keep_method_kind_in_data() {
        let item = bsl_shared::domain::CompletionItem::new(
            "Добавить".to_string(),
            bsl_shared::domain::CompletionKind::Method,
        );
        let lsp_item = to_lsp_completion(item, None, vec![], false, None);
        let kind = lsp_item
            .data
            .as_ref()
            .and_then(|value| value.get("kind"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        assert_eq!(kind, "method");
    }

    #[test]
    fn property_completion_items_keep_property_kind_in_data() {
        let item = bsl_shared::domain::CompletionItem::new(
            "Длина".to_string(),
            bsl_shared::domain::CompletionKind::Property,
        );
        let lsp_item = to_lsp_completion(item, Some("Массив".to_string()), vec![0], false, None);
        let kind = lsp_item
            .data
            .as_ref()
            .and_then(|value| value.get("kind"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        assert_eq!(kind, "property");

        let candidate_id = parse_candidate_id(&lsp_item).expect("candidate_id");
        match candidate_id.payload {
            CompletionCandidateIdPayload::Property { owner_type, name } => {
                assert_eq!(owner_type, "Массив");
                assert_eq!(name, "Длина");
            }
            other => panic!("expected property candidate_id, got {:?}", other),
        }
    }

    fn build_v2_ir(
        content: &str,
        uri: &Url,
        deps: Arc<bsl_analysis_v2::SemanticDeps>,
    ) -> (Arc<str>, Arc<str>, Arc<SemanticProgram>) {
        let mut host = bsl_analysis_v2::AnalysisHostV2::default();
        host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
            deps_id: bsl_analysis_v2::DepsSnapshotId::from_hash("test"),
            deps: deps.clone(),
        });

        let path = uri
            .to_file_path()
            .ok()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| uri.to_string());
        let file_id = bsl_analysis_v2::FileId(1);
        host.apply_change(bsl_analysis_v2::Change::SetFile {
            file_id,
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from(path),
        });

        let analysis = host.analysis();
        let file_content = analysis
            .file_text(file_id)
            .ok()
            .flatten()
            .expect("file_text");
        let file_path = analysis
            .file_path(file_id)
            .ok()
            .flatten()
            .expect("file_path");
        let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

        (file_content, file_path, ir_program)
    }

    #[tokio::test]
    async fn m5_completion_v2_is_deterministic() {
        let content = read_fixture("m5_snippets_resolve.bsl");
        let position = find_position(&content, "Массив.");
        let uri = Url::parse("file:///m5_snippets_resolve.bsl").expect("url");
        let env = create_test_env();
        let index = env.index.clone();
        let index_snapshot = index.snapshot();
        let deps = env.deps.clone();

        let (file_content, file_path, ir_program) = build_v2_ir(&content, &uri, deps.clone());
        let v2 = handle_completion_v2(
            file_content.clone(),
            file_path.clone(),
            ir_program.clone(),
            None,
            deps.clone(),
            position,
            &uri,
            &index_snapshot,
            true,
        )
        .await
        .expect("completion v2");
        let v2_items = extract_items(v2.response);
        assert!(
            v2_items.iter().any(|item| item.label == "Добавить"),
            "expected v2 completion to contain 'Добавить'"
        );

        let v2_snapshot = completion_items_snapshot(&v2_items);

        // Determinism smoke: same input -> same output twice.
        let v2_second = handle_completion_v2(
            file_content,
            file_path,
            ir_program,
            None,
            deps,
            position,
            &uri,
            &index_snapshot,
            true,
        )
        .await
        .expect("completion v2 (second)");
        let v2_second_items = extract_items(v2_second.response);
        let v2_second_snapshot = completion_items_snapshot(&v2_second_items);
        assert_eq!(v2_snapshot, v2_second_snapshot);
    }

    #[tokio::test]
    async fn m5_completion_resolve_snippets_snapshot() {
        let content = read_fixture("m5_snippets_resolve.bsl");
        let position = find_position(&content, "Массив.");
        let uri = Url::parse("file:///m5_snippets_resolve.bsl").expect("url");
        let env = create_test_env();
        let index_snapshot = env.index.snapshot();
        let deps = env.deps;

        let (file_content, file_path, ir_program) = build_v2_ir(&content, &uri, deps.clone());
        let response = handle_completion_v2(
            file_content,
            file_path,
            ir_program,
            None,
            deps.clone(),
            position,
            &uri,
            &index_snapshot,
            true,
        )
        .await
        .expect("completion");

        let items = match response.response {
            CompletionResponse::List(list) => list.items,
            CompletionResponse::Array(list) => list,
        };

        let item = items
            .into_iter()
            .find(|entry| entry.label == "Добавить")
            .expect("Добавить completion");

        let resolved_true = handle_completion_resolve(item.clone(), Some(deps.clone()), true).await;
        let resolved_false = handle_completion_resolve(item.clone(), Some(deps), false).await;

        let snapshot = serde_json::json!({
            "completion": {
                "label": item.label,
                "kind": completion_kind(item.kind),
                "insertText": item.insert_text,
                "insertTextFormat": insert_text_format(item.insert_text_format),
            },
            "resolveSnippetSupportTrue": {
                "label": resolved_true.label,
                "detail": resolved_true.detail,
                "hasDocumentation": resolved_true.documentation.is_some(),
                "insertText": resolved_true.insert_text,
                "insertTextFormat": insert_text_format(resolved_true.insert_text_format),
            },
            "resolveSnippetSupportFalse": {
                "label": resolved_false.label,
                "detail": resolved_false.detail,
                "hasDocumentation": resolved_false.documentation.is_some(),
                "insertText": resolved_false.insert_text,
                "insertTextFormat": insert_text_format(resolved_false.insert_text_format),
            },
        });

        assert_snapshot("m5_completion_resolve_snippets.json", &snapshot);
    }

    #[tokio::test]
    async fn m6_completion_resolve_uses_candidate_id_for_function_origin() {
        let env = create_test_env();
        let deps = env.deps.clone();

        let file_symbol = to_lsp_completion(
            bsl_shared::domain::CompletionItem::new(
                "Дубль".to_string(),
                bsl_shared::domain::CompletionKind::Function,
            ),
            None,
            vec![0],
            false,
            Some(deps.as_ref()),
        );
        let module_symbol = to_lsp_completion(
            bsl_shared::domain::CompletionItem::new(
                "Дубль".to_string(),
                bsl_shared::domain::CompletionKind::Function,
            ),
            None,
            vec![1],
            false,
            Some(deps.as_ref()),
        );

        let file_resolved = handle_completion_resolve(file_symbol, Some(deps.clone()), false).await;
        let module_resolved = handle_completion_resolve(module_symbol, Some(deps), false).await;

        assert_eq!(
            file_resolved.detail, None,
            "file-level symbol should not resolve to global signature"
        );
        assert_eq!(
            module_resolved.detail.as_deref(),
            Some("Число"),
            "module/global function should resolve via SignatureIndex"
        );
    }

    #[tokio::test]
    async fn m6_completion_resolve_uses_candidate_id_for_property() {
        let env = create_test_env();
        let deps = env.deps.clone();

        let item = to_lsp_completion(
            bsl_shared::domain::CompletionItem::new(
                "Длина".to_string(),
                bsl_shared::domain::CompletionKind::Property,
            ),
            Some("Массив".to_string()),
            vec![0],
            false,
            Some(deps.as_ref()),
        );

        let resolved = handle_completion_resolve(item, Some(deps), false).await;
        assert_eq!(resolved.detail.as_deref(), Some("Число"));
    }

    #[tokio::test]
    async fn m6_completion_resolve_uses_candidate_id_for_metadata() {
        let env = create_test_env();
        let deps = env.deps.clone();

        let item = to_lsp_completion(
            bsl_shared::domain::CompletionItem::new(
                "ТестДок".to_string(),
                bsl_shared::domain::CompletionKind::Document,
            ),
            None,
            vec![2],
            false,
            Some(deps.as_ref()),
        );

        let resolved = handle_completion_resolve(item, Some(deps), false).await;
        assert_eq!(resolved.detail.as_deref(), Some("Документ"));
        assert!(resolved.documentation.is_some());
    }

    #[tokio::test]
    async fn m6_completion_resolve_dedup_sources_prefers_local_function() {
        let env = create_test_env();
        let deps = env.deps.clone();

        let deduped = to_lsp_completion(
            bsl_shared::domain::CompletionItem::new(
                "Дубль".to_string(),
                bsl_shared::domain::CompletionKind::Function,
            ),
            None,
            vec![0, 1],
            false,
            Some(deps.as_ref()),
        );

        let resolved = handle_completion_resolve(deduped, Some(deps), false).await;
        assert_eq!(
            resolved.detail, None,
            "deduped local+module function should not resolve to global signature"
        );
    }

    #[tokio::test]
    async fn m6_completion_resolve_legacy_fallback_works_without_candidate_id() {
        let env = create_test_env();
        let deps = env.deps.clone();

        let mut legacy = to_lsp_completion(
            bsl_shared::domain::CompletionItem::new(
                "Добавить".to_string(),
                bsl_shared::domain::CompletionKind::Method,
            ),
            Some("Массив".to_string()),
            vec![0],
            false,
            Some(deps.as_ref()),
        );
        if let Some(value) = legacy.data.as_mut() {
            if let Some(obj) = value.as_object_mut() {
                obj.remove("candidate_id");
            }
        }

        let resolved = handle_completion_resolve(legacy, Some(deps), false).await;
        assert_eq!(
            resolved.detail.as_deref(),
            Some("Булево"),
            "legacy resolve should still work via kind/owner_type"
        );
    }
}
