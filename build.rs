//! Build script для bsl-gradual-types
//! Создает заглушку для tree_sitter_bsl пока нет реальной библиотеки

use std::env;
use std::path::PathBuf;

fn main() {
    // Временная заглушка для tree_sitter_bsl
    // TODO: Заменить на реальную tree-sitter-bsl библиотеку
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Создаем C файл с заглушкой
    let c_stub = r#"
#include <stdint.h>
#include <stdlib.h>

// Заглушка для tree_sitter_bsl
// Создает минимальную структуру TSLanguage с abi_version 0
typedef struct TSLanguage {
    uint32_t version;
    uint32_t symbol_count;
    const char * const *symbol_names;
    // Остальные поля не используются в заглушке
    const void *parse_table;
    const void *parse_actions;
    const void *lex_modes;
    const void *keyword_lex_modes;
    const void *keyword_capture_token;
    const void *external_token_count;
    const void *external_scanner;
    const void *field_count;
    const void *field_map_slices;
    const void *field_map_entries;
    const void *field_names;
    const void *max_alias_sequence_length;
    const void *alias_map;
    const void *alias_sequences;
    const void *small_parse_table;
    const void *small_parse_table_map;
    const void *public_symbol_map;
} TSLanguage;

// Статическая заглушка языка
static const TSLanguage bsl_language_stub = {
    .version = 0,  // Версия 0 означает заглушку
    .symbol_count = 0,
    .symbol_names = NULL,
    .parse_table = NULL,
    .parse_actions = NULL,
    .lex_modes = NULL,
    .keyword_lex_modes = NULL,
    .keyword_capture_token = NULL,
    .external_token_count = NULL,
    .external_scanner = NULL,
    .field_count = NULL,
    .field_map_slices = NULL,
    .field_map_entries = NULL,
    .field_names = NULL,
    .max_alias_sequence_length = NULL,
    .alias_map = NULL,
    .alias_sequences = NULL,
    .small_parse_table = NULL,
    .small_parse_table_map = NULL,
    .public_symbol_map = NULL,
};

const TSLanguage *tree_sitter_bsl(void) {
    return &bsl_language_stub;
}
"#;
    
    let c_file = out_dir.join("tree_sitter_bsl_stub.c");
    std::fs::write(&c_file, c_stub).unwrap();
    
    // Компилируем C заглушку
    cc::Build::new()
        .file(&c_file)
        .compile("tree_sitter_bsl_stub");
    
    println!("cargo:rustc-link-lib=tree_sitter_bsl_stub");
    println!("cargo:rerun-if-changed=build.rs");
}