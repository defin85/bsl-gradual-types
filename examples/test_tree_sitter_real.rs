//! Реальный тест загрузки tree-sitter-bsl

use tree_sitter::{Parser, Language};

// Внешняя функция для получения языка BSL
extern "C" {
    fn tree_sitter_bsl() -> Language;
}

fn main() {
    println!("Testing real tree-sitter-bsl loading...");
    
    // Пытаемся создать парсер и загрузить язык
    let mut parser = Parser::new();
    
    // Загружаем язык BSL
    let language = unsafe { tree_sitter_bsl() };
    
    println!("Language loaded, version: {:?}", unsafe {
        tree_sitter::ffi::ts_language_version(language.into())
    });
    
    // Пытаемся установить язык
    match parser.set_language(&language) {
        Ok(_) => {
            println!("✓ Language set successfully!");
            
            // Пробуем парсить простой код
            let code = "А = 1;";
            match parser.parse(code, None) {
                Some(tree) => {
                    println!("✓ Code parsed successfully!");
                    println!("Root node: {:?}", tree.root_node().kind());
                    println!("Child count: {}", tree.root_node().child_count());
                },
                None => println!("✗ Failed to parse code"),
            }
        },
        Err(e) => {
            println!("✗ Failed to set language: {:?}", e);
            
            // Выводим больше информации о версиях
            println!("\nVersion mismatch details:");
            println!("This usually means the grammar was compiled with a different tree-sitter version");
            println!("Current tree-sitter lib expects versions 13-14");
            println!("Grammar was compiled with version 15+");
            println!("\nPossible solutions:");
            println!("1. Downgrade tree-sitter-cli in tree-sitter-bsl to match our library");
            println!("2. Upgrade our tree-sitter library to match the grammar");
            println!("3. Use dynamic loading with tree-sitter-loader");
        }
    }
}