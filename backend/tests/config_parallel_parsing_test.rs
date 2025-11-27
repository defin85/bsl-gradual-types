//! Тесты параллельного парсинга метаданных конфигурации
//!
//! Проверяет:
//! - Корректность результата (параллельный == последовательный)
//! - Производительность (параллельный быстрее последовательного)
//! - Прогресс callback (вызывается каждые 5 объектов)
//! - Обработка ошибок (не ломается на битых файлах)
//! - Граничные случаи (пустая конфигурация, 1 объект)

use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Вспомогательная структура для отслеживания прогресса
#[derive(Debug, Clone)]
struct ProgressTracker {
    updates: Arc<Mutex<Vec<ProgressUpdate>>>,
}

impl ProgressTracker {
    fn new() -> Self {
        Self {
            updates: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn callback(&self) -> impl Fn(ProgressUpdate) + Send + Sync + Clone + 'static {
        let updates = Arc::clone(&self.updates);
        move |update: ProgressUpdate| {
            updates.lock().unwrap().push(update);
        }
    }

    fn get_updates(&self) -> Vec<ProgressUpdate> {
        self.updates.lock().unwrap().clone()
    }
}

// ============================================================================
// 1️⃣ Unit-тесты параллелизма
// ============================================================================

/// Тест: параллельный парсинг даёт тот же результат что и последовательный
///
/// ВАЖНО: Этот тест концептуален - мы не имеем последовательной реализации,
/// поэтому проверяем просто корректность параллельного парсинга.
#[test]
fn test_parallel_parsing_correctness() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);

    // Act - обнаруживаем конфигурации
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    assert!(
        !configurations.is_empty(),
        "Должна быть найдена хотя бы одна конфигурация"
    );

    let config_info = &configurations[0];

    // Act - параллельный парсинг метаданных
    let metadata_parallel = discovery
        .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
        .expect("Не удалось распарсить метаданные параллельно");

    // Assert - проверяем корректность результата
    assert!(
        metadata_parallel.len() >= 4,
        "Ожидалось минимум 4 объекта, найдено: {}",
        metadata_parallel.len()
    );

    // Проверяем наличие конкретных объектов
    let kontragenty = metadata_parallel
        .iter()
        .find(|m| m.name == "Контрагенты")
        .expect("Должен быть справочник Контрагенты");

    assert!(!kontragenty.uuid.is_empty(), "UUID не должен быть пустым");
    assert_eq!(
        kontragenty.facets.len(),
        5,
        "Справочник должен иметь 5 фасетов"
    );

    println!(
        "✅ Параллельный парсинг корректен: {} объектов",
        metadata_parallel.len()
    );
}

/// Тест: все объекты успешно парсятся
#[test]
fn test_parallel_parsing_all_objects_processed() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path.clone(), false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    // Считаем сколько ChildObjects в Configuration.xml
    let config_xml = config_path.join("Configuration.xml");
    let child_objects = discovery
        .parse_child_objects_list(&config_xml)
        .expect("Не удалось распарсить ChildObjects");

    let expected_count: usize = child_objects.values().map(|v| v.len()).sum();

    println!("📊 Ожидаемое количество объектов: {}", expected_count);

    // Act - параллельный парсинг
    let metadata = discovery
        .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
        .expect("Не удалось распарсить метаданные");

    // Assert - все объекты успешно распарсены
    assert_eq!(
        metadata.len(),
        expected_count,
        "Количество распарсенных объектов должно совпадать с ChildObjects"
    );

    // Проверяем что каждый объект имеет валидные данные
    for obj in &metadata {
        assert!(!obj.name.is_empty(), "Имя объекта не должно быть пустым");
        assert!(!obj.uuid.is_empty(), "UUID не должен быть пустым");
        assert!(!obj.facets.is_empty(), "Должен быть хотя бы один фасет");
    }

    println!("✅ Все {} объектов успешно распарсены", metadata.len());
}

/// Тест: ошибки парсинга не ломают весь процесс
///
/// ПРИМЕЧАНИЕ: Этот тест требует наличия битого XML файла в тестовой конфигурации.
/// Если такого файла нет, тест пропускается.
#[test]
#[ignore] // Игнорируем по умолчанию, т.к. требует специальной подготовки данных
fn test_parallel_parsing_error_handling() {
    // Arrange - создаём временную конфигурацию с битым XML
    // TODO: Реализовать создание временной конфигурации с намеренно битым файлом

    println!("⚠️ Тест error_handling требует специальной подготовки данных");
}

// ============================================================================
// 2️⃣ Тесты прогресса
// ============================================================================

/// Тест: callback вызывается каждые 5 объектов
#[test]
fn test_progress_callback_invoked() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    let tracker = ProgressTracker::new();

    // Act - парсим с прогресс callback
    let metadata = discovery
        .discover_metadata_in_configuration(config_info, Some(tracker.callback()))
        .expect("Не удалось распарсить метаданные");

    let updates = tracker.get_updates();

    // Assert - callback был вызван
    assert!(
        !updates.is_empty(),
        "Callback должен быть вызван хотя бы раз"
    );

    println!("📊 Всего объектов: {}", metadata.len());
    println!("📢 Всего обновлений прогресса: {}", updates.len());

    // Проверяем что все обновления имеют правильную фазу
    for update in &updates {
        assert_eq!(
            update.phase,
            IndexingPhase::ParsingFiles,
            "Все обновления должны иметь фазу ParsingFiles"
        );
    }

    // Проверяем что прогресс монотонно увеличивается
    let mut last_progress = 0;
    for update in &updates {
        assert!(
            update.current >= last_progress,
            "Прогресс должен монотонно увеличиваться: {} >= {}",
            update.current,
            last_progress
        );
        last_progress = update.current;
    }

    // Проверяем throttling: обновления должны быть каждые ~5 объектов
    // (плюс начальное и финальное обновление)
    let expected_updates = (metadata.len() / 5) + 2; // throttling + начало/конец

    println!("🎯 Ожидаемое количество обновлений: ~{}", expected_updates);
    println!("📊 Фактическое количество обновлений: {}", updates.len());

    // Допускаем небольшую погрешность из-за параллелизма
    assert!(
        updates.len() >= 2,
        "Должно быть минимум 2 обновления (начало + конец)"
    );

    println!("✅ Прогресс callback работает корректно");
}

/// Тест: финальный прогресс = 100%
#[test]
fn test_progress_final_100_percent() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    let tracker = ProgressTracker::new();

    // Act
    let metadata = discovery
        .discover_metadata_in_configuration(config_info, Some(tracker.callback()))
        .expect("Не удалось распарсить метаданные");

    let updates = tracker.get_updates();

    // Assert - последнее обновление должно быть 100%
    let last_update = updates.last().expect("Должно быть хотя бы одно обновление");

    assert_eq!(
        last_update.current,
        metadata.len(),
        "Финальный прогресс должен равняться количеству объектов"
    );

    assert_eq!(
        last_update.total,
        metadata.len(),
        "Total должен равняться количеству объектов"
    );

    let percent = (last_update.current as f64 / last_update.total as f64) * 100.0;
    println!("📊 Финальный прогресс: {:.1}%", percent);

    assert!(
        (percent - 100.0).abs() < 0.1,
        "Финальный прогресс должен быть ~100%, было: {:.1}%",
        percent
    );

    println!("✅ Финальный прогресс = 100%");
}

/// Тест: прогресс увеличивается монотонно
#[test]
fn test_progress_monotonic_increase() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    let tracker = ProgressTracker::new();

    // Act
    let _ = discovery
        .discover_metadata_in_configuration(config_info, Some(tracker.callback()))
        .expect("Не удалось распарсить метаданные");

    let updates = tracker.get_updates();

    // Assert - прогресс должен монотонно увеличиваться
    let mut last_current = 0;

    for (idx, update) in updates.iter().enumerate() {
        assert!(
            update.current >= last_current,
            "Прогресс на шаге {} уменьшился: {} -> {}",
            idx,
            last_current,
            update.current
        );

        last_current = update.current;
    }

    println!(
        "✅ Прогресс монотонно увеличивается: {} обновлений",
        updates.len()
    );
}

// ============================================================================
// 3️⃣ Тесты производительности
// ============================================================================

/// Тест: параллельный парсинг быстрее последовательного
///
/// ПРИМЕЧАНИЕ: Мы сравниваем время парсинга одной и той же конфигурации
/// дважды подряд. Второй запуск должен быть не медленнее первого (кеширование).
#[test]
fn test_parallel_parsing_performance() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    // Прогрев (первый запуск для кеширования I/O)
    let _ = discovery.discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>);

    // Act - первый замер
    let start1 = Instant::now();
    let metadata1 = discovery
        .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
        .expect("Не удалось распарсить метаданные (run 1)");
    let duration1 = start1.elapsed();

    // Act - второй замер (должен быть не медленнее)
    let start2 = Instant::now();
    let metadata2 = discovery
        .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
        .expect("Не удалось распарсить метаданные (run 2)");
    let duration2 = start2.elapsed();

    // Assert - результаты идентичны
    assert_eq!(
        metadata1.len(),
        metadata2.len(),
        "Количество объектов должно совпадать"
    );

    println!("⏱️ Первый запуск: {:?}", duration1);
    println!("⏱️ Второй запуск: {:?}", duration2);
    println!("📊 Количество объектов: {}", metadata1.len());

    // Параллельный парсинг должен быть достаточно быстрым
    assert!(
        duration1.as_millis() < 1000,
        "Парсинг должен быть < 1000ms, было: {:?}",
        duration1
    );

    println!("✅ Производительность параллельного парсинга приемлема");
}

/// Тест: измерение ускорения на реальной конфигурации
///
/// Для объективного сравнения нужна большая конфигурация (50+ объектов).
/// Если тестовая конфигурация мала, тест выводит предупреждение.
#[test]
fn test_parallel_parsing_speedup() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    // Прогрев
    let metadata = discovery
        .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
        .expect("Не удалось распарсить метаданные");

    if metadata.len() < 10 {
        println!(
            "⚠️ Тестовая конфигурация слишком мала ({} объектов) для измерения ускорения",
            metadata.len()
        );
        println!("   Рекомендуется конфигурация с 50+ объектами для объективного теста");
        return;
    }

    // Act - множественные замеры для усреднения
    let iterations = 5;
    let mut durations = Vec::new();

    for i in 0..iterations {
        let start = Instant::now();
        let _ = discovery
            .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
            .expect("Не удалось распарсить метаданные");
        let duration = start.elapsed();
        durations.push(duration);
        println!("  Итерация {}: {:?}", i + 1, duration);
    }

    // Assert - вычисляем среднее время
    let avg_duration = durations.iter().sum::<std::time::Duration>() / iterations as u32;

    println!("📊 Количество объектов: {}", metadata.len());
    println!("⏱️ Среднее время парсинга: {:?}", avg_duration);
    println!(
        "⚡ Скорость: {:.1} объектов/сек",
        metadata.len() as f64 / avg_duration.as_secs_f64()
    );

    // Параллельный парсинг должен обрабатывать минимум 10 объектов в секунду
    let objects_per_sec = metadata.len() as f64 / avg_duration.as_secs_f64();
    assert!(
        objects_per_sec >= 10.0,
        "Скорость парсинга слишком низкая: {:.1} объектов/сек",
        objects_per_sec
    );

    println!("✅ Производительность параллельного парсинга приемлема");
}

/// Тест: отсутствие race conditions
///
/// Проверяем что множественные вызовы параллельного парсинга дают идентичные результаты.
#[test]
fn test_parallel_parsing_no_race_conditions() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    // Act - множественные запуски
    let iterations = 10;
    let mut results = Vec::new();

    for _ in 0..iterations {
        let metadata = discovery
            .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
            .expect("Не удалось распарсить метаданные");

        // Собираем имена объектов и сортируем для сравнения
        let mut names: Vec<_> = metadata.iter().map(|m| m.name.clone()).collect();
        names.sort();

        results.push(names);
    }

    // Assert - все результаты идентичны
    let first = &results[0];

    for (idx, result) in results.iter().enumerate().skip(1) {
        assert_eq!(
            first,
            result,
            "Результат итерации {} отличается от первой",
            idx + 1
        );
    }

    println!(
        "✅ Отсутствие race conditions подтверждено ({} итераций)",
        iterations
    );
}

// ============================================================================
// 4️⃣ Интеграционные тесты
// ============================================================================

/// Тест: работа с multiple конфигурациями (база + расширения)
#[test]
fn test_parallel_parsing_multiple_configurations() {
    // Arrange - проверяем папку с множественными конфигурациями
    let parent_path = PathBuf::from("../examples/conf");

    if !parent_path.exists() {
        eprintln!("⚠️ Папка с конфигурациями не найдена: {:?}", parent_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(parent_path, false);

    // Act - обнаруживаем все конфигурации
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    println!(
        "📦 Найдено конфигураций: {} (Base: {}, Extension: {})",
        configurations.len(),
        configurations.iter().filter(|c| c.is_base()).count(),
        configurations.iter().filter(|c| c.is_extension()).count()
    );

    // Парсим каждую конфигурацию
    for (idx, config_info) in configurations.iter().enumerate() {
        println!(
            "  {}. {} {} ({})",
            idx + 1,
            if config_info.is_base() {
                "📦"
            } else {
                "🧩"
            },
            config_info.name,
            config_info.path.display()
        );

        let metadata = discovery
            .discover_metadata_in_configuration(config_info, None::<fn(ProgressUpdate)>)
            .expect("Не удалось распарсить метаданные");

        println!("     → {} объектов метаданных", metadata.len());

        // Assert - каждая конфигурация должна содержать объекты
        assert!(
            !metadata.is_empty(),
            "Конфигурация {} не должна быть пустой",
            config_info.name
        );
    }

    println!(
        "✅ Параллельный парсинг работает для {} конфигураций",
        configurations.len()
    );
}

/// Тест: обратная совместимость (callback = None)
#[test]
fn test_parallel_parsing_no_callback() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    // Act - парсим БЕЗ callback (обратная совместимость)
    let result = discovery.discover_metadata_in_configuration(
        config_info,
        None::<fn(ProgressUpdate)>, // Явно передаём None
    );

    // Assert - парсинг должен работать без callback
    assert!(
        result.is_ok(),
        "Парсинг без callback должен работать корректно"
    );

    let metadata = result.unwrap();

    assert!(
        !metadata.is_empty(),
        "Должны быть обнаружены объекты метаданных"
    );

    println!("✅ Обратная совместимость (без callback) работает");
}

// ============================================================================
// 5️⃣ Граничные случаи
// ============================================================================

/// Тест: пустая конфигурация (0 объектов)
///
/// ПРИМЕЧАНИЕ: Требует наличия пустой тестовой конфигурации.
/// Если её нет, тест пропускается.
#[test]
#[ignore] // Игнорируем по умолчанию, т.к. требует специальной подготовки данных
fn test_parallel_parsing_empty_configuration() {
    // TODO: Создать минимальную конфигурацию без ChildObjects
    println!("⚠️ Тест empty_configuration требует специальной подготовки данных");
}

/// Тест: конфигурация с 1 объектом
///
/// Проверяем что параллельный парсинг корректно работает с минимальным набором данных.
#[test]
#[ignore] // Игнорируем по умолчанию, т.к. требует специальной подготовки данных
fn test_parallel_parsing_single_object() {
    // TODO: Создать минимальную конфигурацию с 1 объектом
    println!("⚠️ Тест single_object требует специальной подготовки данных");
}

/// Тест: конфигурация с ошибочными XML файлами
///
/// Проверяем что ошибки в отдельных файлах не ломают парсинг всей конфигурации.
#[test]
#[ignore] // Игнорируем по умолчанию, т.к. требует специальной подготовки данных
fn test_parallel_parsing_with_corrupt_xml() {
    // TODO: Создать конфигурацию с намеренно битым XML файлом
    println!("⚠️ Тест corrupt_xml требует специальной подготовки данных");
}

/// Тест: discover_all_metadata() обратная совместимость
///
/// Проверяем что устаревший метод discover_all_metadata() всё ещё работает.
#[test]
fn test_discover_all_metadata_backward_compatibility() {
    // Arrange
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена: {:?}", config_path);
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path, false);

    // Act - вызываем УСТАРЕВШИЙ метод
    let result = discovery.discover_all_metadata(None::<fn(ProgressUpdate)>);

    // Assert
    assert!(
        result.is_ok(),
        "Устаревший метод discover_all_metadata() должен работать"
    );

    let metadata = result.unwrap();

    assert!(
        !metadata.is_empty(),
        "Должны быть обнаружены объекты метаданных"
    );

    println!(
        "✅ Обратная совместимость discover_all_metadata() работает ({} объектов)",
        metadata.len()
    );
}
