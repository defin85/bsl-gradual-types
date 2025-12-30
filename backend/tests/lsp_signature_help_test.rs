//! Integration тесты для LSP SignatureHelp функциональности (Milestone 2.20)
//!
//! Тестируемые компоненты:
//! - backend/src/bin/lsp_server.rs: find_call_context, calculate_active_parameter, extract_function_name
//! - shared/src/domain/signature_index.rs: SignatureIndex, MethodSignature
//!
//! Проверяемая функциональность:
//! - Парсинг контекста вызова функции
//! - Определение активного параметра
//! - Извлечение имени функции
//! - Case-insensitive поиск методов (кириллица)

#[cfg(test)]
mod lsp_signature_help_tests {
    use bsl_shared::domain::signature_index::{
        ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
    };
    use bsl_shared::domain::type_id::TypeId;
    use bsl_shared::domain::types::ParameterInfo;
    use tower_lsp::lsp_types::Position;

    // ============================================================================
    // Helper structs/functions (вспомогательные типы из lsp_server.rs)
    // ============================================================================

    #[derive(Debug)]
    struct CallContext {
        function_name: String,
        receiver_type: Option<String>,
        is_constructor: bool,
        call_start: Position,
    }

    /// Конвертировать UTF-16 code unit index в char index
    ///
    /// LSP позиции используют UTF-16 code units (из-за VSCode/TypeScript),
    /// а Rust строки используют UTF-8 байты и char индексы.
    fn utf16_to_char_index(text: &str, utf16_index: usize) -> Option<usize> {
        let mut current_utf16 = 0;

        for (char_idx, ch) in text.chars().enumerate() {
            if current_utf16 >= utf16_index {
                return Some(char_idx);
            }
            current_utf16 += ch.len_utf16();
        }

        if current_utf16 == utf16_index {
            Some(text.chars().count())
        } else {
            None
        }
    }

    /// Конвертировать char index в UTF-16 code unit index
    fn char_to_utf16_index(text: &str, char_index: usize) -> usize {
        text.chars().take(char_index).map(|ch| ch.len_utf16()).sum()
    }

    /// Получить UTF-16 длину строки (для тестов)
    fn utf16_len(text: &str) -> usize {
        text.chars().map(|ch| ch.len_utf16()).sum()
    }

    /// Парсит контекст вызова функции - находит открывающую скобку
    fn find_call_context(content: &str, position: Position) -> Option<CallContext> {
        let lines: Vec<&str> = content.lines().collect();
        let max_line = if lines.is_empty() {
            return None;
        } else {
            lines.len() - 1
        };
        let search_until_line = position.line.min(max_line as u32) as usize;

        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut in_string = false;
        let mut in_block_comment = false;

        for line_idx in 0..=search_until_line {
            let line = lines.get(line_idx)?;

            let end_char_idx = if line_idx == position.line as usize {
                utf16_to_char_index(line, position.character as usize)?
            } else {
                line.chars().count()
            };

            if end_char_idx == 0 {
                continue;
            }

            let chars: Vec<char> = line.chars().collect();
            let mut char_idx = 0;

            while char_idx < end_char_idx {
                let ch = chars.get(char_idx).copied()?;
                let next = chars.get(char_idx + 1).copied();

                if in_string {
                    if ch == '"' {
                        if next == Some('"') {
                            char_idx += 2;
                            continue;
                        }
                        in_string = false;
                    }
                    char_idx += 1;
                    continue;
                }

                if in_block_comment {
                    if ch == '*' && next == Some('/') {
                        in_block_comment = false;
                        char_idx += 2;
                        continue;
                    }
                    char_idx += 1;
                    continue;
                }

                if ch == '/' && next == Some('/') {
                    break;
                }

                if ch == '/' && next == Some('*') {
                    in_block_comment = true;
                    char_idx += 2;
                    continue;
                }

                if ch == '"' {
                    in_string = true;
                    char_idx += 1;
                    continue;
                }

                match ch {
                    '(' => stack.push((line_idx, char_idx)),
                    ')' => {
                        stack.pop();
                    }
                    _ => {}
                }

                char_idx += 1;
            }
        }

        let (line_idx, char_idx) = stack.pop()?;
        let line = lines.get(line_idx)?;

        // ИСПРАВЛЕНИЕ: используем char индексы для извлечения подстроки
        let before_paren: String = line.chars().take(char_idx).collect();

        let (function_name, receiver_type, is_constructor) = extract_function_name(&before_paren)?;

        // ИСПРАВЛЕНИЕ: конвертируем char index обратно в UTF-16 для LSP
        let utf16_char = char_to_utf16_index(line, char_idx);

        Some(CallContext {
            function_name,
            receiver_type,
            is_constructor,
            call_start: Position {
                line: line_idx as u32,
                character: utf16_char as u32,
            },
        })
    }

