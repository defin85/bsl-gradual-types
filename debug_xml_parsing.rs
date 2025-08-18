use bsl_gradual_types::adapters::config_parser_quick_xml::ConfigurationQuickXmlParser;

fn main() -> anyhow::Result<()> {
    println!("🔍 Отладка парсинга XML файлов конфигурации\n");
    
    // Проверим каждый файл по отдельности
    let files = [
        ("tests/fixtures/xml_full/Catalogs/Контрагенты.xml", "Catalog"),
        ("tests/fixtures/xml_full/Catalogs/Организации.xml", "Catalog"),
        ("tests/fixtures/xml_full/Documents/ЗаказНаряды.xml", "Document"),
        ("tests/fixtures/xml_full/InformationRegisters/ТестовыйРегистрСведений.xml", "Register"),
    ];
    
    for (file_path, expected_kind) in files {
        println!("📄 Анализ файла: {}", file_path);
        
        if !std::path::Path::new(file_path).exists() {
            println!("   ❌ Файл не найден!\n");
            continue;
        }
        
        // Создаём парсер для этого файла
        let parser = ConfigurationQuickXmlParser::new("tests/fixtures/xml_full");
        
        match parser.parse_metadata_xml(
            &std::path::Path::new(file_path),
            match expected_kind {
                "Catalog" => bsl_gradual_types::core::types::MetadataKind::Catalog,
                "Document" => bsl_gradual_types::core::types::MetadataKind::Document,
                "Register" => bsl_gradual_types::core::types::MetadataKind::Register,
                _ => bsl_gradual_types::core::types::MetadataKind::Catalog,
            }
        ) {
            Ok(metadata) => {
                println!("   ✅ Успешно распарсен:");
                println!("      🏷️  Имя: '{}'", metadata.name);
                println!("      📋 Тип: {:?}", metadata.kind);
                println!("      💬 Синоним: {:?}", metadata.synonym);
                println!("      📝 Атрибутов: {}", metadata.attributes.len());
                println!("      📊 Табличных частей: {}", metadata.tabular_sections.len());
                
                if !metadata.attributes.is_empty() {
                    println!("      📝 Атрибуты:");
                    for attr in &metadata.attributes {
                        println!("         - {} ({})", attr.name, attr.type_definition);
                    }
                }
                
                if !metadata.tabular_sections.is_empty() {
                    println!("      📊 Табличные части:");
                    for ts in &metadata.tabular_sections {
                        println!("         - {} (атрибутов: {})", ts.name, ts.attributes.len());
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Ошибка парсинга: {}", e);
            }
        }
        
        println!();
    }
    
    Ok(())
}