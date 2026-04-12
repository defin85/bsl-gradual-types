//! Парсинг директив компилятора (&НаСервере, &НаКлиенте, etc.)
//!
//! Директивы компилятора в 1С указывают место выполнения кода:
//! - &НаСервере / &AtServer
//! - &НаКлиенте / &AtClient
//! - &НаСервереБезКонтекста / &AtServerNoContext
//! - &НаКлиентеНаСервереБезКонтекста / &AtClientAtServerNoContext

use crate::ast::CompilerDirective;
use tracing::debug;
use tree_sitter::Node;

use super::utils::node_text;

/// Найти директиву компилятора перед функцией/процедурой
///
/// Ищет предыдущий sibling узел с типом "preprocessor" (содержит директивы вида &НаСервере)
/// и парсит его в CompilerDirective.
///
/// # Примеры
///
/// ```bsl
/// &НаСервере
/// Процедура ОбработкаНаСервере()
/// КонецПроцедуры
/// ```
///
/// В этом случае для узла procedure_definition функция найдёт
/// предшествующий "preprocessor" узел с текстом "&НаСервере".
pub fn find_preceding_directive(
    node: &Node,
    source: &str,
) -> Result<Option<CompilerDirective>, String> {
    let mut prev = node.prev_sibling();

    while let Some(sibling) = prev {
        match sibling.kind() {
            "preprocessor" => {
                let text = node_text(&sibling, source)?;
                debug!("Found preprocessor before function/procedure: '{}'", text);
                return Ok(parse_directive(&text));
            }
            // Пропускаем комментарии между директивой и функцией
            "comment" | "line_comment" => {
                prev = sibling.prev_sibling();
            }
            _ => {
                // Встретили другой узел (не комментарий, не препроцессор) - прекращаем поиск
                break;
            }
        }
    }

    Ok(scan_directive_in_source(node, source))
}

/// Парсит текст директивы в CompilerDirective enum
///
/// Поддерживает русские и английские варианты директив:
/// - &НаСервере / &AtServer -> OnServer
/// - &НаСервереБезКонтекста / &AtServerNoContext -> OnServerNoContext
/// - &НаКлиенте / &AtClient -> OnClient
/// - &НаКлиентеНаСервереБезКонтекста / &AtClientAtServerNoContext -> OnClientOnServerNoContext
pub fn parse_directive(text: &str) -> Option<CompilerDirective> {
    let text_lower = text.to_lowercase();
    let text_trimmed = text_lower.trim();

    // Убираем & в начале если есть
    let directive = text_trimmed.strip_prefix('&').unwrap_or(text_trimmed);

    // Порядок важен: сначала более длинные директивы
    if directive.starts_with("наклиентенасерверебезконтекста")
        || directive.starts_with("atclientatservernocontext")
    {
        return Some(CompilerDirective::OnClientOnServerNoContext);
    }

    if directive.starts_with("насерверебезконтекста") || directive.starts_with("atservernocontext")
    {
        return Some(CompilerDirective::OnServerNoContext);
    }

    if directive.starts_with("насервере") || directive.starts_with("atserver") {
        return Some(CompilerDirective::OnServer);
    }

    if directive.starts_with("наклиенте") || directive.starts_with("atclient") {
        return Some(CompilerDirective::OnClient);
    }

    debug!("Unknown compiler directive: '{}'", text);
    None
}

fn scan_directive_in_source(node: &Node, source: &str) -> Option<CompilerDirective> {
    let start_row = node.start_position().row as isize;
    if start_row < 0 {
        return None;
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut row = start_row;
    let mut allow_skip_routine_line = true;
    while row >= 0 {
        let trimmed = lines.get(row as usize)?.trim();
        if trimmed.is_empty() {
            row -= 1;
            continue;
        }
        if trimmed.starts_with("//") {
            row -= 1;
            continue;
        }
        if trimmed.starts_with('&') {
            return parse_directive(trimmed);
        }
        if allow_skip_routine_line && looks_like_routine_declaration(trimmed) {
            allow_skip_routine_line = false;
            row -= 1;
            continue;
        }
        break;
    }

    None
}

fn looks_like_routine_declaration(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("процедура")
        || lower.starts_with("функция")
        || lower.starts_with("procedure")
        || lower.starts_with("function")
}