    /// Извлечь имя функции из текста перед скобкой
    fn extract_function_name(text: &str) -> Option<(String, Option<String>, bool)> {
        let trimmed = text.trim_end();

        if let Some(constructor_name) = extract_constructor_name(trimmed) {
            return Some((constructor_name, None, true));
        }

        // Сначала ищем точку (для методов объектов)
        if let Some(dot_pos) = trimmed.rfind('.') {
            // Метод объекта: "Объект.Метод"
            let after_dot = trimmed[dot_pos + 1..].trim_start();

            // Извлекаем только валидное имя идентификатора после точки
            let method_name = after_dot
                .chars()
                .take_while(|c| is_identifier_char(*c))
                .collect::<String>();

            if !method_name.is_empty() {
                let receiver = trimmed[..dot_pos].trim_end();
                let receiver_compact: String =
                    receiver.chars().filter(|c| !c.is_whitespace()).collect();
                let receiver_type = if is_simple_receiver(&receiver_compact) {
                    Some(receiver_compact)
                } else {
                    None
                };
                return Some((method_name, receiver_type, false));
            }
        }

        // Глобальная функция: извлекаем последний валидный идентификатор
        // Идём с конца и собираем символы пока они валидны для идентификатора
        let function_name = trimmed
            .chars()
            .rev()
            .take_while(|c| is_identifier_char(*c))
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        if !function_name.is_empty() {
            if is_control_keyword(&function_name) {
                return None;
            }
            Some((function_name, None, false))
        } else {
            None
        }
    }

    fn extract_constructor_name(text: &str) -> Option<String> {
        let mut iter = text.split_whitespace();
        let keyword = iter.next()?;
        if keyword.to_lowercase() != "новый" {
            return None;
        }
        let remainder: String = iter.collect::<Vec<_>>().join(" ");
        if remainder.is_empty() {
            return None;
        }
        let normalized: String = remainder.chars().filter(|c| !c.is_whitespace()).collect();
        if is_simple_receiver(&normalized) {
            Some(normalized)
        } else {
            None
        }
    }

    fn is_control_keyword(value: &str) -> bool {
        matches!(
            value.to_lowercase().as_str(),
            "если"
                | "иначеесли"
                | "пока"
                | "для"
                | "каждого"
                | "попытка"
                | "исключение"
                | "конецесли"
                | "конеццикла"
                | "конецпопытки"
                | "конецпроцедуры"
                | "конецфункции"
                | "возврат"
                | "выбор"
                | "когда"
                | "иначе"
        )
    }

    fn is_simple_receiver(text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        text.chars().all(|c| c == '.' || is_identifier_char(c))
    }

    fn is_identifier_char(c: char) -> bool {
        c.is_alphanumeric()
            || c == '_'
            || (c >= '\u{0410}' && c <= '\u{044F}')
            || c == '\u{0401}'
            || c == '\u{0451}'
    }

    /// Определить индекс активного параметра
    fn calculate_active_parameter(content: &str, context: &CallContext, position: Position) -> u32 {
        let lines: Vec<&str> = content.lines().collect();
        let mut param_index = 0;
        let mut paren_depth = 0;
        let mut in_string = false;
        let mut in_block_comment = false;

        for line_idx in context.call_start.line..=position.line {
            let line = match lines.get(line_idx as usize) {
                Some(l) => l,
                None => break,
            };
            let chars: Vec<char> = line.chars().collect();

            // ИСПРАВЛЕНИЕ: конвертируем UTF-16 в char indices
            let start_char_idx = if line_idx == context.call_start.line {
                utf16_to_char_index(line, (context.call_start.character + 1) as usize).unwrap_or(0)
            } else {
                0
            };

            let end_char_idx = if line_idx == position.line {
                utf16_to_char_index(line, position.character as usize).unwrap_or(chars.len())
            } else {
                chars.len()
            };

            if end_char_idx < start_char_idx {
                continue;
            }

            let mut char_idx = start_char_idx;
            while char_idx < end_char_idx {
                let ch = match chars.get(char_idx) {
                    Some(ch) => *ch,
                    None => break,
                };
                let next = chars.get(char_idx + 1).copied();

                if in_string {
                    if ch == '"' {
                        if next == Some('"') {
                            char_idx += 2;
                            continue;
                        }
                        in_string = false;
                    }
                    char_idx += 1;
                    continue;
                }

                if in_block_comment {
                    if ch == '*' && next == Some('/') {
                        in_block_comment = false;
                        char_idx += 2;
                        continue;
                    }
                    char_idx += 1;
                    continue;
                }

                if ch == '/' && next == Some('/') {
                    break;
                }

                if ch == '/' && next == Some('*') {
                    in_block_comment = true;
                    char_idx += 2;
                    continue;
                }

                if ch == '"' {
                    in_string = true;
                    char_idx += 1;
                    continue;
                }

                match ch {
                    '(' => paren_depth += 1,
                    ')' => {
                        if paren_depth > 0 {
                            paren_depth -= 1;
                        }
                    }
                    ',' if paren_depth == 0 => {
                        param_index += 1;
                    }
                    _ => {}
                }

                char_idx += 1;
            }
        }

        param_index
    }

