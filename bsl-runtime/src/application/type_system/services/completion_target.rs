//! CompletionTarget - extraction of receiver expression under cursor for completion.
//!
//! M3 (IntelliSense v2): determine receiver expression for member access completion (`expr.`)
//! using syntax AST (bsl-syntax) rather than tail-of-line string heuristics.

use bsl_syntax::ast::{Expression, ParseResult, Program, Statement};

use crate::application::type_system::extractors::symbol_extractor::utf16_to_byte_offset;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CompletionTargetKind {
    MemberAccess,
    Call,
    Statement,
    TypePosition,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompletionTarget {
    pub kind: CompletionTargetKind,
    pub receiver_expression: Option<Expression>,
    pub receiver_union_expressions: Option<Vec<Expression>>,
    pub receiver: Option<ReceiverChain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ReceiverChainHead {
    Identifier(String),
    ExplicitType(String),
    Call(String),
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverChainSegmentKind {
    Property,
    Call,
    Index,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverChainSegment {
    pub kind: ReceiverChainSegmentKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverChain {
    pub head: ReceiverChainHead,
    pub segments: Vec<ReceiverChainSegment>,
}

impl ReceiverChain {
    #[allow(dead_code)]
    pub fn to_name_chain(&self) -> Option<Vec<String>> {
        let mut out = Vec::new();
        match &self.head {
            ReceiverChainHead::Identifier(name) | ReceiverChainHead::ExplicitType(name) => {
                out.push(name.clone());
            }
            ReceiverChainHead::Call(_) | ReceiverChainHead::Unsupported => return None,
        }

        for seg in &self.segments {
            let name = seg.name.as_ref()?;
            out.push(name.clone());
        }

        Some(out)
    }
}

pub fn extract_completion_target_for_member_access(
    file_content: &str,
    line: u32,
    column: u32,
    _parse_result: &ParseResult,
) -> Option<CompletionTarget> {
    let receiver_text = extract_member_access_receiver_text(file_content, line, column)?;
    let receiver_union_expressions = try_extract_choice_union_expressions(receiver_text);
    let receiver_expression = receiver_union_expressions
        .is_none()
        .then(|| parse_expression_snippet(receiver_text))
        .flatten();

    let receiver = receiver_expression
        .as_ref()
        .and_then(receiver_chain_from_expression);

    if receiver_expression.is_none() && receiver_union_expressions.is_none() {
        return None;
    }

    Some(CompletionTarget {
        kind: CompletionTargetKind::MemberAccess,
        receiver_expression,
        receiver_union_expressions,
        receiver,
    })
}

#[allow(dead_code)]
pub fn extract_member_access_receiver_chain(
    file_content: &str,
    line: u32,
    column: u32,
    _parse_result: &ParseResult,
) -> Option<ReceiverChain> {
    let receiver_text = extract_member_access_receiver_text(file_content, line, column)?;
    let receiver_expr = parse_expression_snippet(receiver_text)?;
    receiver_chain_from_expression(&receiver_expr)
}

#[allow(dead_code)]
pub fn extract_member_access_receiver_expression(
    file_content: &str,
    line: u32,
    column: u32,
    _parse_result: &ParseResult,
) -> Option<Expression> {
    let receiver_text = extract_member_access_receiver_text(file_content, line, column)?;
    parse_expression_snippet(receiver_text)
}

fn extract_member_access_receiver_text(file_content: &str, line: u32, column: u32) -> Option<&str> {
    let line_content = file_content.lines().nth(line as usize)?;
    let cursor_byte_in_line = utf16_to_byte_offset(line_content, column);
    let line_prefix = line_content.get(..cursor_byte_in_line)?;
    let dot_in_line = line_prefix.rfind('.')?;
    let dot_global = file_byte_position_to_byte_offset(file_content, line, dot_in_line)?;

    let file_prefix = file_content.get(..dot_global)?;
    if let Some(choice_text) = extract_choice_expression(file_prefix) {
        return Some(choice_text);
    }

    let receiver_prefix = line_prefix.get(..dot_in_line)?;
    let receiver_expr_text = extract_expression_suffix(receiver_prefix)?;
    let receiver_expr_text = strip_wrapping_parentheses(receiver_expr_text.trim());
    if receiver_expr_text.is_empty() {
        None
    } else {
        Some(receiver_expr_text)
    }
}

fn file_byte_position_to_byte_offset(
    file_content: &str,
    line: u32,
    column_byte: usize,
) -> Option<usize> {
    let mut current_line: u32 = 0;
    let mut offset: usize = 0;

    for chunk in file_content.split_inclusive('\n') {
        if current_line == line {
            let mut line_text = chunk.strip_suffix('\n').unwrap_or(chunk);
            line_text = line_text.strip_suffix('\r').unwrap_or(line_text);
            return Some(offset + column_byte.min(line_text.len()));
        }

        offset = offset.saturating_add(chunk.len());
        current_line = current_line.saturating_add(1);
    }

    None
}

fn parse_expression_snippet(expr_text: &str) -> Option<Expression> {
    let synthetic = format!(
        "Procedure __CompletionTarget__()\n    __tmp = {};\nEndProcedure\n",
        expr_text
    );
    let parse = bsl_syntax::parse_fast(&synthetic).ok()?;
    find_first_assignment_value(&parse.program)
}

fn strip_wrapping_parentheses(text: &str) -> &str {
    let mut out = text.trim();
    loop {
        let Some(stripped) = try_strip_one_pair_of_parens(out) else {
            return out;
        };
        out = stripped.trim();
    }
}

fn try_strip_one_pair_of_parens(text: &str) -> Option<&str> {
    if !text.starts_with('(') || !text.ends_with(')') {
        return None;
    }

    let mut depth: i32 = 0;
    let mut in_string = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            continue;
        }

        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 && idx.saturating_add(ch.len_utf8()) != text.len() {
                    return None;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return None;
    }

    text.get(1..text.len().saturating_sub(1))
}

fn extract_expression_suffix(prefix: &str) -> Option<&str> {
    let trimmed = prefix.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    let start = find_expression_start(trimmed);
    let expr = trimmed.get(start..)?.trim();
    if expr.is_empty() {
        None
    } else {
        Some(expr)
    }
}

fn find_expression_start(prefix: &str) -> usize {
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut in_string = false;

    let chars: Vec<(usize, char)> = prefix.char_indices().collect();
    for &(idx, ch) in chars.iter().rev() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                continue;
            }
            ')' => {
                paren_depth += 1;
                continue;
            }
            '(' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    continue;
                }
                return idx + ch.len_utf8();
            }
            ']' => {
                bracket_depth += 1;
                continue;
            }
            '[' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                    continue;
                }
                return idx + ch.len_utf8();
            }
            _ => {}
        }

        if paren_depth != 0 || bracket_depth != 0 {
            continue;
        }

        match ch {
            ';' | ',' | '=' | '+' | '-' | '*' | '/' => return idx + ch.len_utf8(),
            _ => {}
        }
    }

    0
}

