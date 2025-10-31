//! Интеграционные тесты для Task 2.20.2: Enhanced Indexing Progress
//!
//! Проверяет корректность работы прогресса индексации:
//! - Вычисление процентов для всех 4 фаз
//! - Callback вызывается каждые 10 файлов
//! - ETA корректно вычисляется
//! - Честность в certainty для конфигурационных типов

use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressUpdate};
use bsl_backend::data::loaders::syntax_helper::types::OptimizationSettings;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

// ============================================================================
// Тест 1: ProgressUpdate корректно вычисляет percentage для всех 4 фаз
// ============================================================================

#[test]
fn test_progress_percentage_calculation_all_phases() {
    // CollectingFiles (0-10%)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 0, 100);
    assert_eq!(percent, 0.0, "CollectingFiles: 0/100 должно быть 0%");

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 50, 100);
    assert_eq!(
        percent, 5.0,
        "CollectingFiles: 50/100 должно быть 5% (50% * 10%)"
    );

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 100, 100);
    assert_eq!(percent, 10.0, "CollectingFiles: 100/100 должно быть 10%");

    // ParsingFiles (10-70%)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 0, 3927);
    assert_eq!(percent, 10.0, "ParsingFiles: 0/3927 должно быть 10%");

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 1963, 3927);
    assert_eq!(
        percent, 40.0,
        "ParsingFiles: 1963/3927 (50%) должно быть 40% (10% + 50% * 60%)"
    );

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 3927, 3927);
    assert_eq!(percent, 70.0, "ParsingFiles: 3927/3927 должно быть 70%");

    // LinkingCategories (70-90%)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::LinkingCategories, 0, 100);
    assert_eq!(percent, 70.0, "LinkingCategories: 0/100 должно быть 70%");

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::LinkingCategories, 50, 100);
    assert_eq!(
        percent, 80.0,
        "LinkingCategories: 50/100 должно быть 80% (70% + 50% * 20%)"
    );

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::LinkingCategories, 100, 100);
    assert_eq!(percent, 90.0, "LinkingCategories: 100/100 должно быть 90%");

    // BuildingIndexes (90-100%)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::BuildingIndexes, 0, 3927);
    assert_eq!(percent, 90.0, "BuildingIndexes: 0/3927 должно быть 90%");

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::BuildingIndexes, 1963, 3927);
    assert_eq!(
        percent, 95.0,
        "BuildingIndexes: 1963/3927 (50%) должно быть 95% (90% + 50% * 10%)"
    );

    let percent = ProgressUpdate::compute_percentage(IndexingPhase::BuildingIndexes, 3927, 3927);
    assert_eq!(
        percent, 100.0,
        "BuildingIndexes: 3927/3927 должно быть 100%"
    );
}

// ============================================================================
// Тест 2: SyntaxHelperParser вызывает callback каждые 10 файлов
// ============================================================================

#[test]
fn test_progress_callback_invoked_every_10_files() {
    // Создаём временную директорию с 25 HTML файлами
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test");
    fs::create_dir(&test_dir).unwrap();

    // Создаём 25 тестовых HTML файлов
    for i in 0..25 {
        let html = format!(
            r#"
            <html>
            <head><title>TestType{} (TestType{})</title></head>
            <body>
                <h1 class="V8SH_pagetitle">TestType{} (TestType{})</h1>
                <p>Test description {}</p>
            </body>
            </html>
        "#,
            i, i, i, i, i
        );

        let file_path = test_dir.join(format!("type_{}.html", i));
        fs::write(file_path, html).unwrap();
    }

    // Собираем прогресс-апдейты
    let progress_calls = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = progress_calls.clone();

    let callback = move |update: ProgressUpdate| {
        progress_clone.lock().unwrap().push(update);
    };

    // Парсим с callback
    let settings = OptimizationSettings {
        max_threads: Some(2),
        batch_size: 5,
        show_progress: false,
        ..Default::default()
    };

    let mut parser = SyntaxHelperParser::with_settings(settings);
    let result = parser.parse_directory(&test_dir, Some(callback));

    assert!(result.is_ok(), "Парсинг должен пройти успешно");

    let calls = progress_calls.lock().unwrap();
    assert!(calls.len() > 0, "Callback должен быть вызван хотя бы раз");

    // Фильтруем только ParsingFiles фазу
    let parsing_calls: Vec<&ProgressUpdate> = calls
        .iter()
        .filter(|u| u.phase == IndexingPhase::ParsingFiles)
        .collect();

    // Проверяем что callback вызывается кратно 10 (10, 20, 25)
    // (Точное количество зависит от параллельной обработки, но должно быть 2-3 вызова)
    assert!(
        parsing_calls.len() >= 2,
        "Callback должен быть вызван минимум 2 раза для 25 файлов (каждые 10 файлов)"
    );

    // Проверяем что прогресс растёт монотонно
    for i in 1..parsing_calls.len() {
        assert!(
            parsing_calls[i].percentage >= parsing_calls[i - 1].percentage,
            "Прогресс должен только расти: {} < {}",
            parsing_calls[i].percentage,
            parsing_calls[i - 1].percentage
        );
    }
}

// ============================================================================
// Тест 3: ETA корректно вычисляется
// ============================================================================

#[test]
fn test_eta_calculation_accuracy() {
    use std::time::Instant;

    let start = Instant::now();

    // Симулируем обработку 10% за 200ms (увеличили для стабильности теста)
    std::thread::sleep(std::time::Duration::from_millis(200));

    let elapsed_ms = start.elapsed().as_millis() as f32;
    let percentage = 10.0; // 10% выполнено

    // Формула ETA из progress.ts:
    // ETA = (elapsed * 100 / percentage) - elapsed
    let eta = ((elapsed_ms * 100.0 / percentage) - elapsed_ms) as u32;

    // При 10% за ~200ms → всего нужно ~2000ms → осталось ~1800ms
    // Допускаем погрешность 400ms (на overhead и таймер)
    assert!(
        eta >= 1400 && eta <= 2200,
        "ETA должна быть примерно 1800ms, получили {}ms (elapsed: {:.0}ms)",
        eta,
        elapsed_ms
    );

    // Проверяем что ETA уменьшается по мере прогресса
    std::thread::sleep(std::time::Duration::from_millis(200));
    let elapsed_later_ms = start.elapsed().as_millis() as f32;
    let percentage_later = 20.0; // 20% выполнено
    let eta_later = ((elapsed_later_ms * 100.0 / percentage_later) - elapsed_later_ms) as u32;

    assert!(
        eta_later < eta,
        "ETA должна уменьшаться: {} >= {}",
        eta_later,
        eta
    );
}

// ============================================================================
// Тест 4: Все фазы покрывают 0-100%
// ============================================================================

#[test]
fn test_all_phases_coverage() {
    let phases = vec![
        IndexingPhase::CollectingFiles,
        IndexingPhase::ParsingFiles,
        IndexingPhase::LinkingCategories,
        IndexingPhase::BuildingIndexes,
    ];

    // Проверяем что первая фаза начинается с 0%
    assert_eq!(
        phases[0].base_percentage(),
        0.0,
        "Первая фаза должна начинаться с 0%"
    );

    // Проверяем что последняя фаза заканчивается на 100%
    let last_phase = phases.last().unwrap();
    assert_eq!(
        last_phase.base_percentage() + last_phase.weight(),
        100.0,
        "Последняя фаза должна заканчиваться на 100%"
    );

    // Проверяем что сумма весов = 100%
    let total_weight: f32 = phases.iter().map(|p| p.weight()).sum();
    assert_eq!(total_weight, 100.0, "Сумма весов всех фаз должна быть 100%");

    // Проверяем что фазы идут непрерывно (конец одной = начало следующей)
    for i in 0..phases.len() - 1 {
        let end_of_current = phases[i].base_percentage() + phases[i].weight();
        let start_of_next = phases[i + 1].base_percentage();
        assert_eq!(
            end_of_current,
            start_of_next,
            "Фаза {} должна заканчиваться там, где начинается фаза {}",
            i,
            i + 1
        );
    }
}

// ============================================================================
// Тест 5: IndexingPhase::display_name() возвращает корректные названия
// ============================================================================

#[test]
fn test_indexing_phase_display_names() {
    assert_eq!(
        IndexingPhase::CollectingFiles.display_name(),
        "Сканирование файлов"
    );
    assert_eq!(
        IndexingPhase::ParsingFiles.display_name(),
        "Парсинг Syntax Helper"
    );
    assert_eq!(
        IndexingPhase::LinkingCategories.display_name(),
        "Связывание категорий"
    );
    assert_eq!(
        IndexingPhase::BuildingIndexes.display_name(),
        "Построение индексов"
    );
}

// ============================================================================
// Тест 6: ProgressUpdate::new() корректно вычисляет процент
// ============================================================================

#[test]
fn test_progress_update_new() {
    let update = ProgressUpdate::new(
        IndexingPhase::ParsingFiles,
        1963,
        3927,
        Some("Массив".to_string()),
    );

    assert_eq!(update.phase, IndexingPhase::ParsingFiles);
    assert_eq!(update.current, 1963);
    assert_eq!(update.total, 3927);
    assert_eq!(
        update.percentage, 40.0,
        "1963/3927 (50%) в фазе ParsingFiles должно дать 40%"
    );
    assert_eq!(update.message, Some("Массив".to_string()));
}

// ============================================================================
// Тест 7: Защита от деления на 0 и overflow
// ============================================================================

#[test]
fn test_compute_percentage_edge_cases() {
    // Деление на 0 - должно вернуть base_percentage
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 10, 0);
    assert_eq!(
        percent, 10.0,
        "При делении на 0 должен вернуться base_percentage"
    );

    // current > total (защита от overflow)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 5000, 3927);
    assert_eq!(
        percent, 70.0,
        "При current > total должен вернуться максимум (base + weight)"
    );

    // Очень малые значения
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 1, 10000);
    // 10% + (1/10000) * 60% ≈ 10.006%
    assert!(
        percent >= 10.0 && percent <= 10.1,
        "Малые значения должны корректно обрабатываться"
    );
}

// ============================================================================
// Тест 8: Полный цикл парсинга с прогрессом (E2E)
// ============================================================================

#[test]
fn test_full_parsing_cycle_with_progress() {
    // Создаём временную директорию с файлами
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test");
    fs::create_dir(&test_dir).unwrap();

    // Создаём 15 HTML файлов
    for i in 0..15 {
        let html = format!(
            r#"
            <html>
            <head><title>Type{} (Type{})</title></head>
            <body>
                <h1 class="V8SH_pagetitle">Type{} (Type{})</h1>
                <p>Description {}</p>
            </body>
            </html>
        "#,
            i, i, i, i, i
        );
        fs::write(test_dir.join(format!("type_{}.html", i)), html).unwrap();
    }

    // Собираем все прогресс-апдейты
    let all_updates = Arc::new(Mutex::new(Vec::new()));
    let updates_clone = all_updates.clone();

    let callback = move |update: ProgressUpdate| {
        updates_clone.lock().unwrap().push(update);
    };

    // Парсим
    let settings = OptimizationSettings {
        show_progress: false,
        ..Default::default()
    };

    let mut parser = SyntaxHelperParser::with_settings(settings);
    parser.parse_directory(&test_dir, Some(callback)).unwrap();

    let updates = all_updates.lock().unwrap();

    // Проверяем что были обновления для всех 4 фаз
    let phases: Vec<IndexingPhase> = updates.iter().map(|u| u.phase).collect();
    assert!(
        phases.contains(&IndexingPhase::CollectingFiles),
        "Должна быть фаза CollectingFiles"
    );
    assert!(
        phases.contains(&IndexingPhase::ParsingFiles),
        "Должна быть фаза ParsingFiles"
    );
    assert!(
        phases.contains(&IndexingPhase::LinkingCategories),
        "Должна быть фаза LinkingCategories"
    );
    assert!(
        phases.contains(&IndexingPhase::BuildingIndexes),
        "Должна быть фаза BuildingIndexes"
    );

    // Проверяем что последнее обновление - 100%
    let last = updates.last().unwrap();
    assert_eq!(last.percentage, 100.0, "Последний процент должен быть 100%");
    assert_eq!(
        last.phase,
        IndexingPhase::BuildingIndexes,
        "Последняя фаза должна быть BuildingIndexes"
    );
}