    // ============================================================================
    // ТЕСТЫ: find_call_context
    // ============================================================================

    #[test]
    fn test_find_call_context_simple_function() {
        let code = "Сообщить(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Сообщить");
        assert_eq!(ctx.receiver_type, None);
    }

    #[test]
    fn test_find_call_context_with_parameters() {
        let code = "Сообщить(\"текст\", ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Сообщить");
        assert_eq!(ctx.receiver_type, None);
    }

    #[test]
    fn test_find_call_context_method_call() {
        let code = "МойОбъект.Метод(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Метод");
    }

    #[test]
    fn test_find_call_context_nested_calls() {
        let code = "Функция1(Функция2(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Функция2");
        assert_eq!(ctx.receiver_type, None);
    }

    #[test]
    fn test_find_call_context_multiline() {
        let code = "Сообщить(\n  \"текст\"\n";
        let position = Position {
            line: 2,
            character: 0,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Сообщить");
    }

    #[test]
    fn test_find_call_context_no_call() {
        let code = "x = 5;";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_none());
    }

    // ============================================================================
    // ТЕСТЫ: calculate_active_parameter
    // ============================================================================

    #[test]
    fn test_calculate_active_parameter_first_param() {
        let code = "Функция(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 0);
    }

    #[test]
    fn test_calculate_active_parameter_second_param() {
        let code = "Функция(1, ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 1);
    }

    #[test]
    fn test_calculate_active_parameter_third_param() {
        let code = "Функция(1, 2, ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 2);
    }

    #[test]
    fn test_calculate_active_parameter_with_string_containing_comma() {
        let code = "Функция(\"a,b\", ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 1);
    }

    #[test]
    fn test_calculate_active_parameter_with_escaped_quote() {
        let code = "Функция(\"a\"\"b\", ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 1);
    }

    #[test]
    fn test_calculate_active_parameter_with_nested_call() {
        let code = "Функция(Другая(), ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 1);
    }

    #[test]
    fn test_calculate_active_parameter_ignores_comment_commas() {
        let code = "Функция(1, // коммент, \n 2, ";
        let position = Position {
            line: 1,
            character: utf16_len(" 2, ") as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);
        assert_eq!(active_param, 2);
    }

    // ============================================================================
    // ТЕСТЫ: extract_function_name
    // ============================================================================

    #[test]
    fn test_extract_function_name_global_function() {
        let result = extract_function_name("Сообщить");
        assert!(result.is_some());

        let (name, receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "Сообщить");
        assert_eq!(receiver, None);
        assert!(!is_constructor);
    }

    #[test]
    fn test_extract_function_name_method_call() {
        let result = extract_function_name("МойОбъект.Метод");
        assert!(result.is_some());

        let (name, _receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "Метод");
        assert!(!is_constructor);
    }

    #[test]
    fn test_extract_function_name_with_receiver_type() {
        let result = extract_function_name("Справочники.Номенклатура.СоздатьЭлемент");
        assert!(result.is_some());

        let (name, receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "СоздатьЭлемент");
        assert_eq!(receiver, Some("Справочники.Номенклатура".to_string()));
        assert!(!is_constructor);
    }

    #[test]
    fn test_find_call_context_ignores_comment_paren() {
        let code = "// (\nФункция(";
        let position = Position {
            line: 1,
            character: utf16_len("Функция(") as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());
        assert_eq!(context.unwrap().function_name, "Функция");
    }

    #[test]
    fn test_extract_function_name_with_spaces() {
        let result = extract_function_name("  Сообщить  ");
        assert!(result.is_some());

        let (name, receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "Сообщить");
        assert_eq!(receiver, None);
        assert!(!is_constructor);
    }

    #[test]
    fn test_extract_function_name_case_insensitive() {
        let result1 = extract_function_name("СООБЩИТЬ");
        let result2 = extract_function_name("Сообщить");
        let result3 = extract_function_name("сообщить");

        assert_eq!(result1.unwrap().0, "СООБЩИТЬ");
        assert_eq!(result2.unwrap().0, "Сообщить");
        assert_eq!(result3.unwrap().0, "сообщить");
    }

    #[test]
    fn test_extract_function_name_with_spaces_around_dot() {
        let result = extract_function_name("Справочники . Номенклатура . СоздатьЭлемент");
        assert!(result.is_some());

        let (name, receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "СоздатьЭлемент");
        assert_eq!(receiver, Some("Справочники.Номенклатура".to_string()));
        assert!(!is_constructor);
    }

    #[test]
    fn test_extract_constructor_name() {
        let result = extract_function_name("Новый Массив");
        assert!(result.is_some());

        let (name, receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "Массив");
        assert!(receiver.is_none());
        assert!(is_constructor);
    }

    #[test]
    fn test_extract_constructor_name_qualified_with_spaces() {
        let result = extract_function_name("Новый Справочники . Номенклатура");
        assert!(result.is_some());

        let (name, receiver, is_constructor) = result.unwrap();
        assert_eq!(name, "Справочники.Номенклатура");
        assert!(receiver.is_none());
        assert!(is_constructor);
    }

    #[test]
    fn test_extract_function_name_skips_control_keyword() {
        let result = extract_function_name("Если");
        assert!(result.is_none());
    }

    // ============================================================================
    // ТЕСТЫ: SignatureIndex интеграция
    // ============================================================================

    #[test]
    fn test_signature_index_add_and_find() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![ParameterInfo {
                name: "Элемент".to_string(),
                type_name: Some("Произвольный".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("Массив"), sig);

        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Добавить");
    }

    #[test]
    fn test_signature_index_case_insensitive_search() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("Массив"), sig);

        assert!(index.find_method("Массив", "добавить").is_some());
        assert!(index.find_method("Массив", "ДОБАВИТЬ").is_some());
        assert!(index.find_method("Массив", "Добавить").is_some());
    }

    #[test]
    fn test_signature_index_global_function() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Сообщить".to_string(),
            None,
            vec![ParameterInfo {
                name: "Текст".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_global_function(TypeId::new("Сообщить"), sig);

        let found = index.find_global_function("Сообщить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Сообщить");
    }

    #[test]
    fn test_signature_index_global_function_case_insensitive() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Сообщить".to_string(),
            None,
            vec![],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_global_function(TypeId::new("Сообщить"), sig);

        assert!(index.find_global_function("сообщить").is_some());
        assert!(index.find_global_function("СООБЩИТЬ").is_some());
        assert!(index.find_global_function("Сообщить").is_some());
    }

    // ============================================================================
    // EDGE CASES И ГРАНИЧНЫЕ СЛУЧАИ
    // ============================================================================

    #[test]
    fn test_edge_case_empty_code() {
        let code = "";
        let position = Position {
            line: 0,
            character: 0,
        };

        let context = find_call_context(code, position);
        assert!(context.is_none());
    }

    #[test]
    fn test_edge_case_mismatched_parens() {
        let code = "Функция1)Функция2(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Функция2");
    }

    #[test]
    fn test_edge_case_unicode_function_names() {
        let code = "МойМетод(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "МойМетод");
    }

    #[test]
    fn test_edge_case_method_with_underscores() {
        let code = "Объект.Мой_Метод(";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.function_name, "Мой_Метод");
    }

    #[test]
    fn test_edge_case_very_long_parameter_list() {
        let code = "Функция(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);

        assert_eq!(active_param, 10);
    }

    #[test]
    fn test_edge_case_string_with_escaped_quotes() {
        let code = "Функция(\"строка \"\"с кавычками\"\"\", ";
        let position = Position {
            line: 0,
            character: utf16_len(code) as u32,
        };

        let context = find_call_context(code, position).unwrap();
        let active_param = calculate_active_parameter(code, &context, position);

        assert_eq!(active_param, 1);
    }
}
