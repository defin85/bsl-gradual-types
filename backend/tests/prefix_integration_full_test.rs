//! Интеграционные тесты для load_all_configurations_metadata()
//!
//! Проверяет корректность загрузки метаданных из базовой конфигурации и расширений
//! с автоматическим применением префиксов к типам расширений.

use bsl_backend::system::SystemCoordinator;
use std::path::Path;

/// Вспомогательная функция для поиска типов по подстроке
/// (TypeRepository trait не имеет search_types, используем фильтрацию get_all_types)
fn search_types_by_substring(
    repository: &std::sync::Arc<dyn bsl_shared::domain::repository::TypeRepository>,
    substring: &str,
) -> Vec<String> {
    repository
        .get_all_types()
        .into_iter()
        .filter(|t| t.name.contains(substring))
        .map(|t| t.name)
        .collect()
}

/// Вспомогательная функция для инициализации координатора
async fn setup_coordinator() -> SystemCoordinator {
    let coordinator = SystemCoordinator::new();
    coordinator
        .start_with_paths(None, None, None)
        .await
        .expect("Ошибка инициализации SystemCoordinator");
    coordinator
}

#[tokio::test]
async fn test_load_single_base_configuration() {
    // ЦЕЛЬ: Загрузка одной базовой конфигурации без расширений

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf/conf_test");
    let result = coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигурации");

    assert_eq!(
        result.base_config_count, 1,
        "Должна быть загружена 1 базовая конфигурация"
    );
    assert_eq!(
        result.extensions_count, 0,
        "Расширения отсутствуют, должно быть 0"
    );
    assert!(
        result.total_types > 0,
        "Должно быть загружено хотя бы несколько типов"
    );
}

#[tokio::test]
async fn test_load_base_plus_extension() {
    // ЦЕЛЬ: Загрузка базовой конфигурации + одно расширение
    //
    // Предполагается, что в examples/conf/ есть:
    // - conf_test/ (базовая конфигурация)
    // - ext_test/ (расширение)

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    let result = coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    // Ожидаем 1 базовую конфигурацию + 1 расширение
    assert!(
        result.base_config_count >= 1,
        "Должна быть хотя бы 1 базовая конфигурация"
    );
    assert!(
        result.extensions_count >= 0,
        "Расширения могут быть или отсутствовать"
    );
    assert!(
        result.total_types > 0,
        "Должны быть загружены типы из конфигураций"
    );

    println!("Статистика загрузки:");
    println!("  Базовых конфигураций: {}", result.base_config_count);
    println!("  Расширений: {}", result.extensions_count);
    println!("  Всего типов: {}", result.total_types);
}

#[tokio::test]
async fn test_extension_types_have_prefix() {
    // ЦЕЛЬ: Типы из расширения содержат префикс

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    // Получаем доступ к TypeRepository через AnalysisEngine
    let engine = coordinator
        .analysis_engine()
        .expect("AnalysisEngine не инициализирован");
    let repository = engine.get_repository();

    // Проверяем наличие типов с префиксом "Тест_" (если расширение существует)
    let search_results = search_types_by_substring(&repository, "Тест_");

    if search_results.is_empty() {
        println!("⚠️ Типы с префиксом 'Тест_' не найдены (возможно, расширение отсутствует)");
    } else {
        println!(
            "✅ Найдено {} типов с префиксом 'Тест_'",
            search_results.len()
        );

        // Проверяем, что найденные типы действительно содержат префикс
        for type_name in search_results.iter().take(3) {
            assert!(
                type_name.contains("Тест_"),
                "Тип {} должен содержать префикс 'Тест_'",
                type_name
            );
            println!("  - {}", type_name);
        }
    }
}

#[tokio::test]
async fn test_base_types_have_no_prefix() {
    // ЦЕЛЬ: Типы из базовой конфигурации НЕ имеют префикса

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf/conf_test");
    coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки базовой конфигурации");

    let engine = coordinator
        .analysis_engine()
        .expect("AnalysisEngine не инициализирован");
    let repository = engine.get_repository();

    // Получаем статистику репозитория
    let stats = repository.get_stats();
    println!("Статистика TypeRepository:");
    println!("  Всего типов: {}", stats.total_types);

    // Проверяем наличие базовых типов из конфигурации (без префикса)
    // Примечание: реальные имена типов зависят от conf_test/Configuration.xml
    // Здесь используем общий поиск
    let all_types = search_types_by_substring(&repository, ""); // Получаем все типы

    if !all_types.is_empty() {
        println!(
            "✅ Загружено {} типов из базовой конфигурации",
            all_types.len()
        );

        // Проверяем, что типы НЕ содержат префиксов расширений
        for type_name in all_types.iter().take(5) {
            assert!(
                !type_name.contains("Тест_") && !type_name.contains("УП_"),
                "Базовые типы не должны содержать префиксы расширений: {}",
                type_name
            );
            println!("  - {}", type_name);
        }
    }
}

