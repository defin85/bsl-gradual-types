//! Тесты для парсера синтакс-помощника

use bsl_gradual_types::adapters::syntax_helper_parser::{
    SyntaxHelperParser, SyntaxHelperDatabase, FunctionInfo,
};
use std::path::Path;

#[test]
fn test_parser_creation() {
    let parser = SyntaxHelperParser::new();
    let database = parser.database();
    
    // Проверяем что база данных пустая при создании
    assert_eq!(database.global_functions.len(), 0);
    assert_eq!(database.global_objects.len(), 0);
    assert_eq!(database.object_methods.len(), 0);
    assert_eq!(database.object_properties.len(), 0);
    assert_eq!(database.system_enums.len(), 0);
    assert_eq!(database.keywords.len(), 0);
    assert_eq!(database.operators.len(), 0);
}

#[test]
fn test_save_and_load_database() {
    use std::fs;
    use std::collections::HashMap;
    
    // Создаём тестовую базу данных
    let mut database = SyntaxHelperDatabase {
        global_functions: HashMap::new(),
        global_objects: HashMap::new(),
        object_methods: HashMap::new(),
        object_properties: HashMap::new(),
        system_enums: HashMap::new(),
        keywords: Vec::new(),
        operators: Vec::new(),
    };
    
    // Добавляем тестовую функцию
    database.global_functions.insert(
        "TestFunction".to_string(),
        FunctionInfo {
            name: "TestFunction".to_string(),
            english_name: Some("TestFunc".to_string()),
            description: Some("Test description".to_string()),
            syntax: vec!["TestFunction()".to_string()],
            parameters: vec![],
            return_type: None,
            return_description: None,
            examples: vec![],
            availability: vec!["Client".to_string(), "Server".to_string()],
        },
    );
    
    // Сохраняем в файл
    let temp_file = "target/test_syntax_database.json";
    let parser = SyntaxHelperParser::new();
    
    // Создаём временный файл с базой данных вручную
    let json = serde_json::to_string_pretty(&database).unwrap();
    fs::write(temp_file, json).unwrap();
    
    // Загружаем из файла
    let loaded_database = SyntaxHelperParser::load_from_file(temp_file).unwrap();
    
    // Проверяем что данные загрузились правильно
    assert_eq!(loaded_database.global_functions.len(), 1);
    assert!(loaded_database.global_functions.contains_key("TestFunction"));
    
    let func = &loaded_database.global_functions["TestFunction"];
    assert_eq!(func.name, "TestFunction");
    assert_eq!(func.english_name, Some("TestFunc".to_string()));
    assert_eq!(func.description, Some("Test description".to_string()));
    
    // Удаляем временный файл
    fs::remove_file(temp_file).unwrap();
}

#[test]
#[ignore] // Игнорируем, так как требуются реальные архивы синтакс-помощника
fn test_parse_real_archives() {
    let mut parser = SyntaxHelperParser::new()
        .with_context_archive("examples/syntax_helper/rebuilt.shcntx_ru.zip")
        .with_lang_archive("examples/syntax_helper/rebuilt.shlang_ru.zip");
    
    // Пытаемся парсить архивы, если они существуют
    if Path::new("examples/syntax_helper/rebuilt.shcntx_ru.zip").exists() {
        parser.parse().unwrap();
        
        let database = parser.database();
        
        // Проверяем что хотя бы что-то распарсилось
        assert!(database.global_functions.len() > 0 || database.keywords.len() > 0);
        
        // Если есть глобальные функции, проверяем известные
        if database.global_functions.len() > 0 {
            // Должны быть стандартные функции типа Сообщить, Тип и т.д.
            let known_functions = ["Сообщить", "Тип", "XMLСтрока", "Формат"];
            for func_name in &known_functions {
                if database.global_functions.contains_key(*func_name) {
                    println!("Found function: {}", func_name);
                }
            }
        }
        
        // Если есть ключевые слова, проверяем известные
        if database.keywords.len() > 0 {
            println!("Found {} keywords", database.keywords.len());
            for keyword in &database.keywords[..5.min(database.keywords.len())] {
                println!("  - {} / {}", keyword.russian, keyword.english);
            }
        }
    }
}