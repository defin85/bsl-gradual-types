use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;

fn main() {
    println!("🔍 Тестируем парсинг типа 'Массив' через SyntaxHelperParser\n");

    let mut parser = SyntaxHelperParser::new();

    println!("📂 Парсим синтаксис-помощник из examples/syntax_helper...");
    match parser.parse_syntax_helper("examples/syntax_helper") {
        Ok(_) => {
            let stats = parser.get_stats();
            println!("✅ Парсинг завершен! Всего файлов: {}\n", stats.total_files);
        }
        Err(e) => {
            println!("❌ Ошибка парсинга: {}", e);
            return;
        }
    }

    // Теперь ищем тип "Массив"
    println!("🔍 Ищем тип 'Массив'...\n");
    match parser.find_type("Массив") {
        Some(type_info) => {
            println!("✅ Тип найден!\n");
            println!("📦 TypeInfo:");
            println!("  Русское имя: {}", type_info.identity.russian_name);
            println!("  Английское имя: {}", type_info.identity.english_name);
            println!("  Категория: {}", type_info.identity.category_path);
            println!("  Фасеты: {:?}", type_info.metadata.available_facets);

            println!("\n🔧 Структура:");
            println!("  Методов: {}", type_info.structure.methods.len());
            if !type_info.structure.methods.is_empty() {
                println!("  Первые 9 методов:");
                for method in type_info.structure.methods.iter().take(9) {
                    println!("    - {:?}", method);
                }
            } else {
                println!("  ❌ ПРОБЛЕМА: методы не извлечены!");
            }

            println!("  Свойств: {}", type_info.structure.properties.len());
            println!("  Конструкторов: {}", type_info.structure.constructors.len());

            println!("\n📋 Коллекция:");
            println!("  collection_element: {:?}", type_info.structure.collection_element);
            println!("  iterable: {}", type_info.structure.iterable);
            println!("  indexable: {}", type_info.structure.indexable);

            // Финальный вердикт
            let methods_ok = type_info.structure.methods.len() > 0;
            let collection_ok = type_info.structure.collection_element.is_some();
            let iterable_ok = type_info.structure.iterable;
            let indexable_ok = type_info.structure.indexable;

            println!("\n📊 ИТОГОВАЯ ПРОВЕРКА FACT-BASED ПАРСИНГА:");
            println!("  ✅ Методы извлечены: {}", if methods_ok { "ДА" } else { "НЕТ" });
            println!("  ✅ Элемент коллекции: {}", if collection_ok { "ДА" } else { "НЕТ" });
            println!("  ✅ Итерируемость: {}", if iterable_ok { "ДА" } else { "НЕТ" });
            println!("  ✅ Индексируемость: {}", if indexable_ok { "ДА" } else { "НЕТ" });

            if methods_ok && collection_ok && iterable_ok && indexable_ok {
                println!("\n🎉 FACT-BASED ПАРСИНГ РАБОТАЕТ ПОЛНОСТЬЮ КОРРЕКТНО!");
            } else {
                println!("\n⚠️ FACT-BASED ПАРСИНГ РАБОТАЕТ ЧАСТИЧНО");
            }
        }
        None => {
            println!("❌ Тип 'Массив' НЕ найден в базе!");
        }
    }
}
