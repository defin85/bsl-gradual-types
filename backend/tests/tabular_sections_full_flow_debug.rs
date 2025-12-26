//! DEBUG: Полная цепочка от парсинга до API

#[cfg(test)]
mod full_flow_tests {
    use bsl_backend::system::system_coordinator::SystemCoordinator;
    use std::path::Path;

    #[tokio::test]
    async fn test_full_flow_zakaznarjady() {
        // 1. Создаём координатор
        let coordinator = SystemCoordinator::new();

        // 2. Запускаем с конфигурацией
        let config_path = Path::new("../examples/conf/conf_test");
        coordinator
            .start_with_paths(None, Some(config_path), Some("8.3.25"), None)
            .await
            .expect("Failed to start coordinator");

        println!("=== COORDINATOR STARTED ===\n");

        // 3. Получаем TypeSystemService
        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        // 4. Получаем ПРЯМО из репозитория
        let engine = coordinator.analysis_engine().expect("No analysis engine");

        let repository = engine.get_repository();

        println!("=== CHECKING REPOSITORY ===");

        // Показать все типы в репозитории
        let all_types = repository.get_all_types();
        println!("Всего типов в репозитории: {}", all_types.len());

        // Показать документы
        let documents: Vec<_> = all_types
            .iter()
            .filter(|t| t.name.starts_with("Документы"))
            .collect();
        println!("Документов: {}", documents.len());
        for doc in documents.iter().take(5) {
            println!("  - {} (ТЧ: {})", doc.name, doc.tabular_sections.len());
        }

        let raw_type_opt = repository.find_type("Документы.ЗаказНаряды");

        if let Some(raw_type) = &raw_type_opt {
            println!("\n✅ Найден в репозитории: {}", raw_type.name);
            println!("   Атрибутов: {}", raw_type.attributes.len());
            println!("   Табличных частей: {}", raw_type.tabular_sections.len());

            for ts in &raw_type.tabular_sections {
                println!(
                    "     - ТЧ '{}' (атрибутов: {})",
                    ts.name,
                    ts.attributes.len()
                );
            }
        } else {
            println!("\n❌ НЕ НАЙДЕН в репозитории: Документы.ЗаказНаряды");
        }

        println!("\n=== SEARCHING VIA API ===");

        // 5. Ищем через API
        let result = service
            .search_types_as_dto("Документы.ЗаказНаряды")
            .await
            .expect("Failed to search types");

        assert!(
            !result.types.is_empty(),
            "Should find документы.ЗаказНаряды"
        );

        let doc = &result.types[0];

        println!("✅ Найден через API: {}", doc.name);
        println!("   Attributes count: {:?}", doc.attributes_count);
        println!("   Tabular sections count: {}", doc.tabular_sections.len());

        for ts in &doc.tabular_sections {
            println!(
                "     - ТЧ '{}' (атрибутов: {})",
                ts.name,
                ts.attributes.len()
            );
        }

        // ASSERTIONS
        assert!(raw_type_opt.is_some(), "Тип должен быть в репозитории");

        let raw_type = raw_type_opt.unwrap();
        assert_eq!(
            raw_type.tabular_sections.len(),
            2,
            "В репозитории должно быть 2 ТЧ"
        );

        assert_eq!(
            doc.tabular_sections.len(),
            2,
            "DTO должно содержать 2 ТЧ, найдено: {}",
            doc.tabular_sections.len()
        );
    }
}
