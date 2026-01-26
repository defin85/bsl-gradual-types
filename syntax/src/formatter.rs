use crate::ParseFatalError;

#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub indent_size: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { indent_size: 4 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("parse failed: {0}")]
    Parse(#[from] ParseFatalError),
}

/// Format BSL source using AST-based indentation rules.
///
/// MVP scope:
/// - Reindent (leading whitespace) based on tree-sitter keywords
/// - Trim trailing whitespace
/// - Ensure the file ends with exactly one newline (preserving existing line ending style)
pub fn format_document(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    let tree = crate::parse_tree(source)?;
    let mut lines = split_lines_with_endings(source);
    let line_count = lines.len();

    let (before_dedent, after_indent) = collect_indent_events(&tree, line_count);

    let mut indent_level: i32 = 0;
    for (idx, (line, _eol)) in lines.iter_mut().enumerate() {
        indent_level = (indent_level - before_dedent[idx]).max(0);
        *line = format_line(line, indent_level, options.indent_size);
        indent_level += after_indent[idx];
    }

    let mut out = String::with_capacity(source.len().saturating_add(1));
    for (line, eol) in &lines {
        out.push_str(line);
        out.push_str(eol);
    }

    if !out.ends_with('\n') {
        out.push_str(detect_default_eol(source));
    }

    Ok(out)
}

fn detect_default_eol(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn split_lines_with_endings(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = source.as_bytes();
    for (idx, &b) in bytes.iter().enumerate() {
        if b != b'\n' {
            continue;
        }
        let end = idx + 1;
        let raw = &source[start..end];
        if raw.ends_with("\r\n") {
            out.push((raw[..raw.len() - 2].to_string(), "\r\n".to_string()));
        } else {
            out.push((raw[..raw.len() - 1].to_string(), "\n".to_string()));
        }
        start = end;
    }
    if start <= source.len() {
        out.push((source[start..].to_string(), String::new()));
    }
    out
}

fn format_line(line: &str, indent_level: i32, indent_size: usize) -> String {
    let rstripped = line.trim_end_matches([' ', '\t']);
    if rstripped.is_empty() {
        return String::new();
    }

    let first_non_ws = rstripped
        .bytes()
        .position(|b| b != b' ' && b != b'\t')
        .unwrap_or(rstripped.len());
    let rest = &rstripped[first_non_ws..];

    let indent = " ".repeat(indent_level.max(0) as usize * indent_size);
    format!("{indent}{rest}")
}

fn collect_indent_events(
    tree: &tree_sitter::Tree,
    line_count: usize,
) -> (Vec<i32>, Vec<i32>) {
    let mut before_dedent = vec![0i32; line_count];
    let mut after_indent = vec![0i32; line_count];

    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if let Some((dedent_before, indent_after)) = keyword_effect(kind) {
            let row = node.start_position().row as usize;
            if row < line_count {
                before_dedent[row] += dedent_before;
                after_indent[row] += indent_after;
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    (before_dedent, after_indent)
}

fn keyword_effect(kind: &str) -> Option<(i32, i32)> {
    match kind {
        // Block openers.
        "THEN_KEYWORD" | "DO_KEYWORD" | "ЦИКЛ_KEYWORD" | "PROCEDURE_KEYWORD"
        | "FUNCTION_KEYWORD" | "TRY_KEYWORD" | "ПОПЫТКА_KEYWORD" | "PREPROC_REGION_KEYWORD" => {
            Some((0, 1))
        }

        // Block closers.
        "ENDIF_KEYWORD" | "ENDDO_KEYWORD" | "КОНЕЦЦИКЛА_KEYWORD" | "ENDTRY_KEYWORD"
        | "КОНЕЦПОПЫТКИ_KEYWORD" | "ENDFUNCTION_KEYWORD" | "ENDPROCEDURE_KEYWORD"
        | "PREPROC_ENDIF_KEYWORD" | "PREPROC_ENDREGION_KEYWORD" => Some((1, 0)),

        // Mid-block keywords: dedent for the keyword line, then indent for the body.
        "ELSE_KEYWORD" | "ELSIF_KEYWORD" | "EXCEPT_KEYWORD" | "ИСКЛЮЧЕНИЕ_KEYWORD"
        | "PREPROC_ELSE_KEYWORD" => Some((1, 1)),

        // Preprocessor elseif: has THEN on the same line (handled above), but still needs dedent.
        "PREPROC_ELSIF_KEYWORD" => Some((1, 0)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_reindents_and_trims() {
        let src = "Процедура Тест()\n  Если Истина Тогда  \nСообщить(\"ok\");\n   Иначе\nСообщить(\"no\");   \nКонецЕсли;\nКонецПроцедуры";
        let out = format_document(src, &FormatOptions::default()).expect("format");
        let expected = "Процедура Тест()\n    Если Истина Тогда\n        Сообщить(\"ok\");\n    Иначе\n        Сообщить(\"no\");\n    КонецЕсли;\nКонецПроцедуры\n";
        assert_eq!(out, expected);
    }

    #[test]
    fn format_handles_preprocessor_blocks() {
        let src = "#Если Истина Тогда\nСообщить(1);\n#Иначе\nСообщить(2);\n#КонецЕсли";
        let out = format_document(src, &FormatOptions::default()).expect("format");
        let expected = "#Если Истина Тогда\n    Сообщить(1);\n#Иначе\n    Сообщить(2);\n#КонецЕсли\n";
        assert_eq!(out, expected);
    }
}
