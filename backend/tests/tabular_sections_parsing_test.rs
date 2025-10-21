//! Тест парсинга табличных частей документа ЗаказНаряды

use bsl_backend::data::loaders::config_metadata_parser::parser::UniversalMetadataParser;
use std::path::Path;

#[test]
fn test_parse_document_with_tabular_sections() {
    let xml_path = Path::new("../examples/conf/conf_test/Documents/ЗаказНаряды.xml");

    // Парсим документ
    let metadata = UniversalMetadataParser::parse_any_object(xml_path)
        .expect("Не удалось распарсить документ ЗаказНаряды");

    // Проверка базовых данных
    assert_eq!(metadata.name, "ЗаказНаряды");
    println!("✅ Имя документа: {}", metadata.name);

    // Проверка табличных частей
    println!(
        "📊 Количество табличных частей: {}",
        metadata.tabular_sections.len()
    );
    assert_eq!(
        metadata.tabular_sections.len(),
        2,
        "Должно быть 2 табличные части"
    );

    // Проверка первой табличной части "Работы"
    let raboty = metadata
        .tabular_sections
        .iter()
        .find(|ts| ts.name == "Работы")
        .expect("Табличная часть 'Работы' не найдена");

    println!("📋 Табличная часть 'Работы':");
    println!("   Имя: {}", raboty.name);
    println!("   Синоним: {:?}", raboty.synonym);
    println!("   Количество атрибутов: {}", raboty.attributes.len());

    // ✅ КЛЮЧЕВАЯ ПРОВЕРКА: Атрибут "ВидРаботы" должен быть спарсен!
    for attr in &raboty.attributes {
        println!("   - Атрибут: {} ({})", attr.name, attr.type_name);
    }

    assert_eq!(
        raboty.attributes.len(),
        1,
        "Должен быть 1 атрибут в табличной части 'Работы'"
    );
    assert_eq!(
        raboty.attributes[0].name, "ВидРаботы",
        "Атрибут должен называться 'ВидРаботы'"
    );
    assert!(
        raboty.attributes[0].type_name.contains("string"),
        "Тип должен быть строковым"
    );

    // Проверка второй табличной части "Стороны"
    let storony = metadata
        .tabular_sections
        .iter()
        .find(|ts| ts.name == "Стороны")
        .expect("Табличная часть 'Стороны' не найдена");

    println!("📋 Табличная часть 'Стороны':");
    println!("   Имя: {}", storony.name);
    println!("   Количество атрибутов: {}", storony.attributes.len());

    for attr in &storony.attributes {
        println!("   - Атрибут: {} ({})", attr.name, attr.type_name);
    }

    assert_eq!(
        storony.attributes.len(),
        1,
        "Должен быть 1 атрибут в табличной части 'Стороны'"
    );
    assert_eq!(
        storony.attributes[0].name, "Сторона",
        "Атрибут должен называться 'Сторона'"
    );
}
