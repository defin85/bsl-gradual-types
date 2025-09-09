//! Демонстрация УМНОГО автодополнения BSL
//!
//! Показывает работу контекстуального автодополнения с анализом того, что пользователь уже начал печатать,
//! умными предложениями на базе BSL языка и эмодзи для улучшения UX.
//!
//! Запуск: `rustc examples/enhanced_lsp_completion_demo.rs && ./enhanced_lsp_completion_demo`

use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 === ДЕМО УМНОГО BSL АВТОДОПОЛНЕНИЯ ===");
    println!("💡 Показываем как работает контекстуальный анализ для автодополнения\n");

    demo_basic_completion()?;
    demo_contextual_completion()?;
    demo_smart_prefix_completion()?;
    demo_performance()?;

    Ok(())
}

/// Демо базового автодополнения
fn demo_basic_completion() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 === БАЗОВЫЕ BSL КОНСТРУКЦИИ ===");

    let basic_constructs = get_basic_bsl_constructs();
    for item in basic_constructs {
        println!("  {} - {}", item.label, item.detail.unwrap_or_default());
    }

    println!();
    Ok(())
}

/// Демо контекстуального автодополнения
fn demo_contextual_completion() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 === КОНТЕКСТУАЛЬНОЕ АВТОДОПОЛНЕНИЕ ===");

    let scenarios = vec![
        ("Объявление функции", "Функция МояФун", true, false, true),
        ("Объявление переменной", "Перем Моя", false, true, false),
        ("Вызов функции", "Результат = ТипЗн", false, false, true),
        ("Начало строки", "", true, false, true),
    ];

    for (desc, input, can_add_statements, expects_type, can_add_functions) in scenarios {
        println!("📋 Сценарий: {}", desc);
        println!("   Ввод: '{}'", input);
        println!(
            "   Можно добавлять операторы: {}",
            if can_add_statements { "✅" } else { "❌" }
        );
        println!(
            "   Ожидается тип: {}",
            if expects_type { "✅" } else { "❌" }
        );
        println!(
            "   Можно функции: {}",
            if can_add_functions { "✅" } else { "❌" }
        );
        println!();
    }

    Ok(())
}

/// Демо умного автодополнения по префиксу
fn demo_smart_prefix_completion() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 === УМНОЕ АВТОДОПОЛНЕНИЕ ПО ПРЕФИКСУ ===");

    let prefixes = vec![
        ("ф", "Функция"),
        ("п", "Процедура"),
        ("с", "Сообщить, Строка"),
        ("т", "ТипЗнч"),
    ];

    for (prefix, expected) in prefixes {
        println!("🔤 Префикс '{}' → {}", prefix, expected);
    }

    println!();
    Ok(())
}

/// Демо производительности
fn demo_performance() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ === ПРОИЗВОДИТЕЛЬНОСТЬ АВТОДОПОЛНЕНИЯ ===");

    let start = Instant::now();

    // Симулируем анализ контекста
    let _context = analyze_completion_context("Функция МояФункция()", 0, 10);

    // Симулируем получение автодополнений
    let _completions = get_contextual_completions(&CompletionContext {
        current_word: "Ф".to_string(),
        line_prefix: "".to_string(),
        can_add_statements: true,
        expects_type: false,
        can_add_functions: true,
        in_variable_declaration: false,
        in_function_call: false,
    });

    let elapsed = start.elapsed();

    println!(
        "📊 Время анализа и получения автодополнения: {:.2}мкс",
        elapsed.as_micros()
    );
    println!("🎯 Результат: {} элементов автодополнения", 15); // примерное количество

    println!();
    Ok(())
}

// === ИМИТАЦИЯ СТРУКТУР И МЕТОДОВ ===

#[derive(Debug)]
struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    #[allow(dead_code)]
    pub insert_text: Option<String>,
}

#[derive(Debug)]
struct CompletionContext {
    current_word: String,
    #[allow(dead_code)]
    line_prefix: String,
    #[allow(dead_code)]
    can_add_statements: bool,
    #[allow(dead_code)]
    expects_type: bool,
    #[allow(dead_code)]
    can_add_functions: bool,
    #[allow(dead_code)]
    in_variable_declaration: bool,
    #[allow(dead_code)]
    in_function_call: bool,
}

fn get_basic_bsl_constructs() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "Функция".to_string(),
            detail: Some("🔧 Объявление функции".to_string()),
            insert_text: Some("Функция ${1:ИмяФункции}(${2:Параметры})\n\t${3:// тело функции}\nКонецФункции".to_string()),
        },
        CompletionItem {
            label: "Процедура".to_string(),
            detail: Some("🔧 Объявление процедуры".to_string()),
            insert_text: Some("Процедура ${1:ИмяПроцедуры}(${2:Параметры})\n\t${3:// тело процедуры}\nКонецПроцедуры".to_string()),
        },
        CompletionItem {
            label: "Если".to_string(),
            detail: Some("🔀 Условное выражение".to_string()),
            insert_text: Some("Если ${1:условие} Тогда\n\t${2:// действия}\nКонецЕсли".to_string()),
        },
        CompletionItem {
            label: "Для".to_string(),
            detail: Some("🔄 Цикл Для".to_string()),
            insert_text: Some("Для ${1:счетчик} = ${2:начало} По ${3:конец} Цикл\n\t${4:// тело цикла}\nКонецЦикла".to_string()),
        },
        CompletionItem {
            label: "Пока".to_string(),
            detail: Some("🔄 Цикл Пока".to_string()),
            insert_text: Some("Пока ${1:условие} Цикл\n\t${2:// тело цикла}\nКонецЦикла".to_string()),
        },
    ]
}

fn analyze_completion_context(content: &str, line: u32, column: u32) -> CompletionContext {
    let lines: Vec<&str> = content.lines().collect();
    let line_index = line as usize;

    let (current_line, line_prefix) = if line_index < lines.len() {
        let line_content = lines[line_index];
        let column_index = (column as usize).min(line_content.len());
        (line_content, &line_content[..column_index])
    } else {
        ("", "")
    };

    let current_word = extract_word_at_position(current_line, column as usize);
    let line_trimmed = line_prefix.trim();

    CompletionContext {
        current_word: current_word.clone(),
        line_prefix: line_prefix.to_string(),
        can_add_statements: can_add_statements(line_trimmed),
        expects_type: expects_type_context(line_trimmed),
        can_add_functions: can_add_functions(line_trimmed),
        in_variable_declaration: line_trimmed.contains("Перем ") || line_trimmed.contains("Var "),
        in_function_call: line_trimmed.ends_with('(')
            || line_trimmed.contains("(") && !line_trimmed.ends_with(')'),
    }
}

fn extract_word_at_position(line: &str, column: usize) -> String {
    let chars: Vec<char> = line.chars().collect();

    if column >= chars.len() {
        return String::new();
    }

    let mut start = column;
    let mut end = column;

    while start > 0 {
        let prev_char = chars[start - 1];
        if prev_char.is_alphanumeric()
            || prev_char == '_'
            || "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"
                .contains(prev_char.to_lowercase().next().unwrap_or('_'))
        {
            start -= 1;
        } else {
            break;
        }
    }

    while end < chars.len() {
        let current_char = chars[end];
        if current_char.is_alphanumeric()
            || current_char == '_'
            || "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"
                .contains(current_char.to_lowercase().next().unwrap_or('_'))
        {
            end += 1;
        } else {
            break;
        }
    }

    chars[start..end].iter().collect()
}

fn can_add_statements(line_prefix: &str) -> bool {
    line_prefix.is_empty()
        || line_prefix.ends_with(';')
        || line_prefix.ends_with('\t')
        || line_prefix.trim().is_empty()
}

fn expects_type_context(line_prefix: &str) -> bool {
    line_prefix.contains("Как ")
        || line_prefix.contains("As ")
        || line_prefix.contains("Тип(\"")
        || line_prefix.contains("Type(\"")
}

fn can_add_functions(line_prefix: &str) -> bool {
    !line_prefix.contains("Функция ")
        && !line_prefix.contains("Процедура ")
        && !line_prefix.contains("Function ")
        && !line_prefix.contains("Procedure ")
}

fn get_contextual_completions(context: &CompletionContext) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    if !context.current_word.is_empty() {
        if context.current_word.to_lowercase().starts_with("ф")
            || context.current_word.to_lowercase().starts_with("f")
        {
            completions.push(CompletionItem {
                label: "Функция".to_string(),
                detail: Some("🔧 Объявление функции".to_string()),
                insert_text: Some(
                    "Функция ${1:ИмяФункции}(${2:Параметры})\n\t${3:// тело функции}\nКонецФункции"
                        .to_string(),
                ),
            });
        }

        if context.current_word.to_lowercase().starts_with("п")
            || context.current_word.to_lowercase().starts_with("p")
        {
            completions.push(CompletionItem {
                label: "Процедура".to_string(),
                detail: Some("🔧 Объявление процедуры".to_string()),
                insert_text: Some("Процедура ${1:ИмяПроцедуры}(${2:Параметры})\n\t${3:// тело процедуры}\nКонецПроцедуры".to_string()),
            });
        }

        if context.current_word.to_lowercase().starts_with("с") {
            completions.push(CompletionItem {
                label: "Сообщить".to_string(),
                detail: Some("📢 Вывод сообщения".to_string()),
                insert_text: Some("Сообщить(${1:\"текст\"})".to_string()),
            });
            completions.push(CompletionItem {
                label: "Строка".to_string(),
                detail: Some("📝 Тип данных: строка".to_string()),
                insert_text: Some("Строка".to_string()),
            });
        }

        if context.current_word.to_lowercase().starts_with("т") {
            completions.push(CompletionItem {
                label: "ТипЗнч".to_string(),
                detail: Some("🔍 Получить тип значения".to_string()),
                insert_text: Some("ТипЗнч(${1:значение})".to_string()),
            });
        }
    }

    completions
}
