//! Отладочный тест для проверки парсинга типов атрибутов

use quick_xml::events::Event;
use quick_xml::Reader;

#[test]
fn test_debug_type_parsing() {
    let xml = r#"
        <Type>
            <v8:Type>xs:string</v8:Type>
            <v8:StringQualifiers>
                <v8:Length>10</v8:Length>
            </v8:StringQualifiers>
        </Type>
    "#;

    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_type = false;
    let mut current_element = String::new();
    let mut type_values = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                println!("📦 Start tag: '{}'", tag_name);

                if tag_name == "Type" {
                    in_type = true;
                    println!("  → Вход в <Type>");
                } else {
                    current_element = tag_name.clone();
                    println!("  → current_element = '{}'", current_element);
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap().trim().to_string();
                if !text.is_empty() {
                    println!("📝 Text: '{}' (в элементе '{}')", text, current_element);

                    if in_type {
                        type_values.push(text);
                        println!("  ✅ Добавлен тип: {:?}", type_values);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                println!("📦 End tag: '{}'", tag_name);

                if tag_name == "Type" {
                    in_type = false;
                    println!("  → Выход из <Type>");
                }

                current_element.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Ошибка парсинга: {:?}", e),
            _ => {}
        }
        buf.clear();
    }

    println!("\n📊 Итого извлечено типов: {:?}", type_values);
    assert!(!type_values.is_empty(), "Должен быть извлечён хотя бы один тип");
    assert_eq!(type_values[0], "xs:string", "Первый тип должен быть xs:string");
}
