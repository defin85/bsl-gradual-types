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
    use bsl_shared::domain::signature_index::{MethodSignature, SignatureIndex, SignatureSource};
    use bsl_shared::domain::types::ParameterInfo;
    use tower_lsp::lsp_types::Position;

    // ============================================================================
    // Helper structs/functions (вспомогательные типы из lsp_server.rs)
    // ============================================================================

    #[derive(Debug)]
    struct CallContext {
        function_name: String,
        receiver_type: Option<String>,
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
        let mut paren_depth = 0;
        let mut call_start: Option<(usize, usize)> = None; // (line_idx, char_idx)

        // ИСПРАВЛЕНИЕ: если position.line выходит за границы (после последнего \n),
        // используем последнюю существующую строку
        let max_line = if lines.is_empty() {
            return None;
        } else {
            lines.len() - 1
        };
        let search_until_line = position.line.min(max_line as u32) as usize;

        for line_idx in (0..=search_until_line).rev() {
            let line = lines.get(line_idx)?;

            // ИСПРАВЛЕНИЕ: конвертируем UTF-16 позицию в char index
            let end_char_idx = if line_idx == position.line as usize {
                utf16_to_char_index(line, position.character as usize)?
            } else {
                line.chars().count()
            };

            if end_char_idx == 0 {
                continue;
            }

            // Итерируем по СИМВОЛАМ, не по байтам
            let chars: Vec<char> = line.chars().collect();

            for char_idx in (0..end_char_idx).rev() {
                let ch = chars.get(char_idx)?;

                match ch {
                    ')' => paren_depth += 1,
                    '(' => {
                        if paren_depth == 0 {
                            call_start = Some((line_idx, char_idx));
                            break;
                        }
                        paren_depth -= 1;
                    }
                    _ => {}
                }
            }

            if call_start.is_some() {
                break;
            }
        }

        let (line_idx, char_idx) = call_start?;
        let line = lines.get(line_idx)?;

        // ИСПРАВЛЕНИЕ: используем char индексы для извлечения подстроки
        let before_paren: String = line.chars().take(char_idx).collect();

        let (function_name, receiver_type) = extract_function_name(&before_paren)?;

        // ИСПРАВЛЕНИЕ: конвертируем char index обратно в UTF-16 для LSP
        let utf16_char = char_to_utf16_index(line, char_idx);

        Some(CallContext {
            function_name,
            receiver_type,
            call_start: Position {
                line: line_idx as u32,
                character: utf16_char as u32,
            },
        })
    }

    /// Извлечь имя функции из текста перед скобкой
    fn extract_function_name(text: &str) -> Option<(String, Option<String>)> {
        let trimmed = text.trim_end();

        // Сначала ищем точку (для методов объектов)
        if let Some(dot_pos) = trimmed.rfind('.') {
            // Метод объекта: "Объект.Метод"
            let after_dot = &trimmed[dot_pos + 1..];

            // Извлекаем только валидное имя идентификатора после точки
            let method_name = after_dot
                .chars()
                .take_while(|c| {
                    c.is_alphanumeric()
                        || *c == '_'
                        || (*c >= '\u{0410}' && *c <= '\u{044F}')
                        || *c == '\u{0401}'
                        || *c == '\u{0451}'
                })
                .collect::<String>();

            if !method_name.is_empty() {
                return Some((method_name, None));
            }
        }

        // Глобальная функция: извлекаем последний валидный идентификатор
        // Идём с конца и собираем символы пока они валидны для идентификатора
        let function_name = trimmed
            .chars()
            .rev()
            .take_while(|c| {
                c.is_alphanumeric()
                    || *c == '_'
                    || (*c >= '\u{0410}' && *c <= '\u{044F}')
                    || *c == '\u{0401}'
                    || *c == '\u{0451}'
            })
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        if !function_name.is_empty() {
            Some((function_name, None))
        } else {
            None
        }
    }

    /// Определить индекс активного параметра
    fn calculate_active_parameter(content: &str, context: &CallContext, position: Position) -> u32 {
        let lines: Vec<&str> = content.lines().collect();
        let mut param_index = 0;
        let mut paren_depth = 0;
        let mut in_string = false;

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

            // Итерируем по символам
            for char_idx in start_char_idx..end_char_idx {
                if let Some(&ch) = chars.get(char_idx) {
                    match ch {
                        '"' => in_string = !in_string,
                        '(' if !in_string => paren_depth += 1,
                        ')' if !in_string => paren_depth -= 1,
                        ',' if !in_string && paren_depth == 0 => {
                            param_index += 1;
                        }
                        _ => {}
                    }
                }
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

    // ============================================================================
    // ТЕСТЫ: extract_function_name
    // ============================================================================

    #[test]
    fn test_extract_function_name_global_function() {
        let result = extract_function_name("Сообщить");
        assert!(result.is_some());

        let (name, receiver) = result.unwrap();
        assert_eq!(name, "Сообщить");
        assert_eq!(receiver, None);
    }

    #[test]
    fn test_extract_function_name_method_call() {
        let result = extract_function_name("МойОбъект.Метод");
        assert!(result.is_some());

        let (name, _receiver) = result.unwrap();
        assert_eq!(name, "Метод");
    }

    #[test]
    fn test_extract_function_name_with_spaces() {
        let result = extract_function_name("  Сообщить  ");
        assert!(result.is_some());

        let (name, receiver) = result.unwrap();
        assert_eq!(name, "Сообщить");
        assert_eq!(receiver, None);
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

    // ============================================================================
    // ТЕСТЫ: SignatureIndex интеграция
    // ============================================================================

    #[test]
    fn test_signature_index_add_and_find() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Добавить".to_string(),
            owner_type: Some("Массив".to_string()),
            params: vec![ParameterInfo {
                name: "Элемент".to_string(),
                type_name: Some("Произвольный".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            return_type: None,
            source: SignatureSource::Platform,
        };

        index.add_platform_method("Массив".to_string(), sig);

        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Добавить");
    }

    #[test]
    fn test_signature_index_case_insensitive_search() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Добавить".to_string(),
            owner_type: Some("Массив".to_string()),
            params: vec![],
            return_type: None,
            source: SignatureSource::Platform,
        };

        index.add_platform_method("Массив".to_string(), sig);

        assert!(index.find_method("Массив", "добавить").is_some());
        assert!(index.find_method("Массив", "ДОБАВИТЬ").is_some());
        assert!(index.find_method("Массив", "Добавить").is_some());
    }

    #[test]
    fn test_signature_index_global_function() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Сообщить".to_string(),
            owner_type: None,
            params: vec![ParameterInfo {
                name: "Текст".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            }],
            return_type: None,
            source: SignatureSource::Platform,
        };

        index.add_global_function("Сообщить".to_string(), sig);

        let found = index.find_global_function("Сообщить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Сообщить");
    }

    #[test]
    fn test_signature_index_global_function_case_insensitive() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature {
            name: "Сообщить".to_string(),
            owner_type: None,
            params: vec![],
            return_type: None,
            source: SignatureSource::Platform,
        };

        index.add_global_function("Сообщить".to_string(), sig);

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
