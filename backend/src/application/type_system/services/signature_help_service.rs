use std::sync::Arc;

use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature};

use crate::system::LineIndex;

#[derive(Debug, Clone)]
pub struct SignatureHelpData {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: u32,
}

#[derive(Debug)]
struct CallContext {
    function_name: String,
    receiver_type: Option<String>,
    is_constructor: bool,
    call_start_line: u32,
    call_start_character: u32,
}

pub fn get_signature_help_v2(
    file_content: &str,
    line: u32,
    character: u32,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> Option<SignatureHelpData> {
    let call_context = find_call_context(file_content, line, character)?;

    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));

    let signature_info = get_signature_for_function_with_repository(
        &call_context.function_name,
        call_context.receiver_type.as_deref(),
        call_context.is_constructor,
        &deps.repository,
        Some(resolver.as_ref()),
    )?;

    let active_param = calculate_active_parameter(file_content, &call_context, line, character);
    let (label, parameters) = match signature_info {
        SignatureTarget::Method(signature) => build_signature_labels(&signature.name, &signature.params),
        SignatureTarget::Constructor(signature) => {
            let name = format!("\u{041D}\u{043E}\u{0432}\u{044B}\u{0439} {}", signature.type_name);
            build_signature_labels(&name, &signature.params)
        }
    };

    Some(SignatureHelpData {
        label,
        parameters,
        active_parameter: active_param,
    })
}

fn find_call_context(content: &str, line: u32, character: u32) -> Option<CallContext> {
    let index = LineIndex::new(content);
    let lines: Vec<&str> = content.lines().collect();
    let max_line = lines.len().saturating_sub(1);
    let search_until_line = (line as usize).min(max_line);

    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut in_string = false;
    let mut in_block_comment = false;

    for line_idx in 0..=search_until_line {
        let line_text = lines.get(line_idx)?;

        let end_char_idx = if line_idx == line as usize {
            let end_byte = index.utf16_column_to_byte(content, line_idx, character);
            line_text[..end_byte].chars().count()
        } else {
            line_text.chars().count()
        };

        let chars: Vec<char> = line_text.chars().collect();
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
    let line_text = lines.get(line_idx)?;
    let before_paren: String = line_text.chars().take(char_idx).collect();
    let (function_name, receiver_type, is_constructor) = extract_function_name(&before_paren)?;

    let byte_column = char_index_to_byte_column(line_text, char_idx);
    let call_start_character = index.byte_column_to_utf16(content, line_idx, byte_column);

    Some(CallContext {
        function_name,
        receiver_type,
        is_constructor,
        call_start_line: line_idx as u32,
        call_start_character,
    })
}

fn calculate_active_parameter(
    content: &str,
    context: &CallContext,
    line: u32,
    character: u32,
) -> u32 {
    let index = LineIndex::new(content);
    let lines: Vec<&str> = content.lines().collect();
    let mut param_index = 0;
    let mut paren_depth = 0;
    let mut in_string = false;
    let mut in_block_comment = false;

    for line_idx in context.call_start_line..=line {
        let line_text = match lines.get(line_idx as usize) {
            Some(value) => value,
            None => break,
        };

        let start_char_idx = if line_idx == context.call_start_line {
            let start_byte = index.utf16_column_to_byte(
                content,
                line_idx as usize,
                context.call_start_character.saturating_add(1),
            );
            line_text[..start_byte].chars().count()
        } else {
            0
        };

        let end_char_idx = if line_idx == line {
            let end_byte = index.utf16_column_to_byte(content, line_idx as usize, character);
            line_text[..end_byte].chars().count()
        } else {
            line_text.chars().count()
        };

        let chars: Vec<char> = line_text.chars().collect();
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

fn char_index_to_byte_column(line: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }

    match line.char_indices().nth(char_idx) {
        Some((byte_idx, _)) => byte_idx,
        None => line.len(),
    }
}

fn extract_function_name(text: &str) -> Option<(String, Option<String>, bool)> {
    let trimmed = text.trim_end();

    if let Some(constructor_name) = extract_constructor_name(trimmed) {
        return Some((constructor_name, None, true));
    }

    if let Some(dot_byte_pos) = trimmed.rfind('.') {
        let after_dot = trimmed[dot_byte_pos + 1..].trim_start();

        let method_name = after_dot
            .chars()
            .take_while(|c| is_identifier_char(*c))
            .collect::<String>();

        if !method_name.is_empty() {
            let receiver = trimmed[..dot_byte_pos].trim_end();
            let receiver_compact: String = receiver.chars().filter(|c| !c.is_whitespace()).collect();
            let receiver_type = if is_simple_receiver(&receiver_compact) {
                Some(receiver_compact)
            } else {
                None
            };
            return Some((method_name, receiver_type, false));
        }
    }

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

fn extract_constructor_name(text: &str) -> Option<String> {
    let mut iter = text.split_whitespace();
    let keyword = iter.next()?;
    if keyword.to_lowercase() != "\u{043D}\u{043E}\u{0432}\u{044B}\u{0439}" {
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

fn is_control_keyword(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "\u{0435}\u{0441}\u{043B}\u{0438}"
            | "\u{0438}\u{043D}\u{0430}\u{0447}\u{0435}\u{0435}\u{0441}\u{043B}\u{0438}"
            | "\u{043F}\u{043E}\u{043A}\u{0430}"
            | "\u{0434}\u{043B}\u{044F}"
            | "\u{043A}\u{0430}\u{0436}\u{0434}\u{043E}\u{0433}\u{043E}"
            | "\u{043F}\u{043E}\u{043F}\u{044B}\u{0442}\u{043A}\u{0430}"
            | "\u{0438}\u{0441}\u{043A}\u{043B}\u{044E}\u{0447}\u{0435}\u{043D}\u{0438}\u{0435}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{0435}\u{0441}\u{043B}\u{0438}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{0446}\u{0438}\u{043A}\u{043B}\u{0430}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{043F}\u{043E}\u{043F}\u{044B}\u{0442}\u{043A}\u{0438}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{043F}\u{0440}\u{043E}\u{0446}\u{0435}\u{0434}\u{0443}\u{0440}\u{044B}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{0444}\u{0443}\u{043D}\u{043A}\u{0446}\u{0438}\u{0438}"
            | "\u{0432}\u{043E}\u{0437}\u{0432}\u{0440}\u{0430}\u{0442}"
            | "\u{0432}\u{044B}\u{0431}\u{043E}\u{0440}"
            | "\u{043A}\u{043E}\u{0433}\u{0434}\u{0430}"
            | "\u{0438}\u{043D}\u{0430}\u{0447}\u{0435}"
    )
}

fn is_simple_receiver(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    text.chars().all(|c| c == '.' || is_identifier_char(c))
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || (c >= '\u{0410}' && c <= '\u{044F}')
        || c == '\u{0401}'
        || c == '\u{0451}'
}

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

enum SignatureTarget {
    Method(MethodSignature),
    Constructor(ConstructorSignature),
}

fn build_signature_labels(
    name: &str,
    params: &[bsl_shared::domain::types::ParameterInfo],
) -> (String, Vec<String>) {
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
        .map(|p| format!("{}: {}", p.name, p.type_name.as_deref().unwrap_or("Any")))
        .collect();

    (label, parameters)
}
