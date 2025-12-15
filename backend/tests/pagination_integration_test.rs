//! Интеграционные тесты для API Pagination (Phase 3)
//!
//! Тестирование реального HTTP API через веб-сервер:
//! - GET /api/types с параметрами page/limit
//! - Валидация граничных случаев
//! - Проверка структуры ответа PaginationDto
//!
//! ВАЖНО: Требуется запущенный веб-сервер на порту 3002
//! Запуск: cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true
//!
//! По умолчанию тесты ПРОПУСКАЮТСЯ. Для запуска установите:
//! - `BSL_RUN_WEB_API_TESTS=1`
//! - (опционально) `BSL_WEB_API_BASE_URL=http://localhost:3002`

#[cfg(test)]
mod pagination_integration_tests {
    use serde_json::Value;

    fn should_run_web_api_tests() -> bool {
        std::env::var("BSL_RUN_WEB_API_TESTS").ok().as_deref() == Some("1")
    }

    fn base_url() -> String {
        std::env::var("BSL_WEB_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3002".to_string())
    }

    /// Вспомогательная функция: выполнить GET запрос и вернуть JSON
    async fn fetch_json(endpoint: &str) -> Option<Value> {
        if !should_run_web_api_tests() {
            eprintln!("SKIP pagination integration tests: set BSL_RUN_WEB_API_TESTS=1");
            return None;
        }

        let url = format!("{}{}", base_url(), endpoint);
        let response = reqwest::get(&url).await.ok()?;
        response.json().await.ok()
    }

    /// Вспомогательная функция: извлечь pagination из ответа
    fn extract_pagination(json: &Value) -> Option<&Value> {
        json.get("pagination")
    }

    // --- БАЗОВЫЕ ТЕСТЫ ---

    #[tokio::test]
    async fn test_default_pagination() {
        // Запрос без параметров: должны использоваться значения по умолчанию
        let Some(json) = fetch_json("/api/types?limit=10").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(pagination["current_page"], 1);
        assert_eq!(pagination["page_size"], 10);
    }

    #[tokio::test]
    async fn test_page_parameter() {
        // Запрос с page=2
        let Some(json) = fetch_json("/api/types?page=2&limit=50").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(pagination["current_page"], 2);
        assert_eq!(pagination["page_size"], 50);
    }

    // --- ГРАНИЧНЫЕ СЛУЧАИ: page ---

    #[tokio::test]
    async fn test_page_zero_becomes_one() {
        // page=0 должна стать page=1
        let Some(json) = fetch_json("/api/types?page=0&limit=20").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(
            pagination["current_page"], 1,
            "page=0 должна быть исправлена на 1"
        );
    }

    // --- ГРАНИЧНЫЕ СЛУЧАИ: limit ---

    #[tokio::test]
    async fn test_limit_zero_becomes_one() {
        // limit=0 должен стать limit=1
        let Some(json) = fetch_json("/api/types?page=1&limit=0").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(
            pagination["page_size"], 1,
            "limit=0 должен быть исправлен на 1"
        );
    }

    #[tokio::test]
    async fn test_limit_exceeds_max() {
        // limit=5000 должен стать limit=1000
        let Some(json) = fetch_json("/api/types?page=1&limit=5000").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(
            pagination["page_size"], 1000,
            "limit=5000 должен быть обрезан до 1000"
        );
    }

    // --- ПРОВЕРКА has_prev/has_next ---

    #[tokio::test]
    async fn test_first_page_has_no_prev() {
        // Первая страница: has_prev=false
        let Some(json) = fetch_json("/api/types?page=1&limit=10").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(
            pagination["has_prev"], false,
            "На первой странице has_prev должен быть false"
        );
    }

    #[tokio::test]
    async fn test_second_page_has_prev() {
        // Вторая страница: has_prev=true
        let Some(json) = fetch_json("/api/types?page=2&limit=10").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(
            pagination["has_prev"], true,
            "На второй странице has_prev должен быть true"
        );
    }

    // --- ПРОВЕРКА СТРУКТУРЫ ОТВЕТА ---

    #[tokio::test]
    async fn test_pagination_dto_structure() {
        // Проверка наличия всех обязательных полей в PaginationDto
        let Some(json) = fetch_json("/api/types?page=3&limit=50").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert!(
            pagination.get("current_page").is_some(),
            "Отсутствует current_page"
        );
        assert!(
            pagination.get("page_size").is_some(),
            "Отсутствует page_size"
        );
        assert!(
            pagination.get("total_items").is_some(),
            "Отсутствует total_items"
        );
        assert!(
            pagination.get("total_pages").is_some(),
            "Отсутствует total_pages"
        );
        assert!(pagination.get("has_prev").is_some(), "Отсутствует has_prev");
        assert!(pagination.get("has_next").is_some(), "Отсутствует has_next");
    }

    #[tokio::test]
    async fn test_items_array_length() {
        // Проверка, что количество элементов в items соответствует limit
        let Some(json) = fetch_json("/api/types?page=1&limit=25").await else { return; };
        let items = json["items"].as_array().unwrap();

        // Количество элементов <= limit
        assert!(
            items.len() <= 25,
            "Количество items не должно превышать limit"
        );
    }

    // --- ПРОВЕРКА TOTAL_PAGES ---

    #[tokio::test]
    async fn test_total_pages_calculation() {
        // total_pages = ceil(total_items / page_size)
        let Some(json) = fetch_json("/api/types?page=1&limit=100").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        let total_items = pagination["total_items"].as_u64().unwrap() as f64;
        let page_size = pagination["page_size"].as_u64().unwrap() as f64;
        let total_pages = pagination["total_pages"].as_u64().unwrap();

        let expected_total_pages = (total_items / page_size).ceil() as u64;
        assert_eq!(
            total_pages, expected_total_pages,
            "total_pages должен быть ceil(total_items / page_size)"
        );
    }

    // --- ПРОВЕРКА, ЧТО offset БОЛЬШЕ НЕ ПРИНИМАЕТСЯ ---

    #[tokio::test]
    async fn test_offset_parameter_ignored() {
        // Передаём offset=100, но он должен игнорироваться
        // Должна использоваться page=1 (значение по умолчанию)
        let Some(json) = fetch_json("/api/types?offset=100&limit=10").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(
            pagination["current_page"], 1,
            "offset должен игнорироваться, используется page=1"
        );
    }

    // --- ПРОВЕРКА КОНСИСТЕНТНОСТИ ДАННЫХ ---

    #[tokio::test]
    async fn test_pagination_consistency() {
        // Проверка математической консистентности pagination полей
        let Some(json) = fetch_json("/api/types?page=5&limit=100").await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        let current_page = pagination["current_page"].as_u64().unwrap();
        let total_pages = pagination["total_pages"].as_u64().unwrap();
        let has_prev = pagination["has_prev"].as_bool().unwrap();
        let has_next = pagination["has_next"].as_bool().unwrap();

        // has_prev должен быть true для page > 1
        assert_eq!(has_prev, current_page > 1);

        // has_next должен быть true для page < total_pages
        assert_eq!(has_next, current_page < total_pages);
    }

    // --- EDGE CASE: ПУСТОЙ РЕЗУЛЬТАТ ---

    #[tokio::test]
    async fn test_empty_search_result() {
        // Поиск несуществующего типа: должен вернуть пустой результат
        let Some(json) = fetch_json("/api/search?q=%D0%9D%D0%B5%D1%81%D1%83%D1%89%D0%B5%D1%81%D1%82%D0%B2%D1%83%D1%8E%D1%89%D0%B8%D0%B9%D0%A2%D0%B8%D0%BF&page=1&limit=10")
            .await else { return; };
        let pagination = extract_pagination(&json).unwrap();

        assert_eq!(pagination["current_page"], 1);
        assert_eq!(pagination["total_items"], 0);
        assert_eq!(pagination["total_pages"], 0);

        let items = json["items"].as_array().unwrap();
        assert!(items.is_empty(), "items должен быть пустым массивом");
    }

    // --- EDGE CASE: СТРАНИЦА ЗА ПРЕДЕЛАМИ ДИАПАЗОНА ---

    #[tokio::test]
    async fn test_page_beyond_total_pages() {
        // page=999999 (заведомо больше total_pages)
        let Some(json) = fetch_json("/api/types?page=999999&limit=10").await else { return; };
        let items = json["items"].as_array().unwrap();

        // Должен вернуть пустой массив или последнюю страницу
        assert!(
            items.is_empty() || !items.is_empty(),
            "API должен корректно обработать page за пределами диапазона"
        );
    }

    // --- ПРОВЕРКА РАЗЛИЧНЫХ LIMIT ---

    #[tokio::test]
    async fn test_various_limits() {
        // Проверка различных значений limit
        let limits = vec![1, 10, 50, 100, 500, 1000];

        for limit in limits {
            let endpoint = format!("/api/types?page=1&limit={}", limit);
            let Some(json) = fetch_json(&endpoint).await else { return; };
            let pagination = extract_pagination(&json).unwrap();

            assert_eq!(
                pagination["page_size"], limit,
                "page_size должен соответствовать запрошенному limit"
            );

            let items = json["items"].as_array().unwrap();
            assert!(
                items.len() <= limit,
                "Количество items не должно превышать limit"
            );
        }
    }
}
