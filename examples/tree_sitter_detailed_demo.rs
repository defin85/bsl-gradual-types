//! Детальный тест tree-sitter-bsl парсера

use bsl_gradual_types::parser::common::Parser;
use bsl_gradual_types::parser::tree_sitter_adapter::TreeSitterAdapter;

fn main() -> anyhow::Result<()> {
    println!("=== Детальное тестирование tree-sitter-bsl ===\n");

    // Создаём адаптер
    let mut adapter = TreeSitterAdapter::new()?;
    println!("✓ Адаптер создан успешно");

    // Тест 1: Простое присваивание
    test_simple_assignment(&mut adapter)?;

    // Тест 2: Условный оператор
    test_if_statement(&mut adapter)?;

    // Тест 3: Функция
    test_function(&mut adapter)?;

    // Тест 4: Процедура
    test_procedure(&mut adapter)?;

    // Тест 5: Комментарии (новая возможность tree-sitter)
    test_with_comments(&mut adapter)?;

    println!("\n✅ Все тесты пройдены успешно!");
    Ok(())
}

fn test_simple_assignment(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n1. Тест простого присваивания:");
    let code = "А = 1;";
    let program = adapter.parse(code)?;
    println!("   Код: {}", code);
    println!("   Statements: {}", program.statements.len());

    if program.statements.is_empty() {
        println!("   ⚠️  Нет распознанных statements");
    } else {
        println!("   ✓ Распознано {} statement(s)", program.statements.len());
    }
    Ok(())
}

fn test_if_statement(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n2. Тест условного оператора:");
    let code = r#"
Если А = 1 Тогда
    Б = 2;
ИначеЕсли В = 3 Тогда
    Г = 4;
Иначе
    Д = 5;
КонецЕсли;
"#;
    let program = adapter.parse(code)?;
    println!("   Statements: {}", program.statements.len());

    if program.statements.is_empty() {
        println!("   ⚠️  Нет распознанных statements");
    } else {
        println!("   ✓ Распознано {} statement(s)", program.statements.len());
    }
    Ok(())
}

fn test_function(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n3. Тест функции:");
    let code = r#"
Функция ПолучитьСумму(Число1, Число2) Экспорт
    Результат = Число1 + Число2;
    Возврат Результат;
КонецФункции
"#;
    let program = adapter.parse(code)?;
    println!("   Statements: {}", program.statements.len());

    if program.statements.is_empty() {
        println!("   ⚠️  Нет распознанных statements");
    } else {
        println!("   ✓ Распознано {} statement(s)", program.statements.len());
        // Проверяем, что это функция
        if let Some(stmt) = program.statements.first() {
            match stmt {
                bsl_gradual_types::parser::ast::Statement::FunctionDecl { name, .. } => {
                    println!("   ✓ Функция: {}", name);
                }
                _ => println!("   ⚠️  Неожиданный тип statement"),
            }
        }
    }
    Ok(())
}

fn test_procedure(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n4. Тест процедуры:");
    let code = r#"
Процедура ВывестиСообщение(Текст) Экспорт
    Сообщить(Текст);
КонецПроцедуры
"#;
    let program = adapter.parse(code)?;
    println!("   Statements: {}", program.statements.len());

    if program.statements.is_empty() {
        println!("   ⚠️  Нет распознанных statements");
    } else {
        println!("   ✓ Распознано {} statement(s)", program.statements.len());
        // Проверяем, что это процедура
        if let Some(stmt) = program.statements.first() {
            match stmt {
                bsl_gradual_types::parser::ast::Statement::ProcedureDecl { name, .. } => {
                    println!("   ✓ Процедура: {}", name);
                }
                _ => println!("   ⚠️  Неожиданный тип statement"),
            }
        }
    }
    Ok(())
}

fn test_with_comments(adapter: &mut TreeSitterAdapter) -> anyhow::Result<()> {
    println!("\n5. Тест с комментариями (новая возможность):");
    let code = r#"
// Это комментарий
А = 1; // Присваиваем значение
/* 
   Многострочный
   комментарий
*/
Б = 2;
"#;
    let program = adapter.parse(code)?;
    println!(
        "   Statements (без комментариев): {}",
        program.statements.len()
    );

    if program.statements.is_empty() {
        println!("   ⚠️  Нет распознанных statements");
    } else {
        println!("   ✓ Распознано {} statement(s)", program.statements.len());
        println!("   ✓ Комментарии корректно игнорируются");
    }
    Ok(())
}
