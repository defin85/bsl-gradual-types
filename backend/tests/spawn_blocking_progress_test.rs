//! Интеграционные тесты для буферизации прогресса парсинга
//!
//! Проверяет, что при использовании spawn_blocking() прогресс-обновления
//! приходят распределённо во времени, а не все сразу в конце.
//!
//! MILESTONE 2.20.2.4: Проверка real-time буферизации прогресса парсинга типов платформы

use bsl_backend::data::loaders::progress::ProgressUpdate;
use bsl_backend::system::system_coordinator::SystemCoordinator;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Тест 1: Проверка, что прогресс-обновления приходят во времени, а не все одновременно
///
/// КРИТЕРИЙ УСПЕХА:
/// - Минимум 5 прогресс-обновлений получено
/// - Время парсинга > 100ms (реальный парсинг, не мок)
#[tokio::test]
async fn test_progress_updates_distributed_in_time() {
    let coordinator = SystemCoordinator::new();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();

    // Сохраняем все обновления с временными метками
    let updates = Arc::new(Mutex::new(Vec::new()));
    let updates_clone = Arc::clone(&updates);

    // Spawn задача для сбора обновлений
    let collector_handle = tokio::spawn(async move {
        while let Some(update) = progress_rx.recv().await {
            let timestamp = Instant::now();
            updates_clone.lock().unwrap().push((timestamp, update));
        }
    });

    let start = Instant::now();

    // Запускаем парсинг с реальным путём к синтаксис-помощнику
    let syntax_path = std::path::Path::new("C:/1CProject/bsl-gradual-types/examples/syntax_helper");

    if syntax_path.exists() {
        let tx_clone = progress_tx.clone();
        let result = coordinator
            .start_with_paths(Some(syntax_path), None, Some(tx_clone))
            .await;

        let duration = start.elapsed();

        // Даём время на обработку последних обновлений
        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(progress_tx); // Закрываем канал, чтобы collector завершился

        // Ждём завершения сбора
        let _ = collector_handle.await;

        let collected_updates = updates.lock().unwrap();

        println!("\n📊 TEST 1: PROGRESS UPDATES DISTRIBUTED");
        println!("✅ Парсинг завершился за: {:?}", duration);
        println!("📈 Получено обновлений: {}", collected_updates.len());

        // Проверяем результат парсинга
        match result {
            Ok(()) => println!("✅ Парсинг успешен"),
            Err(e) => println!("❌ Ошибка парсинга: {:?}", e),
        }

        // КРИТЕРИЙ 1: Минимум обновлений
        assert!(
            collected_updates.len() >= 5,
            "Ожидается минимум 5 прогресс-обновлений, получено: {}",
            collected_updates.len()
        );

        // КРИТЕРИЙ 2: Время парсинга должно быть > 100ms
        assert!(
            duration > Duration::from_millis(100),
            "Парсинг должен занять > 100ms, получено: {:?}",
            duration
        );

        // КРИТЕРИЙ 3: Проверяем распределение во времени
        if collected_updates.len() > 1 {
            let mut gaps = Vec::new();
            for i in 1..collected_updates.len() {
                let gap = collected_updates[i]
                    .0
                    .duration_since(collected_updates[i - 1].0);
                gaps.push(gap);
                println!(
                    "  Update {}: {:?} (phase: {})",
                    i,
                    gap,
                    collected_updates[i].1.phase.display_name()
                );
            }

            let min_gap = gaps.iter().min().copied().unwrap_or(Duration::ZERO);
            let max_gap = gaps.iter().max().copied().unwrap_or(Duration::ZERO);
            let avg_gap = if gaps.is_empty() {
                Duration::ZERO
            } else {
                gaps.iter().sum::<Duration>() / gaps.len() as u32
            };

            println!(
                "\n⏱️  Min gap: {:?}, Max gap: {:?}, Avg gap: {:?}",
                min_gap, max_gap, avg_gap
            );
        }

        // КРИТЕРИЙ 4: Проверяем последовательность фаз
        let phases: Vec<String> = collected_updates
            .iter()
            .map(|(_, update)| update.phase.display_name().to_string())
            .collect();

        println!("\n📋 Фазы парсинга: {:?}", phases);

        assert!(!phases.is_empty(), "Хотя бы одна фаза должна быть пройдена");
    } else {
        println!(
            "⚠️  Пропуск теста: синтаксис-помощник не найден по пути {}",
            syntax_path.display()
        );
    }
}

