//! Пример использования парсера запросов 1С

use bsl_gradual_types::query::{parse_1c_query, parse_1c_batch_query, QueryTypeChecker};
use bsl_gradual_types::core::context::ContextResolver;

fn main() {
    println!("=== Демонстрация парсера запросов 1С ===\n");
    
    // Пример 1: Простой запрос
    simple_query_example();
    
    // Пример 2: Запрос с временной таблицей
    temp_table_example();
    
    // Пример 3: Пакетный запрос
    batch_query_example();
    
    // Пример 4: Проверка типов
    type_checking_example();
}

fn simple_query_example() {
    println!("1. Простой запрос:");
    
    let query = r#"ВЫБРАТЬ
        |   Номер КАК НомерДокумента,
        |   Дата,
        |   Контрагент.Наименование КАК НазваниеКонтрагента
        |ИЗ
        |   Документ.ПоступлениеТоваровУслуг
        |ГДЕ
        |   Проведен = ИСТИНА
        |   И Дата >= &НачалоПериода"#;
    
    match parse_1c_query(query) {
        Ok(ast) => {
            println!("✓ Парсинг успешен!");
            println!("  - Полей в SELECT: {}", ast.select_clause.fields.len());
            println!("  - Есть WHERE: {}", ast.where_clause.is_some());
            
            for field in &ast.select_clause.fields {
                if let Some(alias) = &field.alias {
                    println!("  - Поле с алиасом: {}", alias);
                }
            }
        }
        Err(e) => println!("✗ Ошибка: {}", e),
    }
    println!();
}

fn temp_table_example() {
    println!("2. Запрос с временной таблицей:");
    
    let query = r#"ВЫБРАТЬ
        |   Товар.Ссылка,
        |   Товар.Наименование,
        |   Товар.Артикул
        |ПОМЕСТИТЬ ВТ_ОтобранныеТовары
        |ИЗ
        |   Справочник.Номенклатура КАК Товар
        |ГДЕ
        |   Товар.ВидНоменклатуры = &ВидНоменклатуры"#;
    
    match parse_1c_query(query) {
        Ok(ast) => {
            println!("✓ Парсинг успешен!");
            if let Some(temp_table) = &ast.select_clause.into_temp_table {
                println!("  - Создаёт временную таблицу: {}", temp_table);
            }
        }
        Err(e) => println!("✗ Ошибка: {}", e),
    }
    println!();
}

fn batch_query_example() {
    println!("3. Пакетный запрос с анализом зависимостей:");
    
    let queries = r#"ВЫБРАТЬ
        |   Товар.Ссылка КАК Номенклатура,
        |   Товар.Наименование
        |ПОМЕСТИТЬ ВТ_Товары
        |ИЗ
        |   Справочник.Номенклатура КАК Товар
        |ГДЕ
        |   НЕ Товар.ЭтоГруппа;
        |
        |ВЫБРАТЬ
        |   Остатки.Номенклатура,
        |   Остатки.Склад,
        |   Остатки.КоличествоОстаток
        |ПОМЕСТИТЬ ВТ_Остатки  
        |ИЗ
        |   РегистрНакопления.ТоварыНаСкладах.Остатки() КАК Остатки
        |ГДЕ
        |   Остатки.Номенклатура В (ВЫБРАТЬ Номенклатура ИЗ ВТ_Товары);
        |
        |ВЫБРАТЬ
        |   Товары.Наименование,
        |   СУММА(Остатки.КоличествоОстаток) КАК ВсегоНаСкладах
        |ИЗ
        |   ВТ_Товары КАК Товары
        |   ЛЕВОЕ СОЕДИНЕНИЕ ВТ_Остатки КАК Остатки
        |   ПО Товары.Номенклатура = Остатки.Номенклатура
        |СГРУППИРОВАТЬ ПО
        |   Товары.Наименование"#;
    
    match parse_1c_batch_query(queries) {
        Ok(batch) => {
            println!("✓ Парсинг успешен!");
            println!("  - Запросов в пакете: {}", batch.queries.len());
            println!("  - Связанный пакет: {}", batch.is_connected);
            println!("  - Можно параллелить: {}", batch.can_parallelize());
            
            println!("\n  Временные таблицы:");
            for (table, index) in &batch.temp_tables {
                println!("    - {} создаётся в запросе {}", table, index + 1);
            }
            
            println!("\n  Зависимости:");
            for part in &batch.queries {
                if !part.uses_temp_tables.is_empty() {
                    println!("    - Запрос {} использует: {:?}", 
                        part.index + 1, part.uses_temp_tables);
                }
            }
            
            println!("\n  Оптимальный порядок выполнения: {:?}", 
                batch.get_execution_order());
        }
        Err(e) => println!("✗ Ошибка: {}", e),
    }
    println!();
}

fn type_checking_example() {
    println!("4. Проверка типов в запросе:");
    
    let query = r#"ВЫБРАТЬ
        Номер,
        Дата,
        СуммаДокумента
    ИЗ
        Документ.ПоступлениеТоваровУслуг"#;
    
    match parse_1c_query(query) {
        Ok(ast) => {
            println!("✓ Парсинг успешен!");
            
            // Проверка типов
            let mut type_checker = QueryTypeChecker::new(ContextResolver);
            let result = type_checker.check_query(&ast);
            
            if result.errors.is_empty() {
                println!("✓ Проверка типов успешна!");
                println!("  Результирующие поля:");
                for field in &result.fields {
                    println!("    - {}: {:?}", field.name, field.type_resolution.certainty);
                }
            } else {
                println!("⚠ Найдены ошибки типов:");
                for error in &result.errors {
                    println!("    - {}", error.message);
                }
            }
        }
        Err(e) => println!("✗ Ошибка парсинга: {}", e),
    }
}