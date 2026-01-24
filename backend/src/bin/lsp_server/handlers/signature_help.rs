//! Signature Help handler for LSP
//!
//! Handles textDocument/signatureHelp requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::debug;

use bsl_backend::application::type_system;

#[cfg(test)]
use crate::converters::position::{char_to_utf16_index, utf16_to_char_index};
#[cfg(test)]
use bsl_shared::domain::repository::TypeRepository;
#[cfg(test)]
use bsl_shared::domain::resolver::TypeResolver;
#[cfg(test)]
use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature};
#[cfg(test)]
use bsl_shared::engine::AnalysisEngine;

/// Context of a function call
#[derive(Debug)]
#[cfg(test)]
pub struct CallContext {
    pub function_name: String,
    pub receiver_type: Option<String>,
    pub is_constructor: bool,
    pub call_start: Position,
}

/// Handle textDocument/signatureHelp request
#[cfg(test)]
pub async fn handle_signature_help(
    file_content: &str,
    position: Position,
    analysis_engine: Option<Arc<AnalysisEngine>>,
) -> Option<SignatureHelp> {
    debug!(
        "SignatureHelp requested at {}:{}",
        position.line, position.character
    );

    // Find call context
    let call_context = find_call_context(file_content, position)?;

    debug!(
        "Found call context: function={}, receiver={:?}",
        call_context.function_name, call_context.receiver_type
    );

    // Get signature from repository
    let signature_info = {
        let engine = analysis_engine?;
        let repo = engine.get_repository();
        let resolver = engine.get_resolver();
        get_signature_for_function_with_repository(
            &call_context.function_name,
            call_context.receiver_type.as_deref(),
            call_context.is_constructor,
            &repo,
            Some(resolver.as_ref()),
        )?
    };

    // Calculate active parameter
    let active_param = calculate_active_parameter(file_content, &call_context, position);

    // Build response
    Some(build_signature_help_response(signature_info, active_param))
}

pub async fn handle_signature_help_v2(
    file_content: Arc<str>,
    position: Position,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> Option<SignatureHelp> {
    debug!(
        "SignatureHelp v2 requested at {}:{}",
        position.line, position.character
    );

    let data = type_system::get_signature_help_v2(
        file_content.as_ref(),
        position.line,
        position.character,
        deps,
    )?;

    let parameters = data
        .parameters
        .into_iter()
        .map(|label| ParameterInformation {
            label: ParameterLabel::Simple(label),
            documentation: None,
        })
        .collect();

    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: data.label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(data.active_parameter),
        }],
        active_signature: Some(0),
        active_parameter: Some(data.active_parameter),
    })
}

/// Find function call context
#[cfg(test)]
pub fn find_call_context(content: &str, position: Position) -> Option<CallContext> {
    let lines: Vec<&str> = content.lines().collect();
    let max_line = if lines.is_empty() {
        return None;
    } else {
        lines.len() - 1
    };
    let search_until_line = position.line.min(max_line as u32) as usize;

    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut in_string = false;
    let mut in_block_comment = false;

    for line_idx in 0..=search_until_line {
        let line = lines.get(line_idx)?;

        let end_char_idx = if line_idx == position.line as usize {
            utf16_to_char_index(line, position.character as usize)?
        } else {
            line.chars().count()
        };

        let chars: Vec<char> = line.chars().collect();
        let mut char_idx = 0;

        while char_idx < end_char_idx {
            let ch = chars.get(char_idx).copied()?;
            let next = chars.get(char_idx + 1).copied();

            if in_string {
                if ch == '"' {
                    if next == Some('"') {
                        char_idx += 2;
                        continue;
                    }
                    in_string = false;
                }
                char_idx += 1;
                continue;
            }

            if in_block_comment {
                if ch == '*' && next == Some('/') {
                    in_block_comment = false;
                    char_idx += 2;
                    continue;
                }
                char_idx += 1;
                continue;
            }

            if ch == '/' && next == Some('/') {
                break;
            }

            if ch == '/' && next == Some('*') {
                in_block_comment = true;
                char_idx += 2;
                continue;
            }

            if ch == '"' {
                in_string = true;
                char_idx += 1;
                continue;
            }

            match ch {
                '(' => stack.push((line_idx, char_idx)),
                ')' => {
                    stack.pop();
                }
                _ => {}
            }

            char_idx += 1;
        }
    }

    let (line_idx, char_idx) = stack.pop()?;

    // Extract function name before parenthesis
    let line = lines.get(line_idx)?;
    let before_paren: String = line.chars().take(char_idx).collect();

    let (function_name, receiver_type, is_constructor) = extract_function_name(&before_paren)?;

    let utf16_char = char_to_utf16_index(line, char_idx);

    Some(CallContext {
        function_name,
        receiver_type,
        is_constructor,
        call_start: Position {
            line: line_idx as u32,
            character: utf16_char as u32,
        },
    })
}