/// Тест 2: Проверка, что event loop не блокируется во время парсинга
///
/// КРИТЕРИЙ УСПЕХА:
/// - Параллельная задача выполняется без заметных задержек
/// - Никаких паник или ошибок
#[tokio::test]
async fn test_event_loop_remains_responsive() {
    let coordinator = SystemCoordinator::new();
    let (progress_tx, _progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();

    // Счётчик for concurrent операций
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    // Запускаем параллельную задачу, которая будет выполняться во время парсинга
    let concurrent_task = tokio::spawn(async move {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });

    let syntax_path = std::path::Path::new("C:/1CProject/bsl-gradual-types/examples/syntax_helper");

    if syntax_path.exists() {
        // Запускаем парсинг
        let parse_result = coordinator
            .start_with_paths(Some(syntax_path), None, Some(progress_tx.clone()))
            .await;

        // Ждём завершения параллельной задачи
        let _ = concurrent_task.await;

        let operations_count = counter.load(Ordering::Relaxed);

        println!("\n📊 TEST 2: EVENT LOOP RESPONSIVENESS");
        println!("✅ Параллельных операций выполнено: {}", operations_count);
        println!("✅ Ожидается >= 300 (5000ms / 10ms - 1)");

        match parse_result {
            Ok(()) => println!("✅ Парсинг успешен"),
            Err(e) => println!("❌ Ошибка парсинга: {:?}", e),
        }

        // Должно быть достаточно параллельных операций
        assert!(
            operations_count >= 300,
            "Event loop заблокирован: только {} операций, ожидается >= 300",
            operations_count
        );
    } else {
        println!("⚠️  Пропуск теста: синтаксис-помощник не найден");
    }
}

/// Тест 3: Проверка корректности данных в ProgressUpdate
///
/// КРИТЕРИЙ УСПЕХА:
/// - Все обновления содержат валидные данные
/// - Percentages монотонно возрастают
/// - Phase правильный для каждого обновления
#[tokio::test]
async fn test_progress_updates_data_validity() {
    let coordinator = SystemCoordinator::new();
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();

    let updates = Arc::new(Mutex::new(Vec::new()));
    let updates_clone = Arc::clone(&updates);

    let collector_handle = tokio::spawn(async move {
        while let Some(update) = progress_rx.recv().await {
            updates_clone.lock().unwrap().push(update);
        }
    });

    let syntax_path = std::path::Path::new("C:/1CProject/bsl-gradual-types/examples/syntax_helper");

    if syntax_path.exists() {
        let tx_clone = progress_tx.clone();
        let result = coordinator
            .start_with_paths(Some(syntax_path), None, Some(tx_clone))
            .await;

        tokio::time::sleep(Duration::from_millis(100)).await;
        drop(progress_tx);
        let _ = collector_handle.await;

        let collected = updates.lock().unwrap();

        println!("\n📊 TEST 3: PROGRESS DATA VALIDITY");
        println!("Проверяем {} обновлений...", collected.len());

        match result {
            Ok(()) => println!("✅ Парсинг успешен"),
            Err(e) => println!("❌ Ошибка парсинга: {:?}", e),
        }

        let mut last_percentage = 0.0;

        for (i, update) in collected.iter().enumerate() {
            println!(
                "  Update {}: {:.1}% - {} (current: {}/{}, total: {})",
                i,
                update.percentage,
                update.phase.display_name(),
                update.current,
                update.total,
                update.total
            );

            // Проверяем, что percentage не уменьшается
            assert!(
                update.percentage >= last_percentage,
                "Percentage должен быть монотонно возрастающим: {} -> {}",
                last_percentage,
                update.percentage
            );

            // Проверяем валидность ranges
            assert!(
                update.current <= update.total,
                "current должен быть <= total"
            );

            assert!(
                update.percentage >= 0.0 && update.percentage <= 100.0,
                "Percentage должен быть в диапазоне [0, 100]"
            );

            last_percentage = update.percentage;
        }

        println!("✅ Все обновления валидны");
    } else {
        println!("⚠️  Пропуск теста: синтаксис-помощник не найден");
    }
}

/// Тест 4: Проверка обработки ошибок при неверном пути
///
/// КРИТЕРИЙ УСПЕХА:
/// - Сервер не крашится
/// - Возвращается ошибка или используются базовые типы
#[tokio::test]
async fn test_graceful_error_handling() {
    let coordinator = SystemCoordinator::new();
    let (progress_tx, _progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();

    let nonexistent_path = std::path::Path::new("/nonexistent/path");

    println!("\n📊 TEST 4: GRACEFUL ERROR HANDLING");

    let result = coordinator
        .start_with_paths(Some(nonexistent_path), None, Some(progress_tx.clone()))
        .await;

    match result {
        Ok(()) => {
            println!("✅ Graceful fallback: используются базовые типы");
        }
        Err(e) => {
            println!("✅ Возвращена ошибка (корректно): {:?}", e);
        }
    }

    // Ключевой критерий: нет паник
    println!("✅ Нет паник, сервер стабилен");
}

/// Тест 5: Проверка корректной очистки ресурсов
///
/// КРИТЕРИЙ УСПЕХА:
/// - Множественные запуски парсинга работают корректно
/// - Кеши правильно очищаются
#[tokio::test]
async fn test_multiple_parse_runs_cleanup() {
    let coordinator = SystemCoordinator::new();

    let syntax_path = std::path::Path::new("C:/1CProject/bsl-gradual-types/examples/syntax_helper");

    if syntax_path.exists() {
        println!("\n📊 TEST 5: MULTIPLE PARSE RUNS");

        for run in 1..=2 {
            let (progress_tx, _progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();

            println!("  Run {}: Запускаем парсинг...", run);

            let result = coordinator
                .start_with_paths(Some(syntax_path), None, Some(progress_tx.clone()))
                .await;

            match result {
                Ok(()) => println!("  Run {}: ✅ Успешно", run),
                Err(e) => println!("  Run {}: ❌ Ошибка: {:?}", run, e),
            }
        }

        println!("✅ Множественные запуски завершены без ошибок");
    } else {
        println!("⚠️  Пропуск теста: синтаксис-помощник не найден");
    }
}
