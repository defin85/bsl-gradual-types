//! Демонстрация нечеткого поиска (Fuzzy Search)

use anyhow::Result;
use bsl_gradual_types::documentation::search::fuzzy::{FuzzyMatcher, FuzzyMatchType};

fn main() -> Result<()> {
    println!("🔍 Демонстрация нечеткого поиска (Fuzzy Search) для BSL");
    
    let mut fuzzy_matcher = FuzzyMatcher::default_for_bsl();
    
    // Создаем список типовых BSL терминов
    let bsl_terms = vec![
        "ТаблицаЗначений".to_string(),
        "СписокЗначений".to_string(), 
        "ДеревоЗначений".to_string(),
        "Справочники".to_string(),
        "СправочникМенеджер".to_string(),
        "СправочникОбъект".to_string(),
        "СправочникСсылка".to_string(),
        "Документы".to_string(),
        "ДокументМенеджер".to_string(),
        "ДокументОбъект".to_string(),
        "Перечисления".to_string(),
        "РегистрыСведений".to_string(),
        "РегистрыНакопления".to_string(),
        "ОбработкаОбъект".to_string(),
        "ОтчетОбъект".to_string(),
        "HTTPЗапрос".to_string(),
        "HTTPОтвет".to_string(),
        "XMLЧтение".to_string(),
        "XMLЗапись".to_string(),
        "JSONЧтение".to_string(),
        "JSONЗапись".to_string(),
    ];
    
    println!("📚 База терминов: {} BSL типов", bsl_terms.len());
    
    // Тест 1: Точные совпадения
    println!("\n=== Тест 1: Точные совпадения ===");
    test_search(&mut fuzzy_matcher, "ТаблицаЗначений", &bsl_terms);
    
    // Тест 2: Опечатки
    println!("\n=== Тест 2: Поиск с опечатками ===");
    test_search(&mut fuzzy_matcher, "ТаблицаЗначении", &bsl_terms); // Ошибка в окончании
    test_search(&mut fuzzy_matcher, "Справочнки", &bsl_terms);      // Пропущена буква
    test_search(&mut fuzzy_matcher, "HTTPЗопрос", &bsl_terms);      // Замена буквы
    
    // Тест 3: Частичные совпадения
    println!("\n=== Тест 3: Частичные совпадения ===");
    test_search(&mut fuzzy_matcher, "Таблица", &bsl_terms);
    test_search(&mut fuzzy_matcher, "Значений", &bsl_terms);
    test_search(&mut fuzzy_matcher, "HTTP", &bsl_terms);
    
    // Тест 4: Сложные случаи
    println!("\n=== Тест 4: Сложные случаи ===");
    test_search(&mut fuzzy_matcher, "ТабличкаЗначений", &bsl_terms); // Лишняя буква
    test_search(&mut fuzzy_matcher, "СправочникОбъект", &bsl_terms);  // Составное слово
    
    // Статистика кеша
    println!("\n=== 📊 Статистика кеша ===");
    let cache_stats = fuzzy_matcher.cache_stats();
    println!("Записей в кеше: {}", cache_stats.entries_count);
    println!("Примерный объем памяти: {} байт", cache_stats.memory_estimate_bytes);
    
    println!("\n🎉 Демонстрация fuzzy поиска завершена!");
    Ok(())
}

fn test_search(fuzzy_matcher: &mut FuzzyMatcher, query: &str, terms: &[String]) {
    println!("Поиск: '{}'", query);
    
    let matches = fuzzy_matcher.find_matches(query, terms);
    
    if matches.is_empty() {
        println!("  ❌ Ничего не найдено");
        return;
    }
    
    println!("  ✅ Найдено {} совпадений:", matches.len());
    
    for (i, fuzzy_match) in matches.iter().take(3).enumerate() {
        let match_icon = match fuzzy_match.match_type {
            FuzzyMatchType::Exact => "🎯",
            FuzzyMatchType::Prefix => "📍",
            FuzzyMatchType::Contains => "🔍",
            FuzzyMatchType::Fuzzy => "🌟",
        };
        
        println!("    {}. {} {} (схожесть: {:.2}, расстояние: {})", 
            i + 1, 
            match_icon,
            fuzzy_match.term,
            fuzzy_match.similarity,
            fuzzy_match.distance
        );
    }
}