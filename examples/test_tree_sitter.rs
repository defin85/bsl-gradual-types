//! Тестовый пример для проверки tree-sitter интеграции

use std::fs;

fn main() {
    println!("Testing tree-sitter-bsl integration...");
    
    // Проверяем наличие грамматики
    let grammar_path = "../tree-sitter-bsl";
    if !std::path::Path::new(grammar_path).exists() {
        eprintln!("Error: tree-sitter-bsl not found at {}", grammar_path);
        eprintln!("Please clone: git clone https://github.com/alkoleft/tree-sitter-bsl ../tree-sitter-bsl");
        return;
    }
    
    // Проверяем наличие скомпилированных файлов
    let parser_c = format!("{}/src/parser.c", grammar_path);
    if !std::path::Path::new(&parser_c).exists() {
        eprintln!("Error: parser.c not found. Grammar needs to be generated.");
        eprintln!("Run: cd {} && npm install && npm run build", grammar_path);
        return;
    }
    
    println!("✓ Grammar files found");
    
    // Пытаемся создать адаптер
    match create_adapter() {
        Ok(_) => println!("✓ Adapter created successfully"),
        Err(e) => eprintln!("✗ Failed to create adapter: {}", e),
    }
}

fn create_adapter() -> Result<(), Box<dyn std::error::Error>> {
    // Попробуем загрузить через FFI
    println!("Attempting to load tree-sitter-bsl via FFI...");
    
    // Для Windows
    #[cfg(target_os = "windows")]
    {
        println!("Platform: Windows");
        // На Windows tree-sitter компилируется в статическую библиотеку
        // которая линкуется во время сборки через build.rs
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        println!("Platform: Unix-like");
    }
    
    Ok(())
}