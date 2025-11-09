//! Интеграционные тесты для прогресса при парсинге конфигурации (Milestone 2.20)

#![allow(clippy::double_comparisons)]

#[cfg(test)]
mod progress_config_tests {
    use bsl_backend::data::loaders::progress::{IndexingPhase, ProgressSource, ProgressUpdate};
    use std::sync::{Arc, Mutex};

    /// Тест 1: Проверка что ProgressSource::source() корректно определяет Platform/Configuration
    #[test]
    fn test_phase_source_discrimination() {
        // Все фазы платформы должны возвращать Platform
        assert_eq!(
            IndexingPhase::CollectingFiles.source(),
            ProgressSource::Platform
        );
        assert_eq!(
            IndexingPhase::ParsingFiles.source(),
            ProgressSource::Platform
        );
        assert_eq!(
            IndexingPhase::LinkingCategories.source(),
            ProgressSource::Platform
        );
        assert_eq!(
            IndexingPhase::BuildingIndexes.source(),
            ProgressSource::Platform
        );

        // Все фазы конфигурации должны возвращать Configuration
        assert_eq!(
            IndexingPhase::ConfigurationDiscovery.source(),
            ProgressSource::Configuration
        );
        assert_eq!(
            IndexingPhase::ConfigurationParsing.source(),
            ProgressSource::Configuration
        );
        assert_eq!(
            IndexingPhase::ConfigurationLinking.source(),
            ProgressSource::Configuration
        );
        assert_eq!(
            IndexingPhase::ConfigurationFinalizing.source(),
            ProgressSource::Configuration
        );
    }

