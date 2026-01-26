use tower_lsp::lsp_types::{Position, Range, TextEdit};

use bsl_syntax::formatter::{format_document, FormatOptions};

#[derive(Debug, thiserror::Error)]
pub enum FormattingError {
    #[error("formatter failed: {0}")]
    Formatter(String),
}

pub fn format_bsl_to_edits(
    source: &str,
    indent_size: usize,
) -> Result<Option<Vec<TextEdit>>, FormattingError> {
    let options = FormatOptions { indent_size };
    let formatted = format_document(source, &options)
        .map_err(|e| FormattingError::Formatter(e.to_string()))?;

    if formatted == source {
        return Ok(Some(vec![]));
    }

    let edits = compute_line_edits(source, &formatted);
    Ok(Some(edits))
}

pub fn format_bsl_range_to_edits(
    source: &str,
    indent_size: usize,
    range: Range,
) -> Result<Option<Vec<TextEdit>>, FormattingError> {
    let options = FormatOptions { indent_size };
    let formatted = format_document(source, &options)
        .map_err(|e| FormattingError::Formatter(e.to_string()))?;

    if formatted == source {
        return Ok(Some(vec![]));
    }

    let mut edits = compute_line_edits(source, &formatted);

    let start_line = range.start.line;
    let mut end_line = range.end.line;
    if range.end.character == 0 && end_line > 0 {
        end_line -= 1;
    }

    edits.retain(|edit| {
        let line = edit.range.start.line;
        line >= start_line && line <= end_line
    });

    Ok(Some(edits))
}

fn compute_line_edits(old: &str, new: &str) -> Vec<TextEdit> {
    let old_lines = split_lines_for_lsp(old);
    let new_lines = split_lines_for_lsp(new);

    let mut edits = Vec::new();
    let common = old_lines.len().min(new_lines.len());

    for i in 0..common {
        if old_lines[i] == new_lines[i] {
            continue;
        }
        let end_char = old_lines[i].encode_utf16().count() as u32;
        edits.push(TextEdit {
            range: Range {
                start: Position {
                    line: i as u32,
                    character: 0,
                },
                end: Position {
                    line: i as u32,
                    character: end_char,
                },
            },
            new_text: new_lines[i].to_string(),
        });
    }

    // If formatter added a trailing newline (common case), represent it as an insertion at EOF.
    if new_lines.len() == old_lines.len() + 1
        && !old.ends_with('\n')
        && new.ends_with('\n')
        && new_lines.last().is_some_and(|l| l.is_empty())
    {
        let last_line = old_lines.len().saturating_sub(1);
        let last_end_char = old_lines
            .get(last_line)
            .map(|l| l.encode_utf16().count() as u32)
            .unwrap_or(0);
        let newline = if new.contains("\r\n") { "\r\n" } else { "\n" };
        edits.push(TextEdit {
            range: Range {
                start: Position {
                    line: last_line as u32,
                    character: last_end_char,
                },
                end: Position {
                    line: last_line as u32,
                    character: last_end_char,
                },
            },
            new_text: newline.to_string(),
        });
    }

    edits
}

fn split_lines_for_lsp(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    for (idx, &b) in bytes.iter().enumerate() {
        if b != b'\n' {
            continue;
        }
        let mut end = idx;
        if end > start && bytes[end - 1] == b'\r' {
            end -= 1;
        }
        out.push(&text[start..end]);
        start = idx + 1;
    }
    // Always include the tail (even if empty) so that a trailing newline produces a final empty line.
    if start <= text.len() {
        out.push(&text[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_line_edits_replaces_only_changed_lines() {
        let old = "a  \n  b\n";
        let new = "a\n    b\n";
        let edits = compute_line_edits(old, new);
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].new_text, "a");
        assert_eq!(edits[1].new_text, "    b");
    }
}
