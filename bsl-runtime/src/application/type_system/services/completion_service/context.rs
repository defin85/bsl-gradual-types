use super::super::completion_target::extract_member_access_receiver_spans;
use super::*;

/// Context for auto-completion.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub current_word: String,
    pub member_access: bool,
    pub member_base: Option<String>,
    pub trigger_char: Option<char>,
    pub can_add_statements: bool,
    pub expects_type: bool,
    pub can_add_functions: bool,
}

#[allow(dead_code)]
pub(super) fn analyze_completion_context(
    content: &str,
    line: u32,
    column: u32,
) -> CompletionContext {
    analyze_completion_context_with_trigger_hint(content, line, column, None)
}

pub(super) fn analyze_completion_context_with_trigger_hint(
    content: &str,
    line: u32,
    column: u32,
    trigger_char_hint: Option<char>,
) -> CompletionContext {
    let lines: Vec<&str> = content.lines().collect();
    let line_index = line as usize;

    // Get current line and prefix
    let (_current_line, line_prefix_raw, cursor_char) = if line_index < lines.len() {
        let line_content = lines[line_index];
        // Convert UTF-16 offset -> UTF-8 byte offset
        let column_index = utf16_to_byte_offset(line_content, column);
        let line_prefix_raw = line_content.get(..column_index).unwrap_or(line_content);
        let cursor_char = line_content
            .get(column_index..)
            .and_then(|tail| tail.chars().next());
        (line_content, line_prefix_raw, cursor_char)
    } else {
        ("", "", None)
    };

    let in_string_or_comment = is_in_string_or_comment(line_prefix_raw);

    // Some clients request completion with cursor positioned on '.' itself.
    // Treat this as member-access context to avoid falling back to keyword completion.
    let effective_prefix_raw = if !in_string_or_comment {
        if let Some(cursor_char) = cursor_char.filter(|ch| *ch == '.' || *ch == '(') {
            format!("{line_prefix_raw}{cursor_char}")
        } else {
            line_prefix_raw.to_string()
        }
    } else {
        line_prefix_raw.to_string()
    };
    let effective_prefix_raw = if !in_string_or_comment {
        match trigger_char_hint.filter(|ch| *ch == '.' || *ch == '(') {
            Some(trigger_char) if !effective_prefix_raw.trim_end().ends_with(trigger_char) => {
                format!("{effective_prefix_raw}{trigger_char}")
            }
            _ => effective_prefix_raw,
        }
    } else {
        effective_prefix_raw
    };

    let line_prefix = trim_to_window(&effective_prefix_raw, CONTEXT_WINDOW_CHARS);
    let line_trimmed = line_prefix.trim_end();

    let trigger_char = (!in_string_or_comment)
        .then(|| {
            line_trimmed
                .chars()
                .last()
                .filter(|ch| *ch == '.' || *ch == '(')
        })
        .flatten()
        .or(trigger_char_hint.filter(|ch| *ch == '.' || *ch == '('));
    let member_base = (!in_string_or_comment)
        .then(|| extract_member_base(line_trimmed))
        .flatten();
    let member_access = !in_string_or_comment
        && (is_member_access_context(line_trimmed) || trigger_char_hint == Some('.'));

    // Extract current word
    let mut current_word = extract_word_at_position(content, line, column).unwrap_or_default();
    if member_access && (line_trimmed.ends_with('.') || trigger_char_hint == Some('.')) {
        current_word.clear();
    }

    CompletionContext {
        current_word,
        member_access,
        member_base,
        trigger_char,
        can_add_statements: can_add_statements(line_trimmed),
        expects_type: expects_type_context(line_trimmed),
        can_add_functions: can_add_functions(line_trimmed),
    }
}

pub(super) fn is_in_string_or_comment(line_prefix: &str) -> bool {
    let mut in_string = false;
    let mut chars = line_prefix.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_string = false;
                }
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            return true;
        }
    }
    in_string
}

/// Checks if statements can be added at this position
pub(super) fn can_add_statements(line_prefix: &str) -> bool {
    line_prefix.is_empty()
        || line_prefix.ends_with(';')
        || line_prefix.ends_with("Тогда")
        || line_prefix.ends_with("Иначе")
        || line_prefix.ends_with("КонецЕсли")
        || line_prefix.ends_with("КонецЦикла")
        || line_prefix.trim_start().is_empty()
}

/// Checks if a type is expected at this position
pub(super) fn expects_type_context(line_prefix: &str) -> bool {
    line_prefix.contains(":")
        || line_prefix.contains("Тип(")
        || line_prefix.contains("ТипЗнч(")
        || line_prefix.contains("// ")
}

/// Checks if functions can be added at this position
pub(super) fn can_add_functions(line_prefix: &str) -> bool {
    !line_prefix.contains("Процедура") && !line_prefix.contains("Функция")
}

pub(super) fn add_keywords(snapshot: &IndexSnapshot, target: &mut Vec<Candidate>, priority: u8) {
    if snapshot.keyword_index.is_empty() {
        for keyword in DEFAULT_KEYWORDS {
            target.push(Candidate::new(
                CompletionItem::new((*keyword).to_string(), CompletionKind::Keyword),
                priority,
                None,
                None,
                None,
            ));
        }
        return;
    }

    for item in snapshot.keyword_index.iter() {
        target.push(Candidate::new(
            CompletionItem::new(item.name.clone(), CompletionKind::Keyword),
            priority,
            None,
            None,
            None,
        ));
    }
}

