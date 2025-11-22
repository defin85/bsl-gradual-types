#!/usr/bin/env cargo run --bin lsp-server
//! Демонстрация нового LSP Server с реальной функциональностью
//!
//! Этот файл показывает, как работает наш улучшенный LSP Server

use anyhow::Result;
use bsl_gradual_types::system::SystemCoordinator;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Демонстрация LSP Server с реальной функциональностью");
    println!();

    // Создаем координатор системы (IoC Container)
    let coordinator = Arc::new(SystemCoordinator::new());
    let type_service = coordinator.type_service();

    // === ДЕМОНСТРАЦИЯ HOVER ===
    println!("🎯 1. Демонстрация Hover функциональности:");
    let bsl_code = r#"
Функция ВычислитьСумму(А, Б)
    Результат = А + Б;
    Возврат Результат;
КонецФункции
"#;

    match type_service.get_hover_info(bsl_code, 2, 5, None).await {
        Ok(Some(info)) => println!("  ✅ Hover: {}", info),
        Ok(None) => println!("  ❌ Hover: информация не найдена"),
        Err(e) => println!("  ❌ Hover error: {}", e),
    }

    println!();

    // === ДЕМОНСТРАЦИЯ COMPLETION ===
    println!("🎯 2. Демонстрация Completion функциональности:");
    let incomplete_code = r#"
Функция НоваяФункция()
    // здесь нужно автодополнение
"#;

    match type_service.get_completion(incomplete_code, 2, 4).await {
        Ok(completions) => {
            println!("  ✅ Доступно {} вариантов автодополнения:", completions.len());
            for (i, completion) in completions.iter().enumerate().take(5) {
                println!("    {}. {}", i + 1, completion.label);
                if let Some(detail) = &completion.detail {
                    println!("       {}", detail);
                }
            }
            if completions.len() > 5 {
                println!("    ... и ещё {} вариантов", completions.len() - 5);
            }
        }
        Err(e) => println!("  ❌ Completion error: {}", e),
    }

    println!();

    // === ДЕМОНСТРАЦИЯ АНАЛИЗА ФАЙЛА ===
    println!("🎯 3. Демонстрация анализа файла:");

    // Создаем временный BSL файл
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new()?;
    writeln!(
        temp_file,
        r#"
// Пример BSL модуля с типами
Функция ПолучитьТипДанных(Значение)
    ТипЗначения = ТипЗнч(Значение);
    
    Если ТипЗначения = Тип("Строка") Тогда
        Возврат "Строковый тип";
    ИначеЕсли ТипЗначения = Тип("Число") Тогда
        Возврат "Числовой тип";
    Иначе
        Возврат "Неизвестный тип";
    КонецЕсли;
КонецФункции

Процедура ДемонстрацияТипов()
    Стр = "Привет, мир!";
    Число = 42;
    Дата = ТекущаяДата();
    
    Сообщить("Тип строки: " + ПолучитьТипДанных(Стр));
    Сообщить("Тип числа: " + ПолучитьТипДанных(Число));
    Сообщить("Тип даты: " + ПолучитьТипДанных(Дата));
КонецПроцедуры
"#
    )?;

    let file_path = temp_file.path().to_string_lossy().to_string();

    match type_service.analyze_file(&file_path).await {
        Ok(analysis) => {
            println!("  ✅ Анализ файла успешен:");
            println!("     Путь: {}", analysis.file_path);
            println!("     Время анализа: {} мс", analysis.analysis_duration_ms);
        }
        Err(e) => println!("  ❌ Ошибка анализа: {}", e),
    }

    println!();
    println!("🎉 Демонстрация завершена!");
    println!();
    println!("📋 Возможности нашего LSP Server:");
    println!("  ✅ Hover - показывает информацию о символах");
    println!("  ✅ Completion - автодополнение BSL конструкций");
    println!("  ✅ Diagnostics - анализ и диагностика файлов");
    println!("  ✅ Clean Architecture - правильное разделение слоев");

    Ok(())
}
