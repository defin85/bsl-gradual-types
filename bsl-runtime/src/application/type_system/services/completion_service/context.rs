use super::*;

/// Context for auto-completion.
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
            ));
        }
    }
}

pub(super) fn resolve_type_name(
    snapshot: &IndexSnapshot,
    name: &str,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<String> {
    let lowered = name.to_lowercase();
    let from_index = snapshot
        .type_index
        .values()
        .find(|item| item.name.to_lowercase() == lowered)
        .map(|item| item.name.clone());
    if from_index.is_some() {
        return from_index;
    }

    let resolution = TypeResolution::explicit(name);
    metadata_lookup
        .get_raw_type(&resolution)
        .map(|raw| raw.name)
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

pub(super) fn extract_member_receiver_chain(
    content: &str,
    line: u32,
    column: u32,
) -> Option<Vec<String>> {
    let lines: Vec<&str> = content.lines().collect();
    let line_content = *lines.get(line as usize)?;
    let column_index = utf16_to_byte_offset(line_content, column);
    let line_prefix = trim_to_window(&line_content[..column_index], CONTEXT_WINDOW_CHARS);
    let trimmed = line_prefix.trim_end();
    let dot_pos = trimmed.rfind('.')?;
    let receiver_expr = trimmed[..dot_pos].trim_end();
    if receiver_expr.is_empty() {
        return None;
    }
    extract_identifier_chain_tail(receiver_expr)
}

pub(super) fn extract_identifier_chain_tail(expr: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = expr.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut end = chars.len();
    let mut parts_rev: Vec<String> = Vec::new();

    loop {
        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        if end == 0 {
            break;
        }

        let mut start = end;
        while start > 0 && is_identifier_char(chars[start - 1]) {
            start -= 1;
        }
        if start == end {
            return None;
        }
        parts_rev.push(chars[start..end].iter().collect());

        end = start;
        while end > 0 && chars[end - 1].is_whitespace() {
            end -= 1;
        }
        if end == 0 {
            break;
        }
        if chars[end - 1] != '.' {
            break;
        }
        end -= 1;
    }

    if parts_rev.is_empty() {
        return None;
    }
    parts_rev.reverse();
    Some(parts_rev)
}

pub(super) async fn resolve_member_chain_owner_type(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    receiver_chain: &[String],
    snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<TypeResolution> {
    resolve_member_chain_owner_type_sync(
        analysis,
        file_content,
        line,
        column,
        receiver_chain,
        snapshot,
        metadata_lookup,
    )
}

pub(super) fn resolve_member_chain_owner_type_sync(
    analysis: Option<&CompletionAnalysisContext<'_>>,
    file_content: &str,
    line: u32,
    column: u32,
    receiver_chain: &[String],
    snapshot: &IndexSnapshot,
    metadata_lookup: &TypeMetadataLookup,
) -> Option<TypeResolution> {
    if receiver_chain.is_empty() {
        return None;
    }

    let base_name = receiver_chain[0].as_str();
    let mut start_index = 1usize;
    let mut owner = if let Some(kind) = get_collection_kind(base_name) {
        let object_name = receiver_chain.get(1)?;
        start_index = 2;
        let expr = format!("{}.{}", base_name, object_name);
        analysis
            .map(|ctx| ctx.resolver.resolve_expression_sync(&expr))
            .unwrap_or_else(|| {
                TypeResolution::metadata_type(kind, object_name, Some(FacetKind::Manager))
            })
    } else if let Some(type_name) = resolve_type_name(snapshot, base_name, metadata_lookup) {
        analysis
            .map(|ctx| ctx.resolver.resolve_expression_sync(&type_name))
            .unwrap_or_else(|| TypeResolution::explicit(&type_name))
    } else {
        resolve_member_owner_type_sync(analysis, file_content, line, column, base_name)?
    };

    let resolver = analysis.map(|ctx| ctx.resolver);
    for member_name in receiver_chain.iter().skip(start_index) {
        if owner.is_unknown() {
            return None;
        }

        if let Some(resolved) =
            resolve_property_access_type(resolver, metadata_lookup, &owner, member_name)
        {
            owner = resolved;
            continue;
        }

        if let Some(resolved) =
            resolve_method_call_return_type(resolver, metadata_lookup, &owner, member_name)
        {
            owner = resolved;
            continue;
        }

        return None;
    }

    if owner.is_unknown() {
        None
    } else {
        Some(owner)
    }
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