fn find_first_assignment_value(program: &Program) -> Option<Expression> {
    for stmt in &program.statements {
        if let Some(value) = find_first_assignment_value_in_statement(stmt) {
            return Some(value);
        }
    }
    None
}

fn find_first_assignment_value_in_statement(stmt: &Statement) -> Option<Expression> {
    match stmt {
        Statement::Assignment { value, .. } => Some(value.clone()),
        Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => body
            .iter()
            .find_map(find_first_assignment_value_in_statement),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoiceKeywordKind {
    Case,
    When,
    Then,
    Else,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChoiceKeyword {
    kind: ChoiceKeywordKind,
    start: usize,
    end: usize,
}

fn try_extract_choice_union_expressions(receiver_expr_text: &str) -> Option<Vec<Expression>> {
    let choice_text = extract_choice_expression(receiver_expr_text)?;
    let parts = extract_choice_result_expression_slices(choice_text)?;
    let mut out: Vec<Expression> = Vec::new();
    for part in parts {
        if let Some(expr) = parse_expression_snippet(part) {
            out.push(expr);
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn extract_choice_expression(receiver_expr_text: &str) -> Option<&str> {
    let trimmed = receiver_expr_text.trim_end();
    let lower = trimmed.to_lowercase();
    if lower.len() != trimmed.len() {
        return None;
    }

    let keywords = collect_choice_keywords(trimmed, &lower, 0);
    if keywords.is_empty() {
        return None;
    }

    let mut stack: Vec<usize> = Vec::new();
    let mut matched_start: Option<usize> = None;
    for kw in &keywords {
        match kw.kind {
            ChoiceKeywordKind::Case => stack.push(kw.start),
            ChoiceKeywordKind::End => {
                let Some(case_start) = stack.pop() else {
                    continue;
                };
                if kw.end == trimmed.len() {
                    matched_start = Some(case_start);
                }
            }
            _ => {}
        }
    }

    let start = matched_start?;
    trimmed.get(start..)
}

fn extract_choice_result_expression_slices(receiver_expr_text: &str) -> Option<Vec<&str>> {
    let start_offset = receiver_expr_text
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(receiver_expr_text.len());

    let lower = receiver_expr_text.to_lowercase();
    if lower.len() != receiver_expr_text.len() {
        return None;
    }

    if keyword_at(&lower, start_offset, "выбор").is_none()
        && keyword_at(&lower, start_offset, "case").is_none()
    {
        return None;
    }

    let keywords = collect_choice_keywords(receiver_expr_text, &lower, start_offset);
    if keywords.is_empty() {
        return None;
    }

    let end_start = keywords
        .iter()
        .rfind(|kw| kw.kind == ChoiceKeywordKind::End)
        .map(|kw| kw.start)?;

    let mut out: Vec<&str> = Vec::new();

    for kw in &keywords {
        if kw.kind != ChoiceKeywordKind::Then || kw.end > end_start {
            continue;
        }

        let expr_start = skip_ws(receiver_expr_text, kw.end);
        let expr_end = keywords
            .iter()
            .filter(|next| next.start >= expr_start)
            .filter(|next| {
                matches!(
                    next.kind,
                    ChoiceKeywordKind::When | ChoiceKeywordKind::Else | ChoiceKeywordKind::End
                )
            })
            .map(|next| next.start)
            .min()
            .unwrap_or(receiver_expr_text.len());

        let expr = receiver_expr_text.get(expr_start..expr_end)?.trim();
        if !expr.is_empty() {
            out.push(expr);
        }
    }

    if let Some(else_kw) = keywords
        .iter()
        .find(|kw| kw.kind == ChoiceKeywordKind::Else && kw.end <= end_start)
        .copied()
    {
        let expr_start = skip_ws(receiver_expr_text, else_kw.end);
        let expr = receiver_expr_text.get(expr_start..end_start)?.trim();
        if !expr.is_empty() {
            out.push(expr);
        }
    }

    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn collect_choice_keywords(
    receiver_expr_text: &str,
    lower: &str,
    start_offset: usize,
) -> Vec<ChoiceKeyword> {
    let mut keywords: Vec<ChoiceKeyword> = Vec::new();
    let mut i = start_offset;
    let mut in_string = false;
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;

    while i < receiver_expr_text.len() {
        let ch = receiver_expr_text[i..].chars().next().unwrap_or('\0');
        let ch_len = ch.len_utf8().max(1);

        if in_string {
            if ch == '"' {
                let next_i = i.saturating_add(ch_len);
                let is_escaped_quote = receiver_expr_text
                    .get(next_i..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|next_ch| next_ch == '"');

                if is_escaped_quote {
                    i = next_i.saturating_add(1);
                    continue;
                }
                in_string = false;
            }

            i = i.saturating_add(ch_len);
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                i = i.saturating_add(ch_len);
                continue;
            }
            '(' => {
                paren_depth = paren_depth.saturating_add(1);
                i = i.saturating_add(ch_len);
                continue;
            }
            ')' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                i = i.saturating_add(ch_len);
                continue;
            }
            '[' => {
                bracket_depth = bracket_depth.saturating_add(1);
                i = i.saturating_add(ch_len);
                continue;
            }
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
                i = i.saturating_add(ch_len);
                continue;
            }
            _ => {}
        }

        if paren_depth == 0 && bracket_depth == 0 {
            if let Some(end) =
                keyword_at(lower, i, "выбор").or_else(|| keyword_at(lower, i, "case"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::Case,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) =
                keyword_at(lower, i, "когда").or_else(|| keyword_at(lower, i, "when"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::When,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) =
                keyword_at(lower, i, "тогда").or_else(|| keyword_at(lower, i, "then"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::Then,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) =
                keyword_at(lower, i, "иначе").or_else(|| keyword_at(lower, i, "else"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::Else,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }

            if let Some(end) = keyword_at(lower, i, "конецвыбора")
                .or_else(|| keyword_at(lower, i, "endcase"))
                .or_else(|| keyword_at(lower, i, "конец"))
                .or_else(|| keyword_at(lower, i, "end"))
            {
                keywords.push(ChoiceKeyword {
                    kind: ChoiceKeywordKind::End,
                    start: i,
                    end,
                });
                i = end;
                continue;
            }
        }

        i = i.saturating_add(ch_len);
    }

    keywords
}

fn skip_ws(text: &str, mut idx: usize) -> usize {
    while let Some(ch) = text.get(idx..).and_then(|rest| rest.chars().next()) {
        if ch.is_whitespace() {
            idx = idx.saturating_add(ch.len_utf8());
        } else {
            break;
        }
    }
    idx
}

fn keyword_at(lower: &str, idx: usize, keyword: &str) -> Option<usize> {
    let rest = lower.get(idx..)?;
    if !rest.starts_with(keyword) {
        return None;
    }

    let before = lower.get(..idx)?.chars().next_back();
    if before.is_some_and(is_word_char) {
        return None;
    }

    let end = idx.saturating_add(keyword.len());
    let after = lower.get(end..)?.chars().next();
    if after.is_some_and(is_word_char) {
        return None;
    }

    Some(end)
}

fn is_word_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn receiver_chain_from_expression(expr: &Expression) -> Option<ReceiverChain> {
    match expr {
        Expression::Identifier { name, .. } => Some(ReceiverChain {
            head: ReceiverChainHead::Identifier(name.clone()),
            segments: Vec::new(),
        }),
        Expression::New { type_name, .. } => Some(ReceiverChain {
            head: ReceiverChainHead::ExplicitType(type_name.clone()),
            segments: Vec::new(),
        }),
        Expression::Await { expression, .. } => receiver_chain_from_expression(expression),
        Expression::PropertyAccess {
            object, property, ..
        } => {
            let mut chain = receiver_chain_from_expression(object)?;
            chain.segments.push(ReceiverChainSegment {
                kind: ReceiverChainSegmentKind::Property,
                name: Some(property.clone()),
            });
            Some(chain)
        }
        Expression::Call { function, .. } => match function.as_ref() {
            Expression::PropertyAccess {
                object, property, ..
            } => {
                let mut chain = receiver_chain_from_expression(object)?;
                chain.segments.push(ReceiverChainSegment {
                    kind: ReceiverChainSegmentKind::Call,
                    name: Some(property.clone()),
                });
                Some(chain)
            }
            Expression::Identifier { name, .. } => Some(ReceiverChain {
                head: ReceiverChainHead::Call(name.clone()),
                segments: Vec::new(),
            }),
            _ => None,
        },
        Expression::IndexAccess { object, .. } => {
            let mut chain = receiver_chain_from_expression(object)?;
            chain.segments.push(ReceiverChainSegment {
                kind: ReceiverChainSegmentKind::Index,
                name: None,
            });
            Some(chain)
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "completion_target/tests.rs"]
mod tests;
