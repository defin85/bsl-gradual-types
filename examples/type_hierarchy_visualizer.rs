//! Визуализатор иерархии типов из синтакс-помощника
//! 
//! Показывает структуру типов, методов и свойств в удобном виде

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use bsl_gradual_types::adapters::platform_types_v2::create_platform_resolver_with_syntax_helper;
use std::collections::BTreeMap;
use colored::Colorize;

fn main() -> anyhow::Result<()> {
    println!("{}", "=== ВИЗУАЛИЗАЦИЯ ИЕРАРХИИ ТИПОВ BSL ===".cyan().bold());
    println!();
    
    // Загружаем данные из сохранённого JSON
    let json_path = "examples/syntax_helper/syntax_database.json";
    if !std::path::Path::new(json_path).exists() {
        println!("{}","❌ База данных не найдена. Запустите сначала:".red());
        println!("   cargo run --example syntax_helper_parser_demo");
        return Ok(());
    }
    
    let database = SyntaxHelperParser::load_from_file(json_path)?;
    
    // 1. ГЛОБАЛЬНЫЕ ФУНКЦИИ
    println!("{}", "📦 ГЛОБАЛЬНЫЕ ФУНКЦИИ".green().bold());
    println!("{}", "─".repeat(80).dimmed());
    
    // Группируем функции по первой букве для удобства
    let mut functions_by_letter: BTreeMap<char, Vec<String>> = BTreeMap::new();
    
    for (name, func) in &database.global_functions {
        let first_char = name.chars().next().unwrap_or('?');
        functions_by_letter.entry(first_char).or_default().push(
            if let Some(eng) = &func.english_name {
                format!("{} ({})", name, eng.dimmed())
            } else {
                name.clone()
            }
        );
    }
    
    // Показываем только первые несколько букв для краткости
    let mut shown_letters = 0;
    for (letter, functions) in &functions_by_letter {
        if shown_letters >= 5 {
            println!("   {} ({} групп)", "...и ещё".dimmed(), functions_by_letter.len() - 5);
            break;
        }
        
        println!("   {} [{}]:", letter.to_string().yellow(), functions.len());
        for func in functions.iter().take(5) {
            println!("      ├─ {}", func);
        }
        if functions.len() > 5 {
            println!("      └─ {} {} функций", "...ещё".dimmed(), functions.len() - 5);
        }
        shown_letters += 1;
    }
    
    println!("\n   {}: {}", "Всего глобальных функций".bold(), database.global_functions.len());
    
    // 2. КЛЮЧЕВЫЕ СЛОВА
    println!("\n{}", "🔤 КЛЮЧЕВЫЕ СЛОВА ЯЗЫКА".green().bold());
    println!("{}", "─".repeat(80).dimmed());
    
    let keywords_per_line = 8;
    for (i, keyword) in database.keywords.iter().enumerate() {
        if i % keywords_per_line == 0 && i > 0 {
            println!();
        }
        print!("{:15}", keyword.russian.cyan());
    }
    println!("\n   {}: {}", "Всего ключевых слов".bold(), database.keywords.len());
    
    // 3. ГЛОБАЛЬНЫЕ ОБЪЕКТЫ (менеджеры)
    println!("\n{}", "🏢 ГЛОБАЛЬНЫЕ ОБЪЕКТЫ (МЕНЕДЖЕРЫ)".green().bold());
    println!("{}", "─".repeat(80).dimmed());
    
    if database.global_objects.is_empty() {
        println!("   {} Пока не извлечены из синтакс-помощника", "⚠️".yellow());
        
        // Показываем хардкод типы из platform_types_v2
        let resolver = create_platform_resolver_with_syntax_helper();
        let platform_globals = resolver.get_platform_globals();
        
        // Фильтруем только менеджеры
        let managers: Vec<_> = platform_globals.iter()
            .filter(|(name, _)| {
                name.contains("Справочники") || name.contains("Catalogs") ||
                name.contains("Документы") || name.contains("Documents") ||
                name.contains("РегистрыСведений") || name.contains("InformationRegisters") ||
                name.contains("Перечисления") || name.contains("Enums")
            })
            .collect();
            
        println!("\n   {} из hardcoded типов:", "Доступные менеджеры".italic());
        for (name, _) in managers {
            println!("      ├─ {}", name.blue());
        }
    } else {
        for (name, obj) in &database.global_objects {
            println!("   ├─ {} ({})", name.blue(), obj.object_type);
            
            if !obj.methods.is_empty() {
                println!("   │  ├─ Методы:");
                for method in obj.methods.iter().take(3) {
                    println!("   │  │  ├─ {}", method);
                }
                if obj.methods.len() > 3 {
                    println!("   │  │  └─ ...ещё {}", obj.methods.len() - 3);
                }
            }
            
            if !obj.properties.is_empty() {
                println!("   │  └─ Свойства:");
                for prop in obj.properties.iter().take(3) {
                    println!("   │     ├─ {}", prop);
                }
                if obj.properties.len() > 3 {
                    println!("   │     └─ ...ещё {}", obj.properties.len() - 3);
                }
            }
        }
    }
    
    // 4. СИСТЕМНЫЕ ПЕРЕЧИСЛЕНИЯ
    println!("\n{}", "📝 СИСТЕМНЫЕ ПЕРЕЧИСЛЕНИЯ".green().bold());
    println!("{}", "─".repeat(80).dimmed());
    
    if database.system_enums.is_empty() {
        println!("   {} Пока не извлечены из синтакс-помощника", "⚠️".yellow());
    } else {
        for (name, enum_info) in &database.system_enums {
            println!("   ├─ {}", name.magenta());
            for value in enum_info.values.iter().take(3) {
                println!("   │  ├─ {}", value.name);
            }
            if enum_info.values.len() > 3 {
                println!("   │  └─ ...ещё {} значений", enum_info.values.len() - 3);
            }
        }
    }
    
    // 5. СТАТИСТИКА
    println!("\n{}", "📊 ОБЩАЯ СТАТИСТИКА".green().bold());
    println!("{}", "─".repeat(80).dimmed());
    
    let stats = vec![
        ("Глобальных функций", database.global_functions.len()),
        ("Глобальных объектов", database.global_objects.len()),
        ("Методов объектов", database.object_methods.len()),
        ("Свойств объектов", database.object_properties.len()),
        ("Системных перечислений", database.system_enums.len()),
        ("Ключевых слов", database.keywords.len()),
        ("Операторов", database.operators.len()),
    ];
    
    for (name, count) in stats {
        let bar_length = (count as f32 / 500.0 * 50.0).min(50.0) as usize;
        let bar = "█".repeat(bar_length);
        let empty = "░".repeat(50 - bar_length);
        
        println!("   {:25} {} {} {}", 
            name, 
            bar.green(), 
            empty.dimmed(),
            count.to_string().bold()
        );
    }
    
    // 6. ПРИМЕРЫ ИСПОЛЬЗОВАНИЯ
    println!("\n{}", "💡 ПРИМЕРЫ ИСПОЛЬЗОВАНИЯ В КОДЕ".green().bold());
    println!("{}", "─".repeat(80).dimmed());
    
    println!("\n   {}:", "Глобальные функции".yellow());
    println!("   {}", "Сообщить(\"Привет мир!\");".dimmed());
    println!("   {}", "ТекущаяДата = ТекущаяДата();".dimmed());
    println!("   {}", "ТипЗнч = Тип(\"СправочникСсылка.Контрагенты\");".dimmed());
    
    println!("\n   {}:", "Ключевые слова".yellow());
    println!("   {}", "Если Условие Тогда".dimmed());
    println!("   {}", "    Для Каждого Элемент Из Массив Цикл".dimmed());
    println!("   {}", "        Прервать;".dimmed());
    println!("   {}", "    КонецЦикла;".dimmed());
    println!("   {}", "КонецЕсли;".dimmed());
    
    // 7. ДЕРЕВО ТИПОВ (если будут объекты)
    if !database.global_objects.is_empty() || !database.object_methods.is_empty() {
        println!("\n{}", "🌳 ДЕРЕВО ТИПОВ".green().bold());
        println!("{}", "─".repeat(80).dimmed());
        
        // Здесь можно построить более сложное дерево типов
        println!("   Platform");
        println!("   ├─ GlobalContext");
        println!("   │  ├─ Functions ({})", database.global_functions.len());
        println!("   │  └─ Objects ({})", database.global_objects.len());
        println!("   ├─ Managers");
        println!("   │  ├─ CatalogsManager");
        println!("   │  ├─ DocumentsManager");
        println!("   │  └─ ...");
        println!("   └─ Types");
        println!("      ├─ Primitive (String, Number, Date, Boolean)");
        println!("      ├─ Collections (Array, Structure, Map)");
        println!("      └─ Metadata (References, Objects, Records)");
    }
    
    println!("\n{}", "✅ Визуализация завершена!".green().bold());
    
    // Подсказка для дальнейшего анализа
    println!("\n{}", "💡 ПОДСКАЗКИ:".yellow().bold());
    println!("   • Для поиска конкретной функции используйте: {} | grep Функция", "cargo run --example type_hierarchy_visualizer".dimmed());
    println!("   • Для сохранения в файл: {} > types.txt", "cargo run --example type_hierarchy_visualizer".dimmed());
    println!("   • Для извлечения большего количества типов нужно расширить парсер");
    
    Ok(())
}