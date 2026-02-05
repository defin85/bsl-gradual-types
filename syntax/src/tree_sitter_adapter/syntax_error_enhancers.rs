//! Rule-based post-processing for parser syntax errors.
//!
//! Цель: улучшить UX в IDE/LSP (message/span) для распознаваемых паттернов,
//! не меняя грамматику и не добавляя ложноположительных diagnostics на валидном коде.

use std::cmp::Ordering;
use std::cell::RefCell;
use std::collections::HashMap;

use bsl_shared::domain::types::{ErrorType, ParseError, RelatedInformation};
use bsl_shared::ir::Span;
use tree_sitter::Parser;

use super::span::LineIndex;

thread_local! {
    static SNIPPET_PARSER: RefCell<Parser> = {
        let mut parser = Parser::new();
        let language = tree_sitter_bsl::LANGUAGE.into();
        let _ = parser.set_language(&language);
        RefCell::new(parser)
    };
}

pub(crate) fn normalize_syntax_errors(
    source: &str,
    line_index: &LineIndex,
    parser_errors: Vec<ParseError>,
    heuristic_errors: Vec<ParseError>,
) -> Vec<ParseError> {
    if parser_errors.is_empty() && heuristic_errors.is_empty() {
        return Vec::new();
    }

    let ctx = Context {
        source,
        line_index,
        for_unexpected_cache: RefCell::new(HashMap::new()),
    };

    let mut diags = Vec::with_capacity(parser_errors.len() + heuristic_errors.len());
    diags.extend(parser_errors.into_iter().map(|e| SyntaxDiag {
        origin: Origin::Parser,
        error: e,
    }));
    diags.extend(heuristic_errors.into_iter().map(|e| SyntaxDiag {
        origin: Origin::Heuristic,
        error: e,
    }));

    if diags.iter().any(|d| d.origin == Origin::Parser) {
        for diag in diags.iter_mut() {
            if diag.origin != Origin::Parser {
                continue;
            }
            if let Some(rewritten) = rewrite_error(&ctx, &diag.error) {
                diag.error = rewritten;
            }
        }
    }

    let capped = cap_one_error_per_line(&ctx, diags);
    let sorted = sort_deterministically(capped);
    sorted.into_iter().map(|d| d.error).collect()
}

struct Context<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
    for_unexpected_cache: RefCell<HashMap<(usize, usize, usize), Option<(usize, usize)>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Parser,
    Heuristic,
}

#[derive(Debug, Clone)]
struct SyntaxDiag {
    origin: Origin,
    error: ParseError,
}

fn rewrite_error(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    rewrite_for_step_clause(ctx, error)
        .or_else(|| rewrite_for_unexpected_clause(ctx, error))
        .or_else(|| rewrite_if_missing_then(ctx, error))
        .or_else(|| rewrite_try_structure(ctx, error))
}

fn rewrite_for_step_clause(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    let (line_no, _line, masked) = error_line(ctx, error)?;

    if !line_contains_word_ci(&masked, "Для") && !line_contains_word_ci(&masked, "for") {
        return None;
    }
    if !line_contains_word_ci(&masked, "По") && !line_contains_word_ci(&masked, "to") {
        return None;
    }
    if !line_contains_word_ci(&masked, "Цикл") && !line_contains_word_ci(&masked, "do") {
        return None;
    }

    let (_, po_end) = find_word_ci(&masked, "По").or_else(|| find_word_ci(&masked, "to"))?;
    let (do_start, _) = find_word_ci(&masked, "Цикл").or_else(|| find_word_ci(&masked, "do"))?;
    if po_end >= do_start {
        return None;
    }

    let between = &masked[po_end..do_start];
    let (step_rel_start, step_rel_end) =
        find_word_ci(between, "Шаг").or_else(|| find_word_ci(between, "step"))?;

    let line_start_abs = ctx
        .line_index
        .utf16_position_to_byte_offset(ctx.source, line_no as u32, 0) as u32;
    let abs_start = line_start_abs + (po_end + step_rel_start) as u32;
    let abs_end = line_start_abs + (po_end + step_rel_end) as u32;

    Some(ParseError {
        error_type: ErrorType::InvalidSyntax,
        message: "В цикле `Для` нет синтаксиса `Шаг <expr>`. Уберите `Шаг`, либо используйте корректный вариант (например, `Для i = ... По 0 Цикл` для обратного обхода).".to_string(),
        span: Span::new(abs_start, abs_end),
        related: error.related.clone(),
    })
}

fn rewrite_for_unexpected_clause(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    let (line_no, line, masked) = error_line(ctx, error)?;

    if !line_contains_word_ci(&masked, "Для") && !line_contains_word_ci(&masked, "for") {
        return None;
    }
    if !line_contains_word_ci(&masked, "По") && !line_contains_word_ci(&masked, "to") {
        return None;
    }
    if !line_contains_word_ci(&masked, "Цикл") && !line_contains_word_ci(&masked, "do") {
        return None;
    }

    let (_, po_end) = find_word_ci(&masked, "По").or_else(|| find_word_ci(&masked, "to"))?;
    let (do_start, _) = find_word_ci(&masked, "Цикл").or_else(|| find_word_ci(&masked, "do"))?;
    if po_end >= do_start {
        return None;
    }

    let between_masked = &masked[po_end..do_start];
    let cache_key = (line_no, po_end, do_start);
    let cached = ctx.for_unexpected_cache.borrow().get(&cache_key).cloned();
    let (rel_start, rel_end) = if let Some(v) = cached {
        v?
    } else {
        let computed = first_unexpected_token_span_in_to_clause(between_masked);
        ctx.for_unexpected_cache
            .borrow_mut()
            .insert(cache_key, computed);
        computed?
    };
    let token = line[po_end + rel_start..po_end + rel_end].trim().to_string();
    let line_start_abs = ctx
        .line_index
        .utf16_position_to_byte_offset(ctx.source, line_no as u32, 0) as u32;
    let abs_start = line_start_abs + (po_end + rel_start) as u32;
    let abs_end = line_start_abs + (po_end + rel_end) as u32;

    Some(ParseError {
        error_type: ErrorType::InvalidSyntax,
        message: format!(
            "После `По <выражение>` ожидается `Цикл`, найдено `{}`.",
            token
        ),
        span: Span::new(abs_start, abs_end),
        related: error.related.clone(),
    })
}

fn rewrite_if_missing_then(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    let (line_no, line, masked) = error_line(ctx, error)?;
    let trimmed_masked = masked.trim_start();

    if !starts_with_word_ci(trimmed_masked, "Если") && !starts_with_word_ci(trimmed_masked, "if") {
        return None;
    }
    if line_contains_word_ci(trimmed_masked, "Тогда") || line_contains_word_ci(trimmed_masked, "then") {
        return None;
    }

    let line_start_abs = ctx
        .line_index
        .utf16_position_to_byte_offset(ctx.source, line_no as u32, 0);
    let line_end_abs = line_start_abs + line.len();
    let end_u32 = line_end_abs.min(u32::MAX as usize) as u32;

    Some(ParseError {
        error_type: ErrorType::InvalidSyntax,
        message: "В конструкции `Если` после условия ожидается ключевое слово `Тогда`.".to_string(),
        span: Span::new(end_u32, end_u32),
        related: error.related.clone(),
    })
}

fn rewrite_try_structure(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    let mut is_missing_end = false;
    let mut is_missing_except = false;

    match error.error_type {
        ErrorType::MissingToken => {
            let msg = error.message.as_str();
            is_missing_end =
                msg.contains("ENDTRY_KEYWORD") || msg.contains("КОНЕЦПОПЫТКИ_KEYWORD");
            is_missing_except =
                msg.contains("EXCEPT_KEYWORD") || msg.contains("ИСКЛЮЧЕНИЕ_KEYWORD");
        }
        ErrorType::ParseError => {
            let start = (error.span.start as usize).min(ctx.source.len());
            let end = (error.span.end as usize).min(ctx.source.len());
            let (start, end) = if start <= end { (start, end) } else { (end, start) };
            let slice = &ctx.source[start..end];

            // В `ParseError` мы видим сырой кусок текста. Для безопасности маскируем строки/комментарии
            // перед поиском ключевых слов, чтобы избежать ложных матчей.
            let masked_slice = mask_line_for_rules(slice);

            let has_try = line_contains_word_ci(&masked_slice, "Попытка")
                || line_contains_word_ci(&masked_slice, "try");
            let has_except = line_contains_word_ci(&masked_slice, "Исключение")
                || line_contains_word_ci(&masked_slice, "except");
            let has_end = line_contains_word_ci(&masked_slice, "КонецПопытки")
                || line_contains_word_ci(&masked_slice, "endtry");

            if has_try && has_except && !has_end {
                is_missing_end = true;
            }
        }
        _ => {}
    }

    if !is_missing_end && !is_missing_except {
        return None;
    }

    let mut span = error.span;
    if let Some(anchor) = error
        .related
        .iter()
        .find(|r| r.message.contains("Начало блока: Попытка"))
        .map(|r| r.span)
    {
        span = anchor;
    } else if let Some((line_no, line, masked)) = error_line(ctx, error) {
        if let Some((start, end)) =
            find_word_ci(&masked, "Попытка").or_else(|| find_word_ci(&masked, "try"))
        {
            let line_start_abs = ctx
                .line_index
                .utf16_position_to_byte_offset(ctx.source, line_no as u32, 0) as u32;
            span = Span::new(line_start_abs + start as u32, line_start_abs + end as u32);
            let _ = line;
        }
    }

    let message = if is_missing_end {
        "Не закрыт блок `Попытка` (ожидается `КонецПопытки`).".to_string()
    } else {
        "В блоке `Попытка` ожидается секция `Исключение`.".to_string()
    };

    let mut related = error.related.clone();
    if !related
        .iter()
        .any(|r| r.message.contains("Начало блока: Попытка"))
    {
        related.push(RelatedInformation {
            message: "Начало блока: Попытка".to_string(),
            span,
        });
    }

    Some(ParseError {
        error_type: ErrorType::MissingToken,
        message,
        span,
        related,
    })
}

fn first_unexpected_token_span_in_to_clause(between_masked: &str) -> Option<(usize, usize)> {
    let leading_ws = between_masked
        .as_bytes()
        .iter()
        .take_while(|b| b.is_ascii_whitespace())
        .count();
    let expr = between_masked[leading_ws..].trim_end();
    if expr.is_empty() {
        return None;
    }

    let snippet = format!("x = {};", expr);
    let prefix_len = "x = ".len();
    let tree = SNIPPET_PARSER.with(|p| p.borrow_mut().parse(&snippet, None))?;
    let err_pos = first_error_start_byte(&tree.root_node(), &snippet)?;
    if err_pos < prefix_len {
        return None;
    }

    let rel_in_expr = err_pos - prefix_len;
    let abs_in_between = leading_ws + rel_in_expr;
    token_span_at_or_after(between_masked, abs_in_between)
        .or_else(|| last_token_span(between_masked))
}

fn first_error_start_byte(root: &tree_sitter::Node<'_>, source: &str) -> Option<usize> {
    let mut best_error: Option<usize> = None;
    let mut best_missing: Option<usize> = None;

    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        if node.kind() == "ERROR" {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                if text.trim_start().starts_with('&') {
                    // ignore preprocessor directive errors (consistent with syntax_errors.rs)
                } else {
                    let pos = node.start_byte();
                    best_error = Some(best_error.map_or(pos, |b| b.min(pos)));
                }
            } else {
                let pos = node.start_byte();
                best_error = Some(best_error.map_or(pos, |b| b.min(pos)));
            }
        } else if node.is_missing() {
            let pos = node.start_byte();
            best_missing = Some(best_missing.map_or(pos, |b| b.min(pos)));
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    best_error.or(best_missing)
}

fn cap_one_error_per_line(ctx: &Context<'_>, mut errors: Vec<SyntaxDiag>) -> Vec<SyntaxDiag> {
    if errors.len() <= 1 {
        return errors;
    }

    errors.sort_by(|a, b| compare_errors_for_sort(ctx, a, b));

    let mut out: Vec<SyntaxDiag> = Vec::new();
    let mut current_line: Option<usize> = None;

    for err in errors {
        let line = error_line_number(ctx, &err.error).unwrap_or(usize::MAX);
        if current_line == Some(line) {
            continue;
        }
        current_line = Some(line);
        out.push(err);
    }

    out
}

fn sort_deterministically(mut errors: Vec<SyntaxDiag>) -> Vec<SyntaxDiag> {
    errors.sort_by(|a, b| {
        a.error
            .span
            .start
            .cmp(&b.error.span.start)
            .then_with(|| a.error.span.end.cmp(&b.error.span.end))
            .then_with(|| origin_rank(a.origin).cmp(&origin_rank(b.origin)))
            .then_with(|| error_type_rank(a.error.error_type).cmp(&error_type_rank(b.error.error_type)))
            .then_with(|| a.error.message.cmp(&b.error.message))
    });
    errors
}

fn compare_errors_for_sort(ctx: &Context<'_>, a: &SyntaxDiag, b: &SyntaxDiag) -> Ordering {
    let line_a = error_line_number(ctx, &a.error).unwrap_or(usize::MAX);
    let line_b = error_line_number(ctx, &b.error).unwrap_or(usize::MAX);
    line_a
        .cmp(&line_b)
        .then_with(|| compare_error_quality(a, b))
}

fn compare_error_quality(a: &SyntaxDiag, b: &SyntaxDiag) -> Ordering {
    origin_rank(a.origin)
        .cmp(&origin_rank(b.origin))
        .then_with(|| error_type_rank(a.error.error_type).cmp(&error_type_rank(b.error.error_type)))
        .then_with(|| span_len(a.error.span).cmp(&span_len(b.error.span)))
        .then_with(|| a.error.span.start.cmp(&b.error.span.start))
        .then_with(|| a.error.span.end.cmp(&b.error.span.end))
        .then_with(|| a.error.message.cmp(&b.error.message))
}

fn error_type_rank(t: ErrorType) -> u8 {
    match t {
        ErrorType::InvalidSyntax => 0,
        ErrorType::MissingToken => 1,
        ErrorType::ParseError => 2,
        ErrorType::UnexpectedToken => 3,
    }
}

fn origin_rank(origin: Origin) -> u8 {
    match origin {
        Origin::Parser => 0,
        Origin::Heuristic => 1,
    }
}

fn span_len(span: Span) -> u32 {
    span.end.saturating_sub(span.start)
}

fn error_line_number(ctx: &Context<'_>, error: &ParseError) -> Option<usize> {
    let capped = (error.span.start as usize).min(ctx.source.len());
    Some(ctx.line_index.byte_offset_to_point(ctx.source, capped).0)
}

fn error_line<'a>(ctx: &'a Context<'a>, error: &ParseError) -> Option<(usize, &'a str, String)> {
    let capped = (error.span.start as usize).min(ctx.source.len());
    let (line_no, _) = ctx.line_index.byte_offset_to_point(ctx.source, capped);
    let line = ctx.line_index.line_text(ctx.source, line_no);
    let masked = mask_line_for_rules(line);
    Some((line_no, line, masked))
}

fn token_span_at_or_after(haystack: &str, pos: usize) -> Option<(usize, usize)> {
    let bytes = haystack.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut start = pos.min(bytes.len());
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    if start >= bytes.len() {
        return None;
    }

    let mut end = start + 1;
    let b = bytes[start];

    if is_word_byte(b) {
        while end < bytes.len() && is_word_byte(bytes[end]) {
            end += 1;
        }
        return Some((start, end));
    }

    if b.is_ascii_digit() {
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
            end += 1;
        }
        return Some((start, end));
    }

    Some((start, end))
}

fn last_token_span(haystack: &str) -> Option<(usize, usize)> {
    let bytes = haystack.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let mut start = end - 1;
    let b = bytes[start];

    if is_word_byte(b) {
        while start > 0 && is_word_byte(bytes[start - 1]) {
            start -= 1;
        }
        return Some((start, end));
    }

    if b.is_ascii_digit() {
        while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
            start -= 1;
        }
        return Some((start, end));
    }

    Some((start, start + 1))
}

fn starts_with_word_ci(haystack: &str, needle: &str) -> bool {
    find_word_ci(haystack, needle).is_some_and(|(start, _)| start == 0)
}

fn line_contains_word_ci(haystack: &str, needle: &str) -> bool {
    find_word_ci(haystack, needle).is_some()
}

