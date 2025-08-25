//! Build script для bsl-gradual-types
//! Создает заглушку для tree_sitter_bsl пока нет реальной библиотеки

fn main() {
    // Проверяем наличие реальной tree-sitter-bsl библиотеки
    let tree_sitter_bsl_path = "../tree-sitter-bsl";
    let parser_c_path = format!("{}/src/parser.c", tree_sitter_bsl_path);

    if std::path::Path::new(&parser_c_path).exists() {
        println!(
            "cargo:warning=🔍 НАЙДЕНА реальная tree-sitter-bsl: {}",
            tree_sitter_bsl_path
        );

        // Компилируем РЕАЛЬНЫЙ парсер с правильным именем
        cc::Build::new()
            .std("c11")
            .include(format!("{}/src", tree_sitter_bsl_path))
            .file(&parser_c_path)
            .compile("tree_sitter_bsl"); // ← ИСПРАВЛЕНО ИМЯ (с подчёркиваниями)

        println!("cargo:warning=✅ РЕАЛЬНАЯ tree-sitter-bsl скомпилирована как tree_sitter_bsl");
        println!("cargo:rustc-link-lib=tree_sitter_bsl"); // ← ИСПРАВЛЕНО ИМЯ
        return;
    }

    // Fallback на заглушку если библиотека не найдена
    println!(
        "cargo:warning=⚠️ tree-sitter-bsl НЕ НАЙДЕНА в {}, используем заглушку",
        tree_sitter_bsl_path
    );

    // НЕ создаём заглушку - используем fallback в коде
    println!("cargo:warning=❌ Заглушка НЕ создана - tree-sitter функции недоступны");
    println!("cargo:warning=📋 Код должен использовать cfg условную компиляцию");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../tree-sitter-bsl/src/parser.c");
}
