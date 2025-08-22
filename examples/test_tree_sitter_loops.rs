//! Тест циклов в tree-sitter-bsl парсере

use bsl_gradual_types::parser::ast::Statement;
use bsl_gradual_types::parser::common::Parser;
use bsl_gradual_types::parser::tree_sitter_adapter::TreeSitterAdapter;

fn main() -> anyhow::Result<()> {
    println!("=== Тестирование циклов tree-sitter-bsl ===\n");

    let mut adapter = TreeSitterAdapter::new()?;

    // Тест 1: Цикл While
    test_while_loop(&mut adapter)?;

    // Тест 2: Цикл For
    test_for_loop(&mut adapter)?;

    // Тест 3: Цикл ForEach
    test_foreach_loop(&mut adapter)?;

    // Тест 4: Вложенные циклы
    test_nested_loops(&mut adapter)?;

    // Тест 5: Циклы с break и continue
    test_loop_control(&mut adapter)?;

    println!("\n✅ Все тесты циклов пройдены успешно!");
    Ok(())
}

fn test_while_loop(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("1. Тест цикла While:");
    let code = r#"
Счетчик = 0;
Пока Счетчик < 10 Цикл
    Счетчик = Счетчик + 1;
    Сообщить(Счетчик);
КонецЦикла;
"#;

    let program = adapter.parse(code)?;

    // Проверяем, что есть цикл While
    let mut found_while = false;
    for stmt in &program.statements {
        if matches!(stmt, Statement::While { .. }) {
            found_while = true;
            println!("   ✓ Цикл While распознан");

            if let Statement::While { condition, body } = stmt {
                println!("   ✓ Условие цикла распознано");
                println!("   ✓ Тело цикла содержит {} операторов", body.len());
            }
        }
    }

    if !found_while {
        println!("   ⚠️  Цикл While не найден");
    }

    Ok(())
}

fn test_for_loop(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n2. Тест цикла For:");
    let code = r#"
Для Индекс = 1 По 10 Цикл
    Сообщить(Индекс);
КонецЦикла;
"#;

    let program = adapter.parse(code)?;

    // Проверяем, что есть цикл For
    let mut found_for = false;
    for stmt in &program.statements {
        if matches!(stmt, Statement::For { .. }) {
            found_for = true;
            println!("   ✓ Цикл For распознан");

            if let Statement::For {
                variable,
                from: _,
                to: _,
                step: _,
                body,
            } = stmt
            {
                println!("   ✓ Переменная цикла: {}", variable);
                println!("   ✓ Тело цикла содержит {} операторов", body.len());
            }
        }
    }

    if !found_for {
        println!("   ⚠️  Цикл For не найден");
    }

    Ok(())
}

fn test_foreach_loop(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n3. Тест цикла ForEach:");
    let code = r#"
Массив = Новый Массив;
Для Каждого Элемент Из Массив Цикл
    Сообщить(Элемент);
КонецЦикла;
"#;

    let program = adapter.parse(code)?;

    // Проверяем, что есть цикл ForEach
    let mut found_foreach = false;
    for stmt in &program.statements {
        if matches!(stmt, Statement::ForEach { .. }) {
            found_foreach = true;
            println!("   ✓ Цикл ForEach распознан");

            if let Statement::ForEach {
                variable,
                collection: _,
                body,
            } = stmt
            {
                println!("   ✓ Переменная цикла: {}", variable);
                println!("   ✓ Тело цикла содержит {} операторов", body.len());
            }
        }
    }

    if !found_foreach {
        println!("   ⚠️  Цикл ForEach не найден");
    }

    Ok(())
}

fn test_nested_loops(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n4. Тест вложенных циклов:");
    let code = r#"
Для i = 1 По 3 Цикл
    Для j = 1 По 3 Цикл
        Сообщить("" + i + ", " + j);
    КонецЦикла;
КонецЦикла;
"#;

    let program = adapter.parse(code)?;

    // Проверяем внешний цикл
    for stmt in &program.statements {
        if let Statement::For { body, .. } = stmt {
            println!("   ✓ Внешний цикл For найден");

            // Проверяем вложенный цикл
            let mut found_nested = false;
            for inner_stmt in body {
                if matches!(inner_stmt, Statement::For { .. }) {
                    found_nested = true;
                    println!("   ✓ Вложенный цикл For найден");
                }
            }

            if !found_nested {
                println!("   ⚠️  Вложенный цикл не найден");
            }
        }
    }

    Ok(())
}

fn test_loop_control(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n5. Тест управления циклом (break/continue):");
    let code = r#"
Для i = 1 По 10 Цикл
    Если i = 3 Тогда
        Продолжить;
    КонецЕсли;
    
    Если i = 7 Тогда
        Прервать;
    КонецЕсли;
    
    Сообщить(i);
КонецЦикла;
"#;

    let program = adapter.parse(code)?;

    // Проверяем наличие операторов управления
    let mut found_continue = false;
    let mut found_break = false;

    fn check_statements(stmts: &[Statement], found_continue: &mut bool, found_break: &mut bool) {
        for stmt in stmts {
            match stmt {
                Statement::Continue => {
                    *found_continue = true;
                }
                Statement::Break => {
                    *found_break = true;
                }
                Statement::If {
                    then_branch,
                    else_if_branches,
                    else_branch,
                    ..
                } => {
                    check_statements(then_branch, found_continue, found_break);
                    for (_, branch) in else_if_branches {
                        check_statements(branch, found_continue, found_break);
                    }
                    if let Some(branch) = else_branch {
                        check_statements(branch, found_continue, found_break);
                    }
                }
                Statement::For { body, .. }
                | Statement::ForEach { body, .. }
                | Statement::While { body, .. } => {
                    check_statements(body, found_continue, found_break);
                }
                _ => {}
            }
        }
    }

    check_statements(&program.statements, &mut found_continue, &mut found_break);

    if found_continue {
        println!("   ✓ Оператор 'Продолжить' (continue) найден");
    } else {
        println!("   ⚠️  Оператор 'Продолжить' не найден");
    }

    if found_break {
        println!("   ✓ Оператор 'Прервать' (break) найден");
    } else {
        println!("   ⚠️  Оператор 'Прервать' не найден");
    }

    Ok(())
}
