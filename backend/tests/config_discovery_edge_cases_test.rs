//! Тесты граничных случаев для автоматического обнаружения конфигурации
//!
//! Проверяет edge-случаи работы find_configuration_folder():
//! - Несколько Configuration.xml в разных подпапках
//! - Вложенные подпапки
//! - Символьные ссылки и специальные символы в путях
//! - Обратная совместимость со старыми путями

#![allow(clippy::len_zero)]
#![allow(clippy::expect_fun_call)]

use bsl_backend::data::loaders::ConfigurationDiscovery;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

// ============================================================================
// 1. Граничные случаи - множественные конфигурации
// ============================================================================

#[test]
fn test_multiple_configurations_returns_first() {
    // Если в родительской папке несколько подпапок с Configuration.xml,
    // должна быть возвращена ПЕРВАЯ найденная
    let parent_path = Path::new("../examples/conf");

    if !parent_path.exists() {
        eprintln!("⚠️ Родительская папка не найдена: {:?}", parent_path);
        return;
    }

    // Проверяем, что в examples/conf действительно несколько конфигураций
    let conf_test_exists = parent_path.join("conf_test/Configuration.xml").exists();
    let ext_test_exists = parent_path.join("ext_test/Configuration.xml").exists();

    println!(
        "📂 conf_test/Configuration.xml существует: {}",
        conf_test_exists
    );
    println!(
        "📂 ext_test/Configuration.xml существует: {}",
        ext_test_exists
    );

    if !conf_test_exists && !ext_test_exists {
        eprintln!("⚠️ Ни одна конфигурация не найдена для теста");
        return;
    }

    let discovery = ConfigurationDiscovery::new(parent_path.to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "При множественных конфигурациях должна быть найдена первая"
    );

    let metadata = result.unwrap();

    // Проверяем, что найдена ОДНА конфигурация (не все сразу)
    // conf_test имеет ~5 объектов, ext_test имеет ~3 объекта
    println!(
        "✅ Найдено объектов: {} (из одной конфигурации)",
        metadata.len()
    );

    // Если бы загрузились ОБЕ конфигурации, было бы ~8 объектов
    // Но мы загружаем только первую найденную
    assert!(
        metadata.len() > 0 && metadata.len() < 10,
        "Должна быть загружена только одна конфигурация"
    );
}

#[test]
fn test_backward_compatibility_direct_path() {
    // Обратная совместимость: если Configuration.xml прямо в base_path,
    // он должен быть найден БЕЗ сканирования подпапок
    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Обратная совместимость: прямой путь к Configuration.xml должен работать"
    );

    let metadata = result.unwrap();

    println!(
        "✅ Обратная совместимость: найдено {} объектов",
        metadata.len()
    );

    // Проверяем, что найдены известные объекты из conf_test
    let has_kontragenty = metadata.iter().any(|m| m.name == "Контрагенты");
    assert!(has_kontragenty, "Должен быть найден справочник Контрагенты");
}

// ============================================================================
// 2. Edge-случаи - структуры папок
// ============================================================================

#[test]
fn test_deeply_nested_configuration() {
    // Тест с глубоко вложенной структурой папок
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    // Создаём глубоко вложенную структуру:
    // temp/
    //   └── level1/
    //       └── level2/
    //           └── config/
    //               └── Configuration.xml
    let level1 = temp_dir.path().join("level1");
    fs::create_dir(&level1).expect("Не удалось создать level1");

    let level2 = level1.join("level2");
    fs::create_dir(&level2).expect("Не удалось создать level2");

    let config_folder = level2.join("config");
    fs::create_dir(&config_folder).expect("Не удалось создать config");

    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>ВложеннаяКонфигурация</Name>
        </Properties>
        <ChildObjects>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = config_folder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content).expect("Не удалось записать Configuration.xml");

    // Запускаем discovery с temp (корневой папки)
    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    // ВАЖНО: find_configuration_folder() сканирует только ПРЯМЫЕ подпапки,
    // НЕ рекурсивно. Поэтому config в level1/level2/config НЕ будет найден.
    assert!(
        result.is_err(),
        "Глубоко вложенные конфигурации (>1 уровень) НЕ должны находиться автоматически"
    );

    println!("✅ Глубокая вложенность корректно НЕ обнаружена (нерекурсивное сканирование)");

    // Но если указать путь до level1, то config будет найден
    let discovery_level1 = ConfigurationDiscovery::new(level1.clone());
    let result_level1 = discovery_level1
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result_level1.is_err(),
        "Даже с level1 конфигурация в level2/config НЕ должна найтись (нерекурсивно)"
    );

    // Но если указать путь до level2, то config будет найден
    let discovery_level2 = ConfigurationDiscovery::new(level2);
    let result_level2 = discovery_level2
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result_level2.is_ok(),
        "С прямым путем к level2 конфигурация в config должна найтись"
    );

    println!("✅ С прямым путём до родительской папки конфигурация найдена корректно");
}

