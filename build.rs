use std::path::PathBuf;

fn main() {
    // Путь к грамматике tree-sitter-bsl
    let tree_sitter_bsl_path = PathBuf::from("../tree-sitter-bsl");
    
    // Проверяем, существует ли директория tree-sitter-bsl
    if !tree_sitter_bsl_path.exists() {
        println!("cargo:warning=tree-sitter-bsl not found at {:?}", tree_sitter_bsl_path);
        println!("cargo:warning=Please clone https://github.com/alkoleft/tree-sitter-bsl");
        return;
    }
    
    let src_dir = tree_sitter_bsl_path.join("src");
    
    // Компилируем парсер tree-sitter-bsl
    cc::Build::new()
        .include(&src_dir)
        .file(src_dir.join("parser.c"))
        .compile("tree-sitter-bsl");
    
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../tree-sitter-bsl/src/parser.c");
}