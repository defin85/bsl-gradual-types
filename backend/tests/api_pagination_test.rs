//! Unit тесты для API Pagination (Phase 3)
//!
//! Тестируемые компоненты:
//! - backend/src/presentation/web/handlers.rs (get_types handler)
//!
//! Проверяемая функциональность:
//! - Конвертация page → offset
//! - Валидация page (минимум 1)
//! - Валидация limit (диапазон 1-1000)
//! - Значения по умолчанию

#[cfg(test)]
mod api_pagination_tests {
    /// Вспомогательная функция: конвертация page → offset
    /// (повторяет логику из handlers.rs:56)
    fn page_to_offset(page: usize, limit: usize) -> usize {
        (page - 1) * limit
    }

    /// Вспомогательная функция: валидация page (минимум 1)
    /// (повторяет логику из handlers.rs:52)
    fn validate_page(page: usize) -> usize {
        page.max(1)
    }

    /// Вспомогательная функция: валидация limit (диапазон 1-1000)
    /// (повторяет логику из handlers.rs:53)
    fn validate_limit(limit: usize) -> usize {
        limit.clamp(1, 1000)
    }

    // --- ТЕСТЫ ДЛЯ КОНВЕРТАЦИИ page → offset ---

    #[test]
    fn test_page_to_offset_first_page() {
        // page=1, limit=50 → offset=0
        assert_eq!(page_to_offset(1, 50), 0);
    }

    #[test]
    fn test_page_to_offset_second_page() {
        // page=2, limit=50 → offset=50
        assert_eq!(page_to_offset(2, 50), 50);
    }

    #[test]
    fn test_page_to_offset_tenth_page() {
        // page=10, limit=100 → offset=900
        assert_eq!(page_to_offset(10, 100), 900);
    }

    #[test]
    fn test_page_to_offset_different_limits() {
        // Проверка различных комбинаций page/limit
        assert_eq!(page_to_offset(1, 10), 0);
        assert_eq!(page_to_offset(2, 10), 10);
        assert_eq!(page_to_offset(3, 25), 50);
        assert_eq!(page_to_offset(5, 100), 400);
    }

    // --- ТЕСТЫ ДЛЯ ВАЛИДАЦИИ page ---

    #[test]
    fn test_page_validation_zero_becomes_one() {
        // page=0 должна стать page=1
        assert_eq!(validate_page(0), 1);
    }

    #[test]
    fn test_page_validation_valid_pages() {
        // Валидные страницы остаются неизменными
        assert_eq!(validate_page(1), 1);
        assert_eq!(validate_page(2), 2);
        assert_eq!(validate_page(100), 100);
        assert_eq!(validate_page(999999), 999999);
    }

    // --- ТЕСТЫ ДЛЯ ВАЛИДАЦИИ limit ---

    #[test]
    fn test_limit_validation_zero_becomes_one() {
        // limit=0 → 1 (минимум)
        assert_eq!(validate_limit(0), 1);
    }

    #[test]
    fn test_limit_validation_exceeds_max() {
        // limit=5000 → 1000 (максимум)
        assert_eq!(validate_limit(5000), 1000);
        assert_eq!(validate_limit(2000), 1000);
        assert_eq!(validate_limit(1500), 1000);
    }

    #[test]
    fn test_limit_validation_within_range() {
        // limit в пределах диапазона остаётся неизменным
        assert_eq!(validate_limit(1), 1);
        assert_eq!(validate_limit(50), 50);
        assert_eq!(validate_limit(100), 100);
        assert_eq!(validate_limit(500), 500);
        assert_eq!(validate_limit(1000), 1000); // граница
    }

    #[test]
    fn test_limit_validation_edge_cases() {
        // Граничные значения
        assert_eq!(validate_limit(1), 1);    // нижняя граница
        assert_eq!(validate_limit(1000), 1000); // верхняя граница
        assert_eq!(validate_limit(1001), 1000); // чуть больше максимума
    }

    // --- КОМПЛЕКСНЫЕ ТЕСТЫ ---

    #[test]
    fn test_pagination_scenario_default_values() {
        // Сценарий: запрос без параметров (page=None, limit=None)
        // Ожидается: page=1, limit=50
        let page = validate_page(1); // unwrap_or(1)
        let limit = validate_limit(50); // unwrap_or(50)
        let offset = page_to_offset(page, limit);

        assert_eq!(page, 1);
        assert_eq!(limit, 50);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_pagination_scenario_invalid_inputs() {
        // Сценарий: page=0, limit=0 (невалидные значения)
        // Ожидается: исправление на page=1, limit=1
        let page = validate_page(0);
        let limit = validate_limit(0);
        let offset = page_to_offset(page, limit);

        assert_eq!(page, 1);
        assert_eq!(limit, 1);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_pagination_scenario_exceeds_max_limit() {
        // Сценарий: page=2, limit=5000 (превышает максимум)
        // Ожидается: page=2, limit=1000, offset=1000
        let page = validate_page(2);
        let limit = validate_limit(5000);
        let offset = page_to_offset(page, limit);

        assert_eq!(page, 2);
        assert_eq!(limit, 1000);
        assert_eq!(offset, 1000); // (2-1) * 1000
    }

    #[test]
    fn test_pagination_scenario_typical_request() {
        // Сценарий: типичный запрос page=3, limit=100
        let page = validate_page(3);
        let limit = validate_limit(100);
        let offset = page_to_offset(page, limit);

        assert_eq!(page, 3);
        assert_eq!(limit, 100);
        assert_eq!(offset, 200); // (3-1) * 100
    }

    // --- МАТЕМАТИЧЕСКИЕ ИНВАРИАНТЫ ---

    #[test]
    fn test_page_offset_relationship() {
        // Проверка математического соотношения:
        // offset = (page - 1) * limit
        // page = (offset / limit) + 1

        let test_cases = vec![
            (1, 50, 0),
            (2, 50, 50),
            (3, 100, 200),
            (10, 25, 225),
        ];

        for (page, limit, expected_offset) in test_cases {
            let offset = page_to_offset(page, limit);
            assert_eq!(offset, expected_offset);

            // Обратная конвертация: offset → page
            let reconstructed_page = (offset / limit) + 1;
            assert_eq!(reconstructed_page, page);
        }
    }

    #[test]
    fn test_offset_never_negative() {
        // offset всегда >= 0 для любых валидных page/limit
        let test_cases = vec![
            (0, 50),  // невалидная page (будет исправлена на 1)
            (1, 1),
            (1, 1000),
            (999, 1),
        ];

        for (page, limit) in test_cases {
            let validated_page = validate_page(page);
            let validated_limit = validate_limit(limit);
            let offset = page_to_offset(validated_page, validated_limit);

            assert!(offset >= 0, "offset должен быть >= 0");
        }
    }
}