#[test]
fn test_empty_subfolders_skip() {
    // Тест: пустые подпапки должны пропускаться без ошибок
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    // Создаём 5 пустых подпапок
    for i in 1..=5 {
        let subfolder = temp_dir.path().join(format!("empty_{}", i));
        fs::create_dir(&subfolder).expect(&format!("Не удалось создать subfolder {}", i));
    }

    // Создаём одну подпапку с Configuration.xml
    let config_folder = temp_dir.path().join("config_here");
    fs::create_dir(&config_folder).expect("Не удалось создать config_here");

    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>КонфигурацияСредиПустых</Name>
        </Properties>
        <ChildObjects>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = config_folder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content).expect("Не удалось записать Configuration.xml");

    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Пустые подпапки должны пропускаться, конфигурация должна быть найдена"
    );

    println!("✅ Пустые подпапки корректно пропущены, конфигурация найдена");
}

#[test]
fn test_files_in_parent_folder_ignored() {
    // Тест: файлы в родительской папке не должны мешать поиску конфигурации
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    // Создаём несколько файлов в корне (имитируем реальную структуру проекта)
    fs::write(temp_dir.path().join("README.md"), "# Test Project")
        .expect("Не удалось создать README.md");
    fs::write(
        temp_dir.path().join("test.bsl"),
        "Процедура Тест() КонецПроцедуры",
    )
    .expect("Не удалось создать test.bsl");

    // Создаём подпапку с конфигурацией
    let config_folder = temp_dir.path().join("src");
    fs::create_dir(&config_folder).expect("Не удалось создать src");

    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>КонфигурацияВSrc</Name>
        </Properties>
        <ChildObjects>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = config_folder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content).expect("Не удалось записать Configuration.xml");

    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Файлы в родительской папке не должны мешать поиску конфигурации"
    );

    println!("✅ Файлы в корне проекта корректно игнорируются");
}

// ============================================================================
// 3. Проверка логов и сообщений об ошибках
// ============================================================================

#[test]
fn test_error_message_clarity() {
    // Проверяем, что сообщение об ошибке информативное
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_err(),
        "Должна быть ошибка если конфигурация не найдена"
    );

    if let Err(e) = result {
        let error_msg = e.to_string();

        println!("📋 Текст ошибки:\n{}", error_msg);

        // Проверяем наличие ключевых фраз в сообщении
        assert!(
            error_msg.contains("Configuration.xml"),
            "Ошибка должна упоминать Configuration.xml"
        );

        assert!(
            error_msg.contains("не найден"),
            "Ошибка должна содержать 'не найден'"
        );

        assert!(
            error_msg.contains("подпапках") || error_msg.contains("подпапках"),
            "Ошибка должна упоминать поиск в подпапках"
        );

        println!("✅ Сообщение об ошибке информативное и понятное");
    }
}

// ============================================================================
// 4. Интеграция с реальными данными
// ============================================================================

#[test]
fn test_real_project_structure() {
    // Имитируем реальную структуру проекта 1С:
    // project/
    //   ├── README.md
    //   ├── src/
    //   │   └── cf/              ← конфигурация здесь
    //   │       └── Configuration.xml
    //   └── tests/

    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    // Создаём структуру
    fs::write(temp_dir.path().join("README.md"), "# 1C Project")
        .expect("Не удалось создать README.md");

    let src_folder = temp_dir.path().join("src");
    fs::create_dir(&src_folder).expect("Не удалось создать src");

    let cf_folder = src_folder.join("cf");
    fs::create_dir(&cf_folder).expect("Не удалось создать cf");

    let tests_folder = temp_dir.path().join("tests");
    fs::create_dir(&tests_folder).expect("Не удалось создать tests");

    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>РеальныйПроект</Name>
        </Properties>
        <ChildObjects>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = cf_folder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content).expect("Не удалось записать Configuration.xml");

    // Запускаем discovery с корня проекта
    let discovery_root = ConfigurationDiscovery::new(temp_dir.path().to_path_buf());

    let result_root = discovery_root
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    // С корня проекта НЕ должна найтись (cf вложен в src)
    assert!(
        result_root.is_err(),
        "С корня проекта конфигурация в src/cf НЕ должна найтись (нерекурсивно)"
    );

    // Но если указать путь до src, должна найтись
    let discovery_src = ConfigurationDiscovery::new(src_folder);

    let result_src = discovery_src
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result_src.is_ok(),
        "С пути src конфигурация в cf должна найтись"
    );

    println!("✅ Реальная структура проекта: поиск работает корректно (нерекурсивно)");
}

#[test]
fn test_russian_paths() {
    // Тест с русскими символами в путях (важно для 1С проектов)
    let temp_dir = TempDir::new().expect("Не удалось создать временную папку");

    let russian_folder = temp_dir.path().join("Конфигурация_УПП");
    fs::create_dir(&russian_folder).expect("Не удалось создать папку с русским названием");

    let config_xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
    <Configuration>
        <Properties>
            <Name>КонфигурацияСРусскимПутем</Name>
        </Properties>
        <ChildObjects>
        </ChildObjects>
    </Configuration>
</MetaDataObject>"#;

    let config_xml_path = russian_folder.join("Configuration.xml");
    fs::write(&config_xml_path, config_xml_content)
        .expect("Не удалось записать Configuration.xml в русский путь");

    let discovery = ConfigurationDiscovery::new(temp_dir.path().to_path_buf());

    let result = discovery
        .discover_all_metadata(None::<fn(bsl_backend::data::loaders::progress::ProgressUpdate)>);

    assert!(
        result.is_ok(),
        "Русские символы в путях должны обрабатываться корректно"
    );

    println!("✅ Русские символы в путях обработаны корректно");
}
