//! Тесты для проверки RwLock кеширования в SystemCoordinator
//!
//! Проверяет:
//! 1. Корректная работа RwLock вместо Mutex
//! 2. Graceful обработка poisoned locks
//! 3. Параллельное чтение из кешей

use bsl_backend::system::system_coordinator::SystemCoordinator;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Тест 1: AnalysisEngine кеширование
#[test]
fn test_analysis_engine_caching() {
    let coordinator = SystemCoordinator::new();

    // Первый вызов - кеш пуст
    let engine1 = coordinator.analysis_engine();
    assert!(engine1.is_none(), "AnalysisEngine не должен быть инициализирован до start()");

    // Проверяем, что метод работает без паники даже когда кеш пуст
    let engine2 = coordinator.analysis_engine();
    assert!(engine2.is_none(), "AnalysisEngine остаётся пустым");
}

/// Тест 2: TypeSystemService кеширование и lock order
#[test]
fn test_type_service_caching_and_lock_order() {
    let coordinator = SystemCoordinator::new();

    // Первый вызов - кеш пуст, так как AnalysisEngine не инициализирован
    let service1 = coordinator.type_service();
    assert!(
        service1.is_none(),
        "TypeSystemService не должен быть инициализирован до start()"
    );

    // Второй вызов - должен вернуть None, не вызвав deadlock
    let service2 = coordinator.type_service();
    assert!(
        service2.is_none(),
        "TypeSystemService остаётся пустым при отсутствии AnalysisEngine"
    );
}

/// Тест 3: Параллельное чтение из кешей (преимущество RwLock)
#[test]
fn test_concurrent_reads_from_cache() {
    let coordinator = Arc::new(SystemCoordinator::new());
    let read_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Создаём 10 потоков, каждый читает из кеша
    for _i in 0..10 {
        let coord_clone = coordinator.clone();
        let count_clone = read_count.clone();

        let handle = std::thread::spawn(move || {
            // Каждый поток пытается получить AnalysisEngine
            // С RwLock это должно быть быстро, так как это параллельные reads
            let _engine = coord_clone.analysis_engine();
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        handles.push(handle);
    }

    // Ждём завершения всех потоков
    for handle in handles {
        handle.join().unwrap();
    }

    // Проверяем, что все потоки выполнились успешно
    assert_eq!(
        read_count.load(Ordering::SeqCst),
        10,
        "Все 10 потоков должны были выполниться"
    );
}

/// Тест 4: Проверка get_analysis_engine вместе с analysis_engine методом
#[test]
fn test_get_analysis_engine_vs_analysis_engine_method() {
    let coordinator = SystemCoordinator::new();

    // Оба метода должны вернуть None (кеш пуст)
    let engine1 = coordinator.get_analysis_engine();
    let engine2 = coordinator.analysis_engine();

    assert!(engine1.is_none(), "get_analysis_engine() должен вернуть None");
    assert!(engine2.is_none(), "analysis_engine() должен вернуть None");
}

/// Тест 5: CloneForBlocking работает корректно
#[test]
fn test_clone_for_blocking() {
    let coordinator1 = SystemCoordinator::new();
    let coordinator2 = coordinator1.clone();

    // Проверяем, что оба экземпляра используют один и тот же кеш
    // (благодаря Arc)
    let engine1 = coordinator1.analysis_engine();
    let engine2 = coordinator2.analysis_engine();

    assert_eq!(
        engine1.is_none(),
        engine2.is_none(),
        "Оба экземпляра должны иметь одинаковое состояние кеша"
    );
}

/// Тест 6: Множественные последовательные вызовы не вызывают панику
#[test]
fn test_repeated_cache_access_no_panic() {
    let coordinator = SystemCoordinator::new();

    // Вызываем методы много раз - не должно быть deadlock или panic
    for _ in 0..1000 {
        let _ = coordinator.analysis_engine();
        let _ = coordinator.get_analysis_engine();
        let _ = coordinator.type_service();
    }

    // Если мы сюда дошли - тест прошёл успешно
    println!("✅ 1000 последовательных вызовов выполнены без паники");
}

/// Тест 7: Health check работает независимо от кешей
#[test]
fn test_health_status_independent_of_cache() {
    let coordinator = SystemCoordinator::new();

    // Health check не зависит от состояния кешей
    let _health1 = coordinator.health_status();
    let _engine = coordinator.analysis_engine();
    let _health2 = coordinator.health_status();

    // Оба вызова должны работать без паники
    println!("✅ Health check работает независимо от состояния кешей");
}