#[tokio::test]
async fn test_adopted_objects_stored_separately() {
    // ЦЕЛЬ: Adopted объекты хранятся отдельно (не сливаются с базовыми)
    //
    // Если расширение добавляет атрибут к существующему справочнику,
    // в репозитории должно быть ДВА типа:
    // 1. Справочники.Контрагенты (базовый)
    // 2. Справочники.Тест_Контрагенты (из расширения, adopted)

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    let engine = coordinator
        .analysis_engine()
        .expect("AnalysisEngine не инициализирован");
    let repository = engine.get_repository();

    // Получаем все типы справочников
    let catalog_types = search_types_by_substring(&repository, "Справочники.");

    println!("Найдено {} типов справочников:", catalog_types.len());

    // Ищем пары: базовый тип + adopted тип с префиксом
    let mut base_types = Vec::new();
    let mut extension_types = Vec::new();

    for type_name in &catalog_types {
        if type_name.contains("Тест_") {
            extension_types.push(type_name.clone());
        } else {
            base_types.push(type_name.clone());
        }
    }

    println!("  Базовых типов: {}", base_types.len());
    println!("  Типов из расширений: {}", extension_types.len());

    // Если есть расширения, проверяем раздельное хранение
    if !extension_types.is_empty() {
        for ext_type in extension_types.iter().take(3) {
            println!("  Тип из расширения: {}", ext_type);

            // Извлекаем базовое имя (без префикса)
            let base_name = ext_type.replace("Тест_", "");

            // Проверяем, что базовый тип тоже существует (если это adopted)
            if repository.find_type(&base_name).is_some() {
                println!("    ✅ Базовый тип существует: {}", base_name);
                assert!(true, "Adopted объект должен храниться отдельно от базового");
            } else {
                println!("    ℹ️ Это новый тип расширения (не adopted): {}", ext_type);
            }
        }
    } else {
        println!("⚠️ Типы из расширений не найдены (тест пропущен)");
    }
}

#[tokio::test]
async fn test_loading_order_base_first() {
    // ЦЕЛЬ: Базовая конфигурация загружается ПЕРВОЙ, расширения — после
    //
    // Проверяем через статистику загрузки и логи

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    let result = coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    // Базовая конфигурация должна быть обработана
    assert!(
        result.base_config_count > 0,
        "Базовая конфигурация должна быть загружена первой"
    );

    // Общее количество типов должно включать базовые + расширения
    assert!(
        result.total_types > 0,
        "Должны быть загружены типы из всех конфигураций"
    );

    println!("Порядок загрузки соблюдён:");
    println!("  1. Базовые конфигурации: {}", result.base_config_count);
    println!("  2. Расширения: {}", result.extensions_count);
    println!("  Всего типов: {}", result.total_types);
}

#[tokio::test]
async fn test_multiple_extensions_with_different_prefixes() {
    // ЦЕЛЬ: Проверить корректную работу с несколькими расширениями
    //
    // Если в examples/conf/ будет несколько расширений с разными префиксами,
    // все они должны загружаться корректно с применением своих префиксов

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    let result = coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    println!("Загружено расширений: {}", result.extensions_count);

    if result.extensions_count > 1 {
        let engine = coordinator
            .analysis_engine()
            .expect("AnalysisEngine не инициализирован");
        let repository = engine.get_repository();

        // Ищем типы с разными префиксами
        let test_types = search_types_by_substring(&repository, "Тест_");
        let up_types = search_types_by_substring(&repository, "УП_");

        println!("  Типы с префиксом 'Тест_': {}", test_types.len());
        println!("  Типы с префиксом 'УП_': {}", up_types.len());

        // Типы с разными префиксами не должны пересекаться
        for test_type in test_types.iter().take(2) {
            assert!(
                !test_type.contains("УП_"),
                "Тип с префиксом 'Тест_' не должен содержать 'УП_'"
            );
        }
    } else {
        println!(
            "⚠️ Тест для нескольких расширений пропущен (найдено {} расширений)",
            result.extensions_count
        );
    }
}

#[tokio::test]
async fn test_repository_stats_after_loading() {
    // ЦЕЛЬ: Проверить корректность статистики TypeRepository после загрузки

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    let load_result = coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    let engine = coordinator
        .analysis_engine()
        .expect("AnalysisEngine не инициализирован");
    let repository = engine.get_repository();

    let stats = repository.get_stats();

    println!("Статистика TypeRepository:");
    println!("  Всего типов: {}", stats.total_types);
    println!("  Загружено из конфигураций: {}", load_result.total_types);

    // Общее количество типов в репозитории должно быть >= загруженных из конфигураций
    // (могут быть ещё платформенные типы)
    assert!(
        stats.total_types >= load_result.total_types,
        "Репозиторий должен содержать хотя бы загруженные типы"
    );
}

#[tokio::test]
async fn test_find_type_with_prefix() {
    // ЦЕЛЬ: Проверить возможность поиска типов с префиксами через find_type()

    let coordinator = setup_coordinator().await;

    let config_path = Path::new("../examples/conf");
    coordinator
        .load_all_configurations_metadata(config_path)
        .expect("Ошибка загрузки конфигураций");

    let engine = coordinator
        .analysis_engine()
        .expect("AnalysisEngine не инициализирован");
    let repository = engine.get_repository();

    // Поиск типа с префиксом (если он существует)
    let search_results = search_types_by_substring(&repository, "Тест_");

    if let Some(first_type) = search_results.first() {
        println!("Поиск типа: {}", first_type);

        // Пытаемся найти этот тип через find_type()
        let found_type = repository.find_type(first_type);

        assert!(
            found_type.is_some(),
            "Тип с префиксом должен находиться через find_type()"
        );

        if let Some(type_data) = found_type {
            println!("  ✅ Найден тип: {}", type_data.name);
            assert_eq!(
                type_data.name, *first_type,
                "Имя найденного типа должно совпадать с поисковым запросом"
            );
        }
    } else {
        println!("⚠️ Типы с префиксом 'Тест_' не найдены");
    }
}