fn find_word_ci(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }

    let needle_chars: Vec<char> = needle.chars().collect();
    let mut positions: Vec<usize> = haystack.char_indices().map(|(i, _)| i).collect();
    positions.push(haystack.len());

    for &start in positions.iter().take(positions.len().saturating_sub(1)) {
        let mut h_iter = haystack[start..].char_indices();
        let mut matched_end = start;

        let mut ok = true;
        for &nc in &needle_chars {
            let Some((rel, hc)) = h_iter.next() else {
                ok = false;
                break;
            };
            matched_end = start + rel + hc.len_utf8();
            if !char_eq_ci(hc, nc) {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }

        if !is_word_boundary(haystack, start, matched_end) {
            continue;
        }

        return Some((start, matched_end));
    }

    None
}

fn char_eq_ci(a: char, b: char) -> bool {
    lower_single(a).unwrap_or(a) == lower_single(b).unwrap_or(b)
}

fn lower_single(c: char) -> Option<char> {
    let mut it = c.to_lowercase();
    let first = it.next()?;
    if it.next().is_some() {
        return None;
    }
    Some(first)
}

fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let prev = text[..start].chars().last();
    let next = text[end..].chars().next();
    !prev.is_some_and(is_word_char) && !next.is_some_and(is_word_char)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn mask_line_for_rules(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut in_string = false;

    while i < bytes.len() {
        let b = bytes[i];

        if !in_string {
            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                out.push(b'/');
                out.push(b'/');
                i += 2;
                while i < bytes.len() {
                    let cb = bytes[i];
                    if cb == b'\n' {
                        out.push(b'\n');
                        i += 1;
                        break;
                    }
                    if cb == b'\r' {
                        out.push(b'\r');
                        i += 1;
                        if i < bytes.len() && bytes[i] == b'\n' {
                            out.push(b'\n');
                            i += 1;
                        }
                        break;
                    }
                    out.push(b' ');
                    i += 1;
                }
                continue;
            }
            if b == b'"' {
                in_string = true;
                out.push(b'"');
                i += 1;
                continue;
            }

            out.push(b);
            i += 1;
            continue;
        }

        // in_string
        if b == b'\n' {
            in_string = false;
            out.push(b'\n');
            i += 1;
            continue;
        }
        if b == b'\r' {
            in_string = false;
            out.push(b'\r');
            i += 1;
            if i < bytes.len() && bytes[i] == b'\n' {
                out.push(b'\n');
                i += 1;
            }
            continue;
        }
        if b == b'"' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'"' {
                // escaped quote ("")
                out.push(b'"');
                out.push(b'"');
                i += 2;
                continue;
            }
            in_string = false;
            out.push(b'"');
            i += 1;
            continue;
        }

        out.push(b' ');
        i += 1;
    }

    // SAFETY: out contains either original bytes from valid UTF-8 text, or ASCII bytes.
    unsafe { String::from_utf8_unchecked(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generalized_for_picks_first_garbage_token_not_last() {
        let between = " 0 abc def ";
        let (start, end) = first_unexpected_token_span_in_to_clause(between).expect("token span");
        assert_eq!(&between[start..end], "abc");
    }

    #[test]
    fn masking_comment_stops_at_eol_and_keeps_next_line() {
        let input = "// Шаг -1\nabc Цикл";
        let masked = mask_line_for_rules(input);
        assert!(masked.starts_with("//"));
        assert!(
            !masked.contains("Шаг"),
            "comment text should be masked, got: {masked:?}"
        );
        assert!(
            masked.contains("\nabc Цикл"),
            "next line should remain visible, got: {masked:?}"
        );
    }

    #[test]
    fn line_cap_prefers_parser_origin_even_if_message_looks_heuristic() {
        let source = "x y";
        let line_index = LineIndex::new(source);

        let parser_error = ParseError {
            error_type: ErrorType::UnexpectedToken,
            message: "Отсутствует тип после 'Новый'".to_string(),
            span: Span::new(0, 0),
            related: Vec::new(),
        };
        let heuristic_error = ParseError {
            error_type: ErrorType::InvalidSyntax,
            message: "parser-ish".to_string(),
            span: Span::new(1, 1),
            related: Vec::new(),
        };

        let out = normalize_syntax_errors(
            source,
            &line_index,
            vec![parser_error.clone()],
            vec![heuristic_error],
        );

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].span, parser_error.span);
        assert_eq!(out[0].message, parser_error.message);
        assert_eq!(out[0].error_type, parser_error.error_type);
    }
}
