use super::*;

#[test]
fn test_indexing_phase_base_percentage() {
    assert_eq!(IndexingPhase::CollectingFiles.base_percentage(), 0.0);
    assert_eq!(IndexingPhase::ParsingFiles.base_percentage(), 10.0);
    assert_eq!(IndexingPhase::LinkingCategories.base_percentage(), 70.0);
    assert_eq!(IndexingPhase::BuildingIndexes.base_percentage(), 90.0);
}

#[test]
fn test_indexing_phase_weight() {
    // Платформа
    assert_eq!(IndexingPhase::CollectingFiles.weight(), 10.0);
    assert_eq!(IndexingPhase::ParsingFiles.weight(), 60.0);
    assert_eq!(IndexingPhase::LinkingCategories.weight(), 20.0);
    assert_eq!(IndexingPhase::BuildingIndexes.weight(), 10.0);

    // Сумма весов платформы = 100.0
    let total_weight_platform = IndexingPhase::CollectingFiles.weight()
        + IndexingPhase::ParsingFiles.weight()
        + IndexingPhase::LinkingCategories.weight()
        + IndexingPhase::BuildingIndexes.weight();
    assert_eq!(total_weight_platform, 100.0);

    // Конфигурация
    assert_eq!(IndexingPhase::ConfigurationDiscovery.weight(), 5.0);
    assert_eq!(IndexingPhase::ConfigurationParsing.weight(), 75.0);
    assert_eq!(IndexingPhase::ConfigurationLinking.weight(), 10.0);
    assert_eq!(IndexingPhase::ConfigurationFinalizing.weight(), 5.0);
    assert_eq!(IndexingPhase::ConfigurationIndexingModules.weight(), 5.0);

    // Сумма весов конфигурации = 100.0
    let total_weight_config = IndexingPhase::ConfigurationDiscovery.weight()
        + IndexingPhase::ConfigurationParsing.weight()
        + IndexingPhase::ConfigurationLinking.weight()
        + IndexingPhase::ConfigurationFinalizing.weight()
        + IndexingPhase::ConfigurationIndexingModules.weight();
    assert_eq!(total_weight_config, 100.0);
}

#[test]
fn test_indexing_phase_display_name() {
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

#[test]
fn test_compute_percentage_collecting_files() {
    // Начало фазы: 0/100
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 0, 100);
    assert_eq!(percent, 0.0);

    // Середина фазы: 50/100
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 50, 100);
    assert_eq!(percent, 5.0); // 0% (base) + 50% * 10% (weight)

    // Конец фазы: 100/100
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::CollectingFiles, 100, 100);
    assert_eq!(percent, 10.0);
}

#[test]
fn test_compute_percentage_parsing_files() {
    // Начало фазы: 0/3927
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 0, 3927);
    assert_eq!(percent, 10.0);

    // Середина фазы: 1963/3927 (50%)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 1963, 3927);
    assert_eq!(percent, 40.0); // 10% (base) + 50% * 60% (weight) = 40.0, округлено до 40

    // Конец фазы: 3927/3927
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 3927, 3927);
    assert_eq!(percent, 70.0);
}

#[test]
fn test_compute_percentage_linking_categories() {
    // Середина фазы: 50/100
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::LinkingCategories, 50, 100);
    assert_eq!(percent, 80.0); // 70% (base) + 50% * 20% (weight)
}

#[test]
fn test_compute_percentage_building_indexes() {
    // Конец фазы: 3927/3927
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::BuildingIndexes, 3927, 3927);
    assert_eq!(percent, 100.0);
}

#[test]
fn test_compute_percentage_edge_cases() {
    // Деление на 0
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 10, 0);
    assert_eq!(percent, 10.0); // Возвращает base_percentage

    // current > total (защита от overflow)
    let percent = ProgressUpdate::compute_percentage(IndexingPhase::ParsingFiles, 5000, 3927);
    assert_eq!(percent, 70.0); // Максимум = base + weight
}

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
    assert_eq!(update.percentage, 40.0); // Округлено до целого
    assert_eq!(update.message, Some("Массив".to_string()));
}
