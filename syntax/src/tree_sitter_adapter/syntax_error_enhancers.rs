//! Rule-based post-processing for parser syntax errors.
//!
//! Цель: улучшить UX в IDE/LSP (message/span) для распознаваемых паттернов,
//! не меняя грамматику и не добавляя ложноположительных diagnostics на валидном коде.

use std::cmp::Ordering;

use bsl_shared::domain::types::{ErrorType, ParseError};
use bsl_shared::ir::Span;

use super::span::LineIndex;

pub(crate) fn enhance_syntax_errors(
    source: &str,
    line_index: &LineIndex,
    errors: Vec<ParseError>,
) -> Vec<ParseError> {
    if errors.is_empty() {
        return errors;
    }

    let ctx = Context { source, line_index };

    let rewritten: Vec<ParseError> = errors
        .into_iter()
        .map(|e| rewrite_error(&ctx, &e).unwrap_or(e))
        .collect();

    let capped = cap_one_error_per_line(&ctx, rewritten);
    sort_deterministically(capped)
}

struct Context<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
}

fn rewrite_error(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    rewrite_for_step_clause(ctx, error)
        .or_else(|| rewrite_for_unexpected_clause(ctx, error))
        .or_else(|| rewrite_if_missing_then(ctx, error))
        .or_else(|| rewrite_try_structure(ctx, error))
}

fn rewrite_for_step_clause(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    let (line_no, line) = error_line(ctx, error)?;

    if !line_contains_word_ci(line, "Для") && !line_contains_word_ci(line, "for") {
        return None;
    }
    if !line_contains_word_ci(line, "По") && !line_contains_word_ci(line, "to") {
        return None;
    }
    if !line_contains_word_ci(line, "Цикл") && !line_contains_word_ci(line, "do") {
        return None;
    }

    let (_, po_end) = find_word_ci(line, "По").or_else(|| find_word_ci(line, "to"))?;
    let (do_start, _) = find_word_ci(line, "Цикл").or_else(|| find_word_ci(line, "do"))?;
    if po_end >= do_start {
        return None;
    }

    let between = &line[po_end..do_start];
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
    let (line_no, line) = error_line(ctx, error)?;

    if !line_contains_word_ci(line, "Для") && !line_contains_word_ci(line, "for") {
        return None;
    }
    if !line_contains_word_ci(line, "По") && !line_contains_word_ci(line, "to") {
        return None;
    }
    if !line_contains_word_ci(line, "Цикл") && !line_contains_word_ci(line, "do") {
        return None;
    }

    let (_, po_end) = find_word_ci(line, "По").or_else(|| find_word_ci(line, "to"))?;
    let (do_start, _) = find_word_ci(line, "Цикл").or_else(|| find_word_ci(line, "do"))?;
    if po_end >= do_start {
        return None;
    }

    let between = &line[po_end..do_start];
    let unexpected = find_any_word_ci(
        between,
        &[
            "Шаг",
            "step",
            "Тогда",
            "then",
            "Иначе",
            "else",
            "Исключение",
            "except",
        ],
    )?;

    let (rel_start, rel_end, token) = unexpected;
    let line_start_abs = ctx
        .line_index
        .utf16_position_to_byte_offset(ctx.source, line_no as u32, 0) as u32;
    let abs_start = line_start_abs + (po_end + rel_start) as u32;
    let abs_end = line_start_abs + (po_end + rel_end) as u32;

    Some(ParseError {
        error_type: ErrorType::InvalidSyntax,
        message: format!(
            "В заголовке цикла `Для` после `По <выражение>` ожидается `Цикл`, найдено `{}`.",
            token
        ),
        span: Span::new(abs_start, abs_end),
        related: error.related.clone(),
    })
}

