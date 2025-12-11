//! DEBUG test для диагностики парсинга табличных частей

#[cfg(test)]
mod parser_debug_tests {
    use bsl_backend::data::loaders::config_metadata_parser::parser::UniversalMetadataParser;
    use std::path::Path;

    #[test]
    fn test_parse_zakaznarjady_directly() {
        // Прямой парсинг XML файла
        let xml_path = Path::new("../examples/conf/conf_test/Documents/ЗаказНаряды.xml");

        assert!(
            xml_path.exists(),
            "XML файл не найден: {:?}",
            xml_path.canonicalize()
        );

        let result = UniversalMetadataParser::parse_any_object(xml_path);

        assert!(
            result.is_ok(),
            "Парсинг не удался: {:?}",
            result.err()
        );

        let metadata = result.unwrap();

        println!("===== PARSED METADATA =====");
        println!("Name: {}", metadata.name);
        println!("UUID: {}", metadata.uuid);
        println!("Object type: {:?}", metadata.object_type);
        println!("Attributes count: {}", metadata.attributes.len());
        println!("Tabular sections count: {}", metadata.tabular_sections.len());

        println!("\n===== TABULAR SECTIONS =====");
        for ts in &metadata.tabular_sections {
            println!("  ТЧ: {} (атрибутов: {})", ts.name, ts.attributes.len());
            for attr in &ts.attributes {
                println!("    - {}: {:?}", attr.name, attr.type_name);
            }
        }

        println!("\n===== ATTRIBUTES =====");
        for (i, attr) in metadata.attributes.iter().enumerate().take(5) {
            println!("  {}. {}: {:?}", i + 1, attr.name, attr.type_name);
        }

        // ASSERTIONS
        assert_eq!(
            metadata.name, "ЗаказНаряды",
            "Неверное имя документа"
        );

        assert_eq!(
            metadata.tabular_sections.len(),
            2,
            "Должно быть 2 табличные части, найдено: {}. Табличные части: {:?}",
            metadata.tabular_sections.len(),
            metadata
                .tabular_sections
                .iter()
                .map(|ts| &ts.name)
                .collect::<Vec<_>>()
        );

        // Проверка первой ТЧ "Работы"
        let raboty = metadata
            .tabular_sections
            .iter()
            .find(|ts| ts.name == "Работы")
            .expect("ТЧ 'Работы' не найдена");

        assert_eq!(
            raboty.attributes.len(),
            1,
            "ТЧ 'Работы' должна иметь 1 атрибут"
        );
        assert_eq!(
            raboty.attributes[0].name, "ВидРаботы",
            "Атрибут должен называться 'ВидРаботы'"
        );

        // Проверка второй ТЧ "Стороны"
        let storony = metadata
            .tabular_sections
            .iter()
            .find(|ts| ts.name == "Стороны")
            .expect("ТЧ 'Стороны' не найдена");

        assert_eq!(
            storony.attributes.len(),
            1,
            "ТЧ 'Стороны' должна иметь 1 атрибут"
        );
        assert_eq!(
            storony.attributes[0].name, "Сторона",
            "Атрибут должен называться 'Сторона'"
        );
    }
}