pub(super) fn add_default_keywords(target: &mut Vec<Candidate>, priority: u8) {
    for keyword in DEFAULT_KEYWORDS {
        target.push(Candidate::new(
            CompletionItem::new((*keyword).to_string(), CompletionKind::Keyword),
            priority,
            None,
            None,
            None,
        ));
    }
}

pub(super) fn add_types(snapshot: &IndexSnapshot, target: &mut Vec<Candidate>, priority: u8) {
    for item in snapshot.type_index.values() {
        if matches!(
            item.kind,
            IndexItemKind::Type(
                TypeKind::Platform
                    | TypeKind::Primitive
                    | TypeKind::Configuration
                    | TypeKind::Generic
                    | TypeKind::Faceted
            )
        ) {
            target.push(Candidate::new(
                CompletionItem::new(item.name.clone(), CompletionKind::Type),
                priority,
                None,
                None,
                None,
            ));
        }
    }
}

pub(super) fn add_repository_types_from_lookup(
    metadata_lookup: &TypeMetadataLookup,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    for type_name in metadata_lookup.get_completion_type_names() {
        target.push(Candidate::new(
            CompletionItem::new(type_name, CompletionKind::Type),
            priority,
            None,
            None,
            None,
        ));
    }
}

pub(super) fn add_global_functions_from_lookup(
    metadata_lookup: &TypeMetadataLookup,
    target: &mut Vec<Candidate>,
    priority: u8,
) {
    for function_name in metadata_lookup.get_global_function_names() {
        target.push(Candidate::new(
            CompletionItem::new(function_name, CompletionKind::Function),
            priority,
            None,
            None,
            Some(SymbolScope::Module),
        ));
    }
}

pub(super) fn resolve_member_access_owner_types_from_program(
    ir_program: &SemanticProgram,
    file_content: &str,
    line: u32,
    column: u32,
) -> Vec<TypeResolution> {
    let Some(spans) = extract_member_access_receiver_spans(file_content, line, column) else {
        return Vec::new();
    };
    let scope_position = resolve_completion_scope_position(ir_program, file_content, line, column);
    let local_candidates = scope_position
        .as_ref()
        .map(|position| collect_local_candidates_from_ir(ir_program, position))
        .unwrap_or_default();

    let mut resolutions = Vec::new();
    for span in spans {
        let span = bsl_shared::ir::Span::new(span.start, span.end);
        let resolution = ir_program
            .semantic_facts
            .type_resolution_for_span(span)
            .or_else(|| {
                ir_program
                    .nodes
                    .iter()
                    .filter(|node| node.span.start <= span.start && node.span.end >= span.end)
                    .min_by_key(|node| node.span.len())
                    .and_then(|node| {
                        ir_program
                            .semantic_facts
                            .type_resolution_for_span(node.span)
                    })
            })
            .or_else(|| {
                receiver_identifier_resolution_from_locals(
                    ir_program,
                    file_content,
                    span,
                    &local_candidates,
                )
            });
        let Some(resolution) = resolution else {
            continue;
        };
        if resolution.is_unknown() || resolution.is_dynamic() {
            continue;
        }
        if !resolutions.contains(&resolution) {
            resolutions.push(resolution);
        }
    }

    resolutions
}

pub(super) fn resolve_member_access_owner_types_from_ir(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
) -> Vec<TypeResolution> {
    let Some(ctx) = analysis else {
        return Vec::new();
    };
    let Some(ir_program) = ctx.ir_program.as_deref() else {
        return Vec::new();
    };
    resolve_member_access_owner_types_from_program(ir_program, file_content, line, column)
}

fn receiver_identifier_resolution_from_locals(
    ir_program: &SemanticProgram,
    file_content: &str,
    receiver_span: bsl_shared::ir::Span,
    local_candidates: &[LocalSymbolCandidate],
) -> Option<TypeResolution> {
    let receiver_text = file_content
        .get(receiver_span.start as usize..receiver_span.end as usize)?
        .trim();
    if receiver_text.is_empty() || !receiver_text.chars().all(is_identifier_char) {
        return None;
    }

    let candidate = local_candidates
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(receiver_text))?;
    ir_program
        .semantic_facts
        .type_at_byte_offset(candidate.span_start)
}

pub(super) fn extract_member_base(line_prefix: &str) -> Option<String> {
    let trimmed = line_prefix.trim_end();
    let dot_pos = trimmed.rfind('.')?;
    let before_dot = &trimmed[..dot_pos];
    let chars: Vec<char> = before_dot.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut end = chars.len();
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    if start == end {
        return None;
    }
    Some(chars[start..end].iter().collect())
}

pub(super) fn is_member_access_context(line_prefix: &str) -> bool {
    let trimmed = line_prefix.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty() || after_dot.chars().all(is_identifier_char)
}

pub(super) fn trim_to_window(line_prefix: &str, window: usize) -> String {
    let mut chars: Vec<char> = line_prefix.chars().collect();
    if chars.len() > window {
        chars.drain(0..(chars.len() - window));
    }
    chars.into_iter().collect()
}

pub(super) fn with_sort_text(
    mut item: CompletionItem,
    score: f32,
    source_priority: u8,
    label_lower: &str,
) -> CompletionItem {
    let score_rank = ((1.0 - score).clamp(0.0, 1.0) * 1000.0) as u32;
    // Primary key is alphabetical label for predictable UX in editors.
    // Keep original case/source/score as stable tie-breakers for deterministic ordering.
    item.sort_text = Some(format!(
        "{}-{}-{:02}-{:04}",
        label_lower,
        item.label.as_str(),
        source_priority,
        score_rank
    ));
    item
}