fn rewrite_if_missing_then(ctx: &Context<'_>, error: &ParseError) -> Option<ParseError> {
    let (line_no, line) = error_line(ctx, error)?;
    let trimmed = line.trim_start();

    if !starts_with_word_ci(trimmed, "Если") && !starts_with_word_ci(trimmed, "if") {
        return None;
    }
    if line_contains_word_ci(trimmed, "Тогда") || line_contains_word_ci(trimmed, "then") {
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

            let has_try = line_contains_word_ci(slice, "Попытка") || line_contains_word_ci(slice, "try");
            let has_except = line_contains_word_ci(slice, "Исключение")
                || line_contains_word_ci(slice, "except");
            let has_end = line_contains_word_ci(slice, "КонецПопытки")
                || line_contains_word_ci(slice, "endtry");

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
    } else if let Some((line_no, line)) = error_line(ctx, error) {
        if let Some((start, end)) = find_word_ci(line, "Попытка").or_else(|| find_word_ci(line, "try"))
        {
            let line_start_abs = ctx
                .line_index
                .utf16_position_to_byte_offset(ctx.source, line_no as u32, 0) as u32;
            span = Span::new(line_start_abs + start as u32, line_start_abs + end as u32);
        }
    }

    let message = if is_missing_end {
        "Не закрыт блок `Попытка` (ожидается `КонецПопытки`).".to_string()
    } else {
        "В блоке `Попытка` ожидается секция `Исключение`.".to_string()
    };

    Some(ParseError {
        error_type: ErrorType::MissingToken,
        message,
        span,
        related: error.related.clone(),
    })
}

fn cap_one_error_per_line(ctx: &Context<'_>, mut errors: Vec<ParseError>) -> Vec<ParseError> {
    if errors.len() <= 1 {
        return errors;
    }

    errors.sort_by(|a, b| compare_errors_for_sort(ctx, a, b));

    let mut out: Vec<ParseError> = Vec::new();
    let mut current_line: Option<usize> = None;

    for err in errors {
        let line = error_line_number(ctx, &err).unwrap_or(usize::MAX);
        if current_line == Some(line) {
            continue;
        }
        current_line = Some(line);
        out.push(err);
    }

    out
}

fn sort_deterministically(mut errors: Vec<ParseError>) -> Vec<ParseError> {
    errors.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then_with(|| a.span.end.cmp(&b.span.end))
            .then_with(|| error_type_rank(a.error_type).cmp(&error_type_rank(b.error_type)))
            .then_with(|| a.message.cmp(&b.message))
    });
    errors
}

fn compare_errors_for_sort(ctx: &Context<'_>, a: &ParseError, b: &ParseError) -> Ordering {
    let line_a = error_line_number(ctx, a).unwrap_or(usize::MAX);
    let line_b = error_line_number(ctx, b).unwrap_or(usize::MAX);
    line_a
        .cmp(&line_b)
        .then_with(|| compare_error_quality(a, b))
}

fn compare_error_quality(a: &ParseError, b: &ParseError) -> Ordering {
    let prio_a = error_type_rank(a.error_type);
    let prio_b = error_type_rank(b.error_type);
    prio_a
        .cmp(&prio_b)
        .then_with(|| span_len(a.span).cmp(&span_len(b.span)))
        .then_with(|| a.span.start.cmp(&b.span.start))
        .then_with(|| a.span.end.cmp(&b.span.end))
        .then_with(|| a.message.cmp(&b.message))
}

fn error_type_rank(t: ErrorType) -> u8 {
    match t {
        ErrorType::InvalidSyntax => 0,
        ErrorType::MissingToken => 1,
        ErrorType::UnexpectedToken => 2,
        ErrorType::ParseError => 3,
    }
}

fn span_len(span: Span) -> u32 {
    span.end.saturating_sub(span.start)
}

fn error_line_number(ctx: &Context<'_>, error: &ParseError) -> Option<usize> {
    let capped = (error.span.start as usize).min(ctx.source.len());
    Some(ctx.line_index.byte_offset_to_point(ctx.source, capped).0)
}

fn error_line<'a>(ctx: &'a Context<'a>, error: &ParseError) -> Option<(usize, &'a str)> {
    let capped = (error.span.start as usize).min(ctx.source.len());
    let (line_no, _) = ctx.line_index.byte_offset_to_point(ctx.source, capped);
    let line = ctx.line_index.line_text(ctx.source, line_no);
    Some((line_no, line))
}

fn find_any_word_ci(haystack: &str, needles: &[&str]) -> Option<(usize, usize, String)> {
    let mut best: Option<(usize, usize, String)> = None;
    for &needle in needles {
        if let Some((start, end)) = find_word_ci(haystack, needle) {
            let token = haystack[start..end].to_string();
            match &best {
                None => best = Some((start, end, token)),
                Some((best_start, _, _)) if start < *best_start => best = Some((start, end, token)),
                _ => {}
            }
        }
    }
    best
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
