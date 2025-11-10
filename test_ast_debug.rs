use bsl_shared::parser::parse_bsl;

fn main() {
    let code = r#"
Функция Тест()
    ТаблицаТип = Новый ТаблицаЗначений;
    Кол = ТаблицаТип.Количество();
КонецФункции
"#;

    match parse_bsl(code) {
        Ok(ast) => {
            println!("AST: {:#?}", ast);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}