    /// Тест 2: Проверка вычисления процентов для всех фаз конфигурации
    #[test]
    fn test_configuration_phases_percentage_calculation() {
        // ConfigurationDiscovery: 0-5%
        let percent_0 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationDiscovery, 0, 10);
        let percent_50 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationDiscovery, 5, 10);
        let percent_100 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationDiscovery, 10, 10);

        assert_eq!(percent_0, 0.0);
        assert_eq!(percent_50, 3.0); // Было 2.5, теперь округлено до 3.0
        assert_eq!(percent_100, 5.0);

        // ConfigurationParsing: 5-85%
        let percent_0 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 0, 100);
        let percent_50 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 50, 100);
        let percent_100 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 100, 100);

        assert_eq!(percent_0, 5.0);
        assert_eq!(percent_50, 45.0);
        assert_eq!(percent_100, 85.0);

        // ConfigurationLinking: 85-95%
        let percent_0 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationLinking, 0, 1);
        let percent_100 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationLinking, 1, 1);

        assert_eq!(percent_0, 85.0);
        assert_eq!(percent_100, 95.0);

        // ConfigurationFinalizing: 95-100%
        let percent_0 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationFinalizing, 0, 1);
        let percent_100 =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationFinalizing, 1, 1);

        assert_eq!(percent_0, 95.0);
        assert_eq!(percent_100, 100.0);
    }

    /// Тест 3: Проверка что все веса фаз конфигурации дают в сумме 100%
    #[test]
    fn test_configuration_weights_sum_to_100() {
        let total_weight = IndexingPhase::ConfigurationDiscovery.weight()
            + IndexingPhase::ConfigurationParsing.weight()
            + IndexingPhase::ConfigurationLinking.weight()
            + IndexingPhase::ConfigurationFinalizing.weight();

        assert_eq!(total_weight, 100.0);
    }

    /// Тест 4: Проверка что веса фаз платформы дают в сумме 100%
    #[test]
    fn test_platform_weights_sum_to_100() {
        let total_weight = IndexingPhase::CollectingFiles.weight()
            + IndexingPhase::ParsingFiles.weight()
            + IndexingPhase::LinkingCategories.weight()
            + IndexingPhase::BuildingIndexes.weight();

        assert_eq!(total_weight, 100.0);
    }

    /// Тест 5: Проверка граничного случая - пустая конфигурация (0 объектов)
    #[test]
    fn test_empty_configuration_progress() {
        // Когда total = 0, должны возвращать базовый процент
        let percent = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 0, 0);
        assert_eq!(percent, 5.0); // base_percentage для ConfigurationParsing

        let percent =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationDiscovery, 0, 0);
        assert_eq!(percent, 0.0); // base_percentage для ConfigurationDiscovery
    }

    /// Тест 6: Проверка граничного случая - current > total
    #[test]
    fn test_overflow_protection() {
        // Если current > total, должны ограничить максимумом = base + weight
        let percent =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 1000, 100);
        assert_eq!(percent, 85.0); // 5% (base) + 100% (clamped) * 80% (weight)

        let percent =
            ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationFinalizing, 100, 1);
        assert_eq!(percent, 100.0); // 95% + 100% (clamped) * 5% = 100%
    }

    /// Тест 7: Проверка что throttling работает (обновления каждые 5 объектов)
    #[test]
    fn test_throttling_simulation() {
        let updates = Arc::new(Mutex::new(Vec::new()));
        let updates_clone = updates.clone();

        // Симулируем throttling: отправляем обновление каждые 5 объектов
        let total_objects = 17;
        for count in 1..=total_objects {
            // В реальном коде throttling происходит в discover_metadata_in_configuration_with_progress
            // Здесь мы проверяем логику: count % 5 == 0 || count == total_objects
            if count % 5 == 0 || count == total_objects {
                let update = ProgressUpdate::new(
                    IndexingPhase::ConfigurationParsing,
                    count,
                    total_objects,
                    Some(format!("Object {}", count)),
                );
                updates_clone.lock().unwrap().push(update);
            }
        }

        let collected = updates.lock().unwrap();
        assert_eq!(collected.len(), 4); // Обновления на 5, 10, 15, 17

        // Проверяем что последнее обновление имеет корректный процент
        let last = collected.last().unwrap();
        assert_eq!(last.current, 17);
        assert_eq!(last.total, 17);
        assert!(last.percentage >= 85.0 && last.percentage <= 85.0); // Должен быть на конце фазы
    }

    /// Тест 8: Проверка что ProgressUpdate содержит корректные данные
    #[test]
    fn test_progress_update_structure() {
        let update = ProgressUpdate::new(
            IndexingPhase::ConfigurationParsing,
            42,
            100,
            Some("TestCatalog".to_string()),
        );

        assert_eq!(update.phase, IndexingPhase::ConfigurationParsing);
        assert_eq!(update.current, 42);
        assert_eq!(update.total, 100);
        assert_eq!(update.percentage, 39.0); // 5 + 42% * 80% = 5 + 33.6 = 38.6, округлено до 39.0
        assert_eq!(update.message, Some("TestCatalog".to_string()));
    }

    /// Тест 9: Проверка что отображаемые имена фаз на русском языке
    #[test]
    fn test_phase_display_names_russian() {
        assert_eq!(
            IndexingPhase::ConfigurationDiscovery.display_name(),
            "Сканирование конфигурации"
        );
        assert_eq!(
            IndexingPhase::ConfigurationParsing.display_name(),
            "Парсинг Configuration.xml"
        );
        assert_eq!(
            IndexingPhase::ConfigurationLinking.display_name(),
            "Построение связей"
        );
        assert_eq!(
            IndexingPhase::ConfigurationFinalizing.display_name(),
            "Финализация"
        );
    }

    /// Тест 10: Проверка что базовые проценты фаз конфигурации возрастают монотонно
    #[test]
    fn test_base_percentages_monotonic_increasing() {
        let base_discovery = IndexingPhase::ConfigurationDiscovery.base_percentage();
        let base_parsing = IndexingPhase::ConfigurationParsing.base_percentage();
        let base_linking = IndexingPhase::ConfigurationLinking.base_percentage();
        let base_finalizing = IndexingPhase::ConfigurationFinalizing.base_percentage();

        assert!(base_discovery < base_parsing);
        assert!(base_parsing < base_linking);
        assert!(base_linking < base_finalizing);
        assert_eq!(base_finalizing, 95.0); // Не 100%, т.к. финализация заканчивается на 100%
    }

    /// Тест 11: Проверка прогрессии всех 4 фаз конфигурации от 0% до 100%
    #[test]
    fn test_full_configuration_progress_sequence() {
        // Начало Discovery
        let p1 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationDiscovery, 0, 1);
        assert_eq!(p1, 0.0);

        // Конец Discovery
        let p2 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationDiscovery, 1, 1);
        assert_eq!(p2, 5.0);

        // Начало Parsing
        let p3 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 0, 100);
        assert_eq!(p3, 5.0);

        // Середина Parsing
        let p4 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 50, 100);
        assert_eq!(p4, 45.0);

        // Конец Parsing
        let p5 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationParsing, 100, 100);
        assert_eq!(p5, 85.0);

        // Начало Linking
        let p6 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationLinking, 0, 1);
        assert_eq!(p6, 85.0);

        // Конец Linking
        let p7 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationLinking, 1, 1);
        assert_eq!(p7, 95.0);

        // Начало Finalizing
        let p8 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationFinalizing, 0, 1);
        assert_eq!(p8, 95.0);

        // Конец Finalizing
        let p9 = ProgressUpdate::compute_percentage(IndexingPhase::ConfigurationFinalizing, 1, 1);
        assert_eq!(p9, 100.0);

        // Проверяем что прогрессия возрастает
        assert!(p1 < p2);
        assert!(p2 <= p3);
        assert!(p3 < p4);
        assert!(p4 < p5);
        assert!(p5 <= p6);
        assert!(p6 < p7);
        assert!(p7 <= p8);
        assert!(p8 < p9);
    }
}
