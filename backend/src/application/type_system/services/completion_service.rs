//! Completion Service - auto-completion operations
//!
//! Functions for LSP completion requests and contextual auto-completion.

use anyhow::Result;
use tracing::info;

use bsl_shared::domain::{CompletionItem, CompletionKind};

use crate::system::ParserCoordinator;
use super::super::extractors::symbol_extractor::{extract_word_at_position, utf16_to_byte_offset};

/// LSP operations - get completion at position
///
/// # Arguments
/// * `parser` - ParserCoordinator for parsing
/// * `file_content` - File content
/// * `line` - Line number (0-based)
/// * `column` - Column number (UTF-16)
///
/// # Returns
/// List of completion items
pub async fn get_completion(
    parser: &ParserCoordinator,
    file_content: &str,
    line: u32,
    column: u32,
) -> Result<Vec<CompletionItem>> {
    info!("Completion request: line {}, column {}", line, column);

    let _parse_result = parser
        .parse(file_content)
        .map_err(|e| anyhow::anyhow!("Parse error for completion: {}", e))?;

    let context = analyze_completion_context(file_content, line, column);
    let mut completions = get_contextual_completions(&context);

    if context.can_add_statements {
        completions.extend(get_basic_bsl_constructs());
    }

    if context.expects_type || context.can_add_statements {
        completions.extend(get_bsl_types());
    }

    if context.can_add_functions {
        completions.extend(get_builtin_functions());
    }

    if completions.is_empty() || completions.len() < 5 {
        completions.extend(get_basic_bsl_constructs());
        completions.extend(get_bsl_types());
        completions.extend(get_builtin_functions());
    }

    Ok(completions)
}

/// Analyzes context for smart auto-completion
///
/// # Arguments
/// * `content` - File content
/// * `line` - Line number (0-based)
/// * `column` - Column number (UTF-16)
///
/// # Returns
/// CompletionContext with analysis results
pub fn analyze_completion_context(
    content: &str,
    line: u32,
    column: u32,
) -> CompletionContext {
    let lines: Vec<&str> = content.lines().collect();
    let line_index = line as usize;

    // Get current line and prefix
    let (_current_line, line_prefix) = if line_index < lines.len() {
        let line_content = lines[line_index];
        // Convert UTF-16 offset -> UTF-8 byte offset
        let column_index = utf16_to_byte_offset(line_content, column);
        (line_content, &line_content[..column_index])
    } else {
        ("", "")
    };

    // Extract current word
    let current_word = extract_word_at_position(content, line, column)
        .unwrap_or_default();

    // Analyze context
    let line_trimmed = line_prefix.trim();

    CompletionContext {
        current_word: current_word.clone(),
        can_add_statements: can_add_statements(line_trimmed),
        expects_type: expects_type_context(line_trimmed),
        can_add_functions: can_add_functions(line_trimmed),
    }
}

/// Checks if statements can be added at this position
fn can_add_statements(line_prefix: &str) -> bool {
    line_prefix.is_empty()
        || line_prefix.ends_with(';')
        || line_prefix.ends_with("Тогда")
        || line_prefix.ends_with("Иначе")
        || line_prefix.ends_with("КонецЕсли")
        || line_prefix.ends_with("КонецЦикла")
        || line_prefix.trim_start().is_empty()
}

/// Checks if a type is expected at this position
fn expects_type_context(line_prefix: &str) -> bool {
    line_prefix.contains(":")
        || line_prefix.contains("Тип(")
        || line_prefix.contains("ТипЗнч(")
        || line_prefix.contains("// ")
}

/// Checks if functions can be added at this position
fn can_add_functions(line_prefix: &str) -> bool {
    !line_prefix.contains("Процедура") && !line_prefix.contains("Функция")
}

/// Gets contextual completions based on context analysis
pub fn get_contextual_completions(context: &CompletionContext) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    // Filter by current word
    if !context.current_word.is_empty() {
        if context.current_word.to_lowercase().starts_with("п") {
            completions.push(CompletionItem {
                label: "Процедура".to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("Объявление процедуры".to_string()),
                documentation: Some("Ключевое слово для объявления процедуры".to_string()),
                insert_text: Some("Процедура ${1:ИмяПроцедуры}(${2:Параметры})\n\t${3:// тело процедуры}\nКонецПроцедуры".to_string()),
                filter_text: Some("Процедура".to_string()),
                sort_text: Some("Процедура".to_string()),
            });
        }

        if context.current_word.to_lowercase().starts_with("с") {
            completions.push(CompletionItem {
                label: "Сообщить".to_string(),
                kind: CompletionKind::Function,
                detail: Some("Вывод сообщения".to_string()),
                documentation: Some("Функция для вывода сообщения пользователю".to_string()),
                insert_text: Some("Сообщить(${1:\"текст\"})".to_string()),
                filter_text: Some("Сообщить".to_string()),
                sort_text: Some("Сообщить".to_string()),
            });
            completions.push(CompletionItem {
                label: "Строка".to_string(),
                kind: CompletionKind::Type,
                detail: Some("Тип данных: строка".to_string()),
                documentation: Some(
                    "Примитивный тип данных для текстовых значений".to_string(),
                ),
                insert_text: Some("Строка".to_string()),
                filter_text: Some("Строка".to_string()),
                sort_text: Some("Строка".to_string()),
            });
        }

        if context.current_word.to_lowercase().starts_with("т") {
            completions.push(CompletionItem {
                label: "ТипЗнч".to_string(),
                kind: CompletionKind::Function,
                detail: Some("Получить тип значения".to_string()),
                documentation: Some(
                    "Функция для получения типа переданного значения".to_string(),
                ),
                insert_text: Some("ТипЗнч(${1:значение})".to_string()),
                filter_text: Some("ТипЗнч".to_string()),
                sort_text: Some("ТипЗнч".to_string()),
            });
        }
    }

    completions
}

