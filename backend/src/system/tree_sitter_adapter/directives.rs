//! Парсинг директив компилятора (&НаСервере, &НаКлиенте, etc.)
//!
//! Директивы компилятора в 1С указывают место выполнения кода:
//! - &НаСервере / &AtServer
//! - &НаКлиенте / &AtClient
//! - &НаСервереБезКонтекста / &AtServerNoContext
//! - &НаКлиентеНаСервереБезКонтекста / &AtClientAtServerNoContext

use crate::parsing::bsl::ast::CompilerDirective;
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
pub fn find_preceding_directive(node: &Node, source: &str) -> Option<CompilerDirective> {
    let mut prev = node.prev_sibling();

    while let Some(sibling) = prev {
        match sibling.kind() {
            "preprocessor" => {
                let text = node_text(&sibling, source);
                debug!("Found preprocessor before function/procedure: '{}'", text);
                return parse_directive(&text);
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

    None
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
