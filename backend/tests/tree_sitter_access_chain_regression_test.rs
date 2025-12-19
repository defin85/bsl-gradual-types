//! Регрессия: конвертация access-цепочек, где объектом является выражение.
//!
//! Пример из реальных конфигураций: `Новый Структура().Вставить(...)`.

use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use tree_sitter::Parser;

#[test]
fn converts_access_chain_with_new_expression_object() {
    let code = r#"
Процедура Тест() Экспорт
    Структура().Вставить("Ключ", 1);
КонецПроцедуры
"#;

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .expect("tree-sitter-bsl language");

    let tree = parser.parse(code, None).expect("parse");
    match TreeSitterAdapter::convert_tree(&tree, code) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("tree-sitter sexp: {}", tree.root_node().to_sexp());
            panic!("convert_tree: {:?}", e);
        }
    }
}