/// Gets basic BSL constructs
pub fn get_basic_bsl_constructs() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "Функция".to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("Объявление функции".to_string()),
            documentation: Some("Ключевое слово для объявления функции".to_string()),
            insert_text: Some("Функция ${1:ИмяФункции}(${2:Параметры})\n\t${3:// тело функции}\nКонецФункции".to_string()),
            filter_text: Some("Функция".to_string()),
            sort_text: Some("Функция".to_string()),
        },
        CompletionItem {
            label: "Процедура".to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("Объявление процедуры".to_string()),
            documentation: Some("Ключевое слово для объявления процедуры".to_string()),
            insert_text: Some("Процедура ${1:ИмяПроцедуры}(${2:Параметры})\n\t${3:// тело процедуры}\nКонецПроцедуры".to_string()),
            filter_text: Some("Процедура".to_string()),
            sort_text: Some("Процедура".to_string()),
        },
        CompletionItem {
            label: "Если".to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("Условное выражение".to_string()),
            documentation: Some("Ключевое слово для условного выполнения".to_string()),
            insert_text: Some("Если ${1:условие} Тогда\n\t${2:// действия}\nКонецЕсли".to_string()),
            filter_text: Some("Если".to_string()),
            sort_text: Some("Если".to_string()),
        },
        CompletionItem {
            label: "Для".to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("Цикл Для".to_string()),
            documentation: Some("Ключевое слово для циклического выполнения".to_string()),
            insert_text: Some("Для ${1:Счетчик} = ${2:НачальноеЗначение} По ${3:КонечноеЗначение} Цикл\n\t${4:// тело цикла}\nКонецЦикла".to_string()),
            filter_text: Some("Для".to_string()),
            sort_text: Some("Для".to_string()),
        },
    ]
}

/// Gets BSL data types
pub fn get_bsl_types() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "Строка".to_string(),
            kind: CompletionKind::Type,
            detail: Some("Строковый тип данных".to_string()),
            documentation: Some("Примитивный тип данных для текстовых значений".to_string()),
            insert_text: Some("Строка".to_string()),
            filter_text: Some("Строка".to_string()),
            sort_text: Some("Строка".to_string()),
        },
        CompletionItem {
            label: "Число".to_string(),
            kind: CompletionKind::Type,
            detail: Some("Числовой тип данных".to_string()),
            documentation: Some("Примитивный тип данных для числовых значений".to_string()),
            insert_text: Some("Число".to_string()),
            filter_text: Some("Число".to_string()),
            sort_text: Some("Число".to_string()),
        },
        CompletionItem {
            label: "Булево".to_string(),
            kind: CompletionKind::Type,
            detail: Some("Булевый тип данных".to_string()),
            documentation: Some("Примитивный тип данных для логических значений".to_string()),
            insert_text: Some("Булево".to_string()),
            filter_text: Some("Булево".to_string()),
            sort_text: Some("Булево".to_string()),
        },
        CompletionItem {
            label: "Дата".to_string(),
            kind: CompletionKind::Type,
            detail: Some("Тип данных дата/время".to_string()),
            documentation: Some(
                "Примитивный тип данных для значений даты и времени".to_string(),
            ),
            insert_text: Some("Дата".to_string()),
            filter_text: Some("Дата".to_string()),
            sort_text: Some("Дата".to_string()),
        },
    ]
}

/// Gets built-in BSL functions
pub fn get_builtin_functions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "Сообщить".to_string(),
            kind: CompletionKind::Function,
            detail: Some("Вывести сообщение пользователю".to_string()),
            documentation: Some(
                "Встроенная функция для вывода сообщения пользователю".to_string(),
            ),
            insert_text: Some("Сообщить(${1:\"текст\"})".to_string()),
            filter_text: Some("Сообщить".to_string()),
            sort_text: Some("Сообщить".to_string()),
        },
        CompletionItem {
            label: "ТипЗнч".to_string(),
            kind: CompletionKind::Function,
            detail: Some("Получить тип значения".to_string()),
            documentation: Some("Встроенная функция для получения типа значения".to_string()),
            insert_text: Some("ТипЗнч(${1:значение})".to_string()),
            filter_text: Some("ТипЗнч".to_string()),
            sort_text: Some("ТипЗнч".to_string()),
        },
        CompletionItem {
            label: "СтрДлина".to_string(),
            kind: CompletionKind::Function,
            detail: Some("Получить длину строки".to_string()),
            documentation: Some("Встроенная функция для получения длины строки".to_string()),
            insert_text: Some("СтрДлина(${1:строка})".to_string()),
            filter_text: Some("СтрДлина".to_string()),
            sort_text: Some("СтрДлина".to_string()),
        },
    ]
}

/// Context for auto-completion
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub current_word: String,
    pub can_add_statements: bool,
    pub expects_type: bool,
    pub can_add_functions: bool,
}