/// Extract function name from text before parenthesis
#[cfg(test)]
fn extract_function_name(text: &str) -> Option<(String, Option<String>, bool)> {
    let trimmed = text.trim_end();

    if let Some(constructor_name) = extract_constructor_name(trimmed) {
        return Some((constructor_name, None, true));
    }

    // First search for dot (for object methods)
    if let Some(dot_byte_pos) = trimmed.rfind('.') {
        let after_dot = trimmed[dot_byte_pos + 1..].trim_start();

        let method_name = after_dot
            .chars()
            .take_while(|c| is_identifier_char(*c))
            .collect::<String>();

        if !method_name.is_empty() {
            let receiver = trimmed[..dot_byte_pos].trim_end();
            let receiver_compact: String =
                receiver.chars().filter(|c| !c.is_whitespace()).collect();
            let receiver_type = if is_simple_receiver(&receiver_compact) {
                Some(receiver_compact)
            } else {
                None
            };
            return Some((method_name, receiver_type, false));
        }
    }

    // Global function: extract last valid identifier
    let function_name = trimmed
        .chars()
        .rev()
        .take_while(|c| is_identifier_char(*c))
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    if !function_name.is_empty() {
        if is_control_keyword(&function_name) {
            return None;
        }
        Some((function_name, None, false))
    } else {
        None
    }
}

#[cfg(test)]
fn extract_constructor_name(text: &str) -> Option<String> {
    let mut iter = text.split_whitespace();
    let keyword = iter.next()?;
    if keyword.to_lowercase() != "новый" {
        return None;
    }
    let remainder: String = iter.collect::<Vec<_>>().join(" ");
    if remainder.is_empty() {
        return None;
    }
    let normalized: String = remainder.chars().filter(|c| !c.is_whitespace()).collect();
    if is_simple_receiver(&normalized) {
        Some(normalized)
    } else {
        None
    }
}

#[cfg(test)]
fn is_control_keyword(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "если"
            | "иначеесли"
            | "пока"
            | "для"
            | "каждого"
            | "попытка"
            | "исключение"
            | "конецесли"
            | "конеццикла"
            | "конецпопытки"
            | "конецпроцедуры"
            | "конецфункции"
            | "возврат"
            | "выбор"
            | "когда"
            | "иначе"
    )
}

#[cfg(test)]
fn is_simple_receiver(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    text.chars().all(|c| c == '.' || is_identifier_char(c))
}

#[cfg(test)]
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || ('\u{0410}'..='\u{044F}').contains(&c)
        || c == '\u{0401}'
        || c == '\u{0451}'
}

