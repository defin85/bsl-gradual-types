//! Тесты производительности для detect_configuration_type()
//!
//! Проверяет, что парсинг останавливается после </Properties>
//! и выполняется быстро (< 10ms).

use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use std::path::PathBuf;
use std::time::Instant;

/// Тест производительности: detect_configuration_type должен быть быстрым
///
/// Парсинг должен останавливаться после </Properties> и не читать весь файл.
/// Ожидаемое время: < 10ms для каждого Configuration.xml
#[test]
fn test_detect_configuration_type_performance() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let ext_path = PathBuf::from("../examples/conf/ext_test");

    let conf_discovery = ConfigurationDiscovery::new(conf_path, false);
    let ext_discovery = ConfigurationDiscovery::new(ext_path, false);

    // Прогрев (чтобы учесть I/O кеширование ОС)
    let _ = conf_discovery.discover_all_configurations();
    let _ = ext_discovery.discover_all_configurations();

    // Act & Assert - базовая конфигурация
    let start = Instant::now();
    let conf_result = conf_discovery.discover_all_configurations();
    let conf_duration = start.elapsed();

    assert!(conf_result.is_ok(), "Конфигурация должна быть найдена");
    println!("⏱️ Базовая конфигурация: {:?}", conf_duration);

    assert!(
        conf_duration.as_millis() < 10,
        "Парсинг базовой конфигурации должен быть < 10ms, было: {:?}",
        conf_duration
    );

    // Act & Assert - расширение
    let start = Instant::now();
    let ext_result = ext_discovery.discover_all_configurations();
    let ext_duration = start.elapsed();

    assert!(ext_result.is_ok(), "Расширение должно быть найдено");
    println!("⏱️ Расширение: {:?}", ext_duration);

    assert!(
        ext_duration.as_millis() < 10,
        "Парсинг расширения должен быть < 10ms, было: {:?}",
        ext_duration
    );

    println!("✅ Производительность detect_configuration_type приемлема");
    println!("   Базовая конфигурация: {:?}", conf_duration);
    println!("   Расширение: {:?}", ext_duration);
}

/// Тест: discover_all_configurations() на папке с множественными конфигурациями
///
/// Проверяет общую производительность обнаружения всех конфигураций.
#[test]
fn test_discover_all_configurations_performance() {
    // Arrange
    let parent_path = PathBuf::from("../examples/conf");
    let discovery = ConfigurationDiscovery::new(parent_path, false);

    // Прогрев
    let _ = discovery.discover_all_configurations();

    // Act
    let start = Instant::now();
    let result = discovery.discover_all_configurations();
    let duration = start.elapsed();

    // Assert
    assert!(result.is_ok(), "Конфигурации должны быть найдены");

    let configurations = result.unwrap();
    println!(
        "⏱️ Обнаружение {} конфигураций: {:?}",
        configurations.len(),
        duration
    );

    // Должно быть быстрым даже для множественных конфигураций
    assert!(
        duration.as_millis() < 50,
        "Обнаружение конфигураций должно быть < 50ms, было: {:?}",
        duration
    );

    println!("✅ Производительность discover_all_configurations приемлема");
}

/// Тест: множественные вызовы detect_configuration_type
///
/// Проверяет, что функция стабильна при повторных вызовах.
#[test]
fn test_detect_configuration_type_stability() {
    // Arrange
    let conf_path = PathBuf::from("../examples/conf/conf_test");
    let discovery = ConfigurationDiscovery::new(conf_path, false);

    let iterations = 100;

    // Act
    let start = Instant::now();
    for _ in 0..iterations {
        let result = discovery.discover_all_configurations();
        assert!(result.is_ok(), "Каждый вызов должен быть успешным");
    }
    let total_duration = start.elapsed();

    // Assert
    let avg_duration = total_duration / iterations;

    println!(
        "⏱️ Средняя производительность ({} итераций): {:?}",
        iterations, avg_duration
    );
    println!("   Общее время: {:?}", total_duration);

    assert!(
        avg_duration.as_micros() < 10_000, // < 10ms
        "Средняя производительность должна быть < 10ms, было: {:?}",
        avg_duration
    );

    println!("✅ Стабильность detect_configuration_type подтверждена");
}
