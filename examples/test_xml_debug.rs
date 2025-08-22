//! Отладка XML парсинга

use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    println!("🔍 Отладка XML парсинга по файлам\n");

    // Проверим каждый файл отдельно
    let files = [
        "tests/fixtures/xml_full/Catalogs/Контрагенты.xml",
        "tests/fixtures/xml_full/Catalogs/Организации.xml",
        "tests/fixtures/xml_full/Documents/ЗаказНаряды.xml",
        "tests/fixtures/xml_full/InformationRegisters/ТестовыйРегистрСведений.xml",
    ];

    for file_path in files {
        println!("📄 Файл: {}", file_path);

        if Path::new(file_path).exists() {
            println!("✅ Файл существует");

            // Читаем первые строки для проверки
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    println!("📝 Размер файла: {} байт", content.len());

                    // Ищем тег Name в Properties
                    if let Some(name_start) = content.find("<Name>") {
                        if let Some(name_end) = content[name_start..].find("</Name>") {
                            let name_content = &content[name_start + 6..name_start + name_end];
                            println!("🏷️ Найденное имя: '{}'", name_content);
                        }
                    } else {
                        println!("❌ Тег <Name> не найден");
                    }

                    // Проверим есть ли атрибуты
                    let attribute_count = content.matches("<Attribute>").count();
                    println!("📝 Тегов <Attribute>: {}", attribute_count);

                    // Проверим табличные части
                    let ts_count = content.matches("<TabularSection>").count();
                    println!("📊 Тегов <TabularSection>: {}", ts_count);
                }
                Err(e) => println!("❌ Ошибка чтения файла: {}", e),
            }
        } else {
            println!("❌ Файл не существует");
        }

        println!();
    }

    println!("🔍 Проверка папок:");
    let folders = ["Catalogs", "Documents", "InformationRegisters", "Enums"];

    for folder in folders {
        let folder_path = format!("tests/fixtures/xml_full/{}", folder);
        println!("📁 {}", folder_path);

        if Path::new(&folder_path).exists() {
            match std::fs::read_dir(&folder_path) {
                Ok(entries) => {
                    let xml_files: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map_or(false, |ext| ext == "xml"))
                        .collect();

                    println!("   ✅ XML файлов: {}", xml_files.len());
                    for entry in xml_files {
                        println!("      - {}", entry.file_name().to_string_lossy());
                    }
                }
                Err(e) => println!("   ❌ Ошибка чтения папки: {}", e),
            }
        } else {
            println!("   ❌ Папка не существует");
        }
    }

    Ok(())
}