/// Calculate active parameter index
#[cfg(test)]
pub fn calculate_active_parameter(content: &str, context: &CallContext, position: Position) -> u32 {
    let lines: Vec<&str> = content.lines().collect();
    let mut param_index = 0;
    let mut paren_depth = 0;
    let mut in_string = false;
    let mut in_block_comment = false;

    for line_idx in context.call_start.line..=position.line {
        let line = match lines.get(line_idx as usize) {
            Some(l) => l,
            None => break,
        };
        let chars: Vec<char> = line.chars().collect();

        let start_char_idx = if line_idx == context.call_start.line {
            utf16_to_char_index(line, (context.call_start.character + 1) as usize).unwrap_or(0)
        } else {
            0
        };

        let end_char_idx = if line_idx == position.line {
            utf16_to_char_index(line, position.character as usize).unwrap_or(chars.len())
        } else {
            chars.len()
        };

        let mut char_idx = start_char_idx;
        while char_idx < end_char_idx {
            let ch = match chars.get(char_idx) {
                Some(ch) => *ch,
                None => break,
            };
            let next = chars.get(char_idx + 1).copied();

            if in_string {
                if ch == '"' {
                    if next == Some('"') {
                        char_idx += 2;
                        continue;
                    }
                    in_string = false;
                }
                char_idx += 1;
                continue;
            }

            if in_block_comment {
                if ch == '*' && next == Some('/') {
                    in_block_comment = false;
                    char_idx += 2;
                    continue;
                }
                char_idx += 1;
                continue;
            }

            if ch == '/' && next == Some('/') {
                break;
            }

            if ch == '/' && next == Some('*') {
                in_block_comment = true;
                char_idx += 2;
                continue;
            }

            if ch == '"' {
                in_string = true;
                char_idx += 1;
                continue;
            }

            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    }
                }
                ',' if paren_depth == 0 => {
                    param_index += 1;
                }
                _ => {}
            }

            char_idx += 1;
        }
    }

    param_index
}

/// Get function signature from TypeRepository
#[cfg(test)]
fn get_signature_for_function_with_repository(
    function_name: &str,
    receiver_type: Option<&str>,
    is_constructor: bool,
    repository: &Arc<dyn TypeRepository>,
    resolver: Option<&TypeResolver>,
) -> Option<SignatureTarget> {
    if is_constructor {
        return repository
            .find_constructor(function_name)
            .map(SignatureTarget::Constructor);
    }

    let owner_type = receiver_type.and_then(|expr| resolve_receiver_type(expr, resolver));
    repository
        .find_method_signature(owner_type.as_deref(), function_name)
        .map(SignatureTarget::Method)
}

#[cfg(test)]
fn resolve_receiver_type(expr: &str, resolver: Option<&TypeResolver>) -> Option<String> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }

    let resolver = resolver?;
    let resolution = resolver.resolve_expression_sync(trimmed);
    if resolution.is_unknown() || resolution.is_dynamic() {
        return None;
    }

    Some(resolution.type_name())
}

#[cfg(test)]
enum SignatureTarget {
    Method(MethodSignature),
    Constructor(ConstructorSignature),
}

/// Build LSP SignatureHelp response
#[cfg(test)]
fn build_signature_help_response(signature: SignatureTarget, active_param: u32) -> SignatureHelp {
    match signature {
        SignatureTarget::Method(signature) => build_method_signature_help(signature, active_param),
        SignatureTarget::Constructor(signature) => {
            build_constructor_signature_help(signature, active_param)
        }
    }
}

#[cfg(test)]
fn build_method_signature_help(signature: MethodSignature, active_param: u32) -> SignatureHelp {
    let (label, parameters) = build_signature_labels(&signature.name, &signature.params);
    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(active_param),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_param),
    }
}

#[cfg(test)]
fn build_constructor_signature_help(
    signature: ConstructorSignature,
    active_param: u32,
) -> SignatureHelp {
    let name = format!("Новый {}", signature.type_name);
    let (label, parameters) = build_signature_labels(&name, &signature.params);
    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(active_param),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_param),
    }
}

#[cfg(test)]
fn build_signature_labels(
    name: &str,
    params: &[bsl_shared::domain::types::ParameterInfo],
) -> (String, Vec<ParameterInformation>) {
    let params_str = params
        .iter()
        .map(|p| {
            let type_str = p.type_name.as_deref().unwrap_or("Any");
            if p.is_optional {
                format!("[{}: {}]", p.name, type_str)
            } else {
                format!("{}: {}", p.name, type_str)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let label = format!("{}({})", name, params_str);

    let parameters = params
        .iter()
        .map(|p| {
            let param_label = format!("{}: {}", p.name, p.type_name.as_deref().unwrap_or("Any"));

            ParameterInformation {
                label: ParameterLabel::Simple(param_label),
                documentation: None,
            }
        })
        .collect();

    (label, parameters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::signature_index::{
        ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
    };
    use bsl_shared::domain::types::{ParameterInfo, RawDataSource, RawTypeData};
    use bsl_shared::engine::AnalysisEngine;
    use bsl_shared::TypeRepository;
    use bsl_shared::TypeResolver;

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

    fn assert_snapshot(name: &str, value: &serde_json::Value) {
        let path = golden_path(name);
        let json = serde_json::to_string_pretty(value).expect("snapshot json");
        if std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1") {
            fs::create_dir_all(path.parent().expect("golden dir")).expect("create golden dir");
            fs::write(&path, json).expect("write golden");
            return;
        }
        let expected = fs::read_to_string(&path).expect("read golden");
        assert_eq!(expected, json);
    }

    struct TestDeps {
        engine: Arc<AnalysisEngine>,
        deps: Arc<bsl_analysis_v2::SemanticDeps>,
    }

    fn create_test_deps() -> TestDeps {
        let repository_impl = Arc::new(InMemoryTypeRepository::new());
        let raw_type = RawTypeData {
            name: "Массив".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        };
        repository_impl
            .load_types(vec![raw_type])
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
        repository_impl.set_signature_index(index);

        let repository = repository_impl as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let engine = Arc::new(AnalysisEngine::new(resolver.clone(), repository.clone()));
        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(resolver),
            repository,
            platform_signatures_loaded: false,
        });

        TestDeps { engine, deps }
    }

    #[tokio::test]
    async fn m5_signature_help_snapshot() {
        let content = read_fixture("m5_signature_help.bsl");
        let test_deps = create_test_deps();

        let constructor_pos = find_position(&content, "Новый Массив(1, ");
        let constructor_legacy =
            handle_signature_help(&content, constructor_pos, Some(test_deps.engine.clone()))
                .await
                .expect("constructor signature help (legacy)");
        let constructor_v2 = handle_signature_help_v2(
            Arc::from(content.clone()),
            constructor_pos,
            test_deps.deps.clone(),
        )
        .await
        .expect("constructor signature help (v2)");

        let method_pos = find_position(&content, "Массив.Добавить(1, ");
        let method_legacy = handle_signature_help(&content, method_pos, Some(test_deps.engine))
            .await
            .expect("method signature help (legacy)");
        let method_v2 = handle_signature_help_v2(Arc::from(content), method_pos, test_deps.deps)
            .await
            .expect("method signature help (v2)");

        assert_eq!(
            constructor_legacy.active_parameter,
            constructor_v2.active_parameter
        );
        assert_eq!(
            constructor_legacy
                .signatures
                .first()
                .map(|sig| sig.label.clone()),
            constructor_v2
                .signatures
                .first()
                .map(|sig| sig.label.clone())
        );
        assert_eq!(method_legacy.active_parameter, method_v2.active_parameter);
        assert_eq!(
            method_legacy
                .signatures
                .first()
                .map(|sig| sig.label.clone()),
            method_v2.signatures.first().map(|sig| sig.label.clone())
        );

        let snapshot = serde_json::json!({
            "constructor": {
                "label": constructor_legacy.signatures.first().map(|sig| sig.label.clone()),
                "activeParameter": constructor_legacy.active_parameter,
            },
            "method": {
                "label": method_legacy.signatures.first().map(|sig| sig.label.clone()),
                "activeParameter": method_legacy.active_parameter,
            },
        });

        assert_snapshot("m5_signature_help.json", &snapshot);
    }
}
