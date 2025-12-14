//! Тест для проверки правильности certainty уровней после исправления в TypeResolver
//!
//! Проверяет, что конфигурационные типы БЕЗ метаданных показывают InferredWeak (50%) вместо Inferred (80%)

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::SystemCoordinator;
use std::sync::Arc;

/// Setup функция для создания TypeSystemService с загруженными типами платформы
async fn setup_type_system_service() -> Arc<TypeSystemService> {
    let coordinator = SystemCoordinator::new();
    coordinator
        .start()
        .await
        .expect("Failed to start SystemCoordinator");
    coordinator
        .type_service()
        .expect("TypeSystemService should be initialized after start()")
}

#[tokio::test]
async fn test_config_types_without_metadata_show_inferred_50() {
    // Setup
    let service = setup_type_system_service().await;

    // BSL код с конфигурационными типами
    let source = r#"Процедура Тест()
    Справочник1 = Справочники.Контрагенты;
    Документ1 = Документы.РеализацияТоваровУслуг;
    Перечисление1 = Перечисления.СтавкиНДС;
    РегистрСведений1 = РегистрыСведений.КурсыВалют;
КонецПроцедуры"#;

    println!("=== Проверка certainty для конфигурационных типов БЕЗ метаданных ===");

    // Тест 1: Справочники.Контрагенты
    // Строка 1 (Справочник1 = ...), колонка 5 (начало имени переменной после 4 пробелов отступа)
    println!("\n--- Тест 1: Справочники.Контрагенты ---");
    let hover_справочник = service
        .get_hover_info(source, 1, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_справочник {
        println!("Hover text:\n{}", hover_text);

        // ✅ R-3: InferredWeak (50%) вместо Inferred(0.5)
        assert!(
            hover_text.contains("🟠 InferredWeak (50%)"),
            "Expected '🟠 InferredWeak (50%)' for Справочники.Контрагенты, got:\n{}",
            hover_text
        );

        // НЕ должно быть старого значения
        assert!(
            !hover_text.contains("80%"),
            "Hover should NOT contain '80%' for config types without metadata, got:\n{}",
            hover_text
        );

        println!("✅ Справочник1: InferredWeak (50%) - ПРАВИЛЬНО");
    } else {
        panic!("Hover для Справочник1 не вернул информацию");
    }

    // Тест 2: Документы.РеализацияТоваровУслуг
    println!("\n--- Тест 2: Документы.РеализацияТоваровУслуг ---");
    let hover_документ = service
        .get_hover_info(source, 2, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_документ {
        println!("Hover text:\n{}", hover_text);

        assert!(
            hover_text.contains("🟠 InferredWeak (50%)"),
            "Expected '🟠 InferredWeak (50%)' for Документы.РеализацияТоваровУслуг, got:\n{}",
            hover_text
        );

        assert!(
            !hover_text.contains("80%"),
            "Hover should NOT contain '80%', got:\n{}",
            hover_text
        );

        println!("✅ Документ1: InferredWeak (50%) - ПРАВИЛЬНО");
    }

    // Тест 3: Перечисления.СтавкиНДС
    println!("\n--- Тест 3: Перечисления.СтавкиНДС ---");
    let hover_перечисление = service
        .get_hover_info(source, 3, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_перечисление {
        println!("Hover text:\n{}", hover_text);

        assert!(
            hover_text.contains("🟠 InferredWeak (50%)"),
            "Expected '🟠 InferredWeak (50%)' for Перечисления.СтавкиНДС, got:\n{}",
            hover_text
        );

        assert!(
            !hover_text.contains("80%"),
            "Hover should NOT contain '80%', got:\n{}",
            hover_text
        );

        println!("✅ Перечисление1: InferredWeak (50%) - ПРАВИЛЬНО");
    }

    // Тест 4: РегистрыСведений.КурсыВалют
    println!("\n--- Тест 4: РегистрыСведений.КурсыВалют ---");
    let hover_регистр = service
        .get_hover_info(source, 4, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_регистр {
        println!("Hover text:\n{}", hover_text);

        assert!(
            hover_text.contains("🟠 InferredWeak (50%)"),
            "Expected '🟠 InferredWeak (50%)' for РегистрыСведений.КурсыВалют, got:\n{}",
            hover_text
        );

        assert!(
            !hover_text.contains("80%"),
            "Hover should NOT contain '80%', got:\n{}",
            hover_text
        );

        println!("✅ РегистрСведений1: InferredWeak (50%) - ПРАВИЛЬНО");
    }

    println!("\n✅ ВСЕ ПРОВЕРКИ ПРОШЛИ: Конфигурационные типы без метаданных → InferredWeak (50%)");
}

#[tokio::test]
async fn test_platform_types_still_show_known() {
    // Setup
    let service = setup_type_system_service().await;

    // BSL код с платформенными типами
    let source = r#"Процедура Тест()
    СтрокаДанных = "Текст";
    ЧислоДанных = 42;
    БулевоДанных = Истина;
КонецПроцедуры"#;

    println!("=== Регрессия: платформенные типы должны быть Known (100%) ===");

    // Тест 1: Строка
    println!("\n--- Тест 1: Строка ---");
    let hover_строка = service
        .get_hover_info(source, 1, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_строка {
        println!("Hover text:\n{}", hover_text);

        assert!(
            hover_text.contains("🟢 Known (100%)"),
            "Expected '🟢 Known (100%)' for Строка, got:\n{}",
            hover_text
        );

        println!("✅ Строка: Known (100%) - ПРАВИЛЬНО");
    }

    // Тест 2: Число
    println!("\n--- Тест 2: Число ---");
    let hover_число = service
        .get_hover_info(source, 2, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_число {
        println!("Hover text:\n{}", hover_text);

        assert!(
            hover_text.contains("🟢 Known (100%)"),
            "Expected '🟢 Known (100%)' for Число, got:\n{}",
            hover_text
        );

        println!("✅ Число: Known (100%) - ПРАВИЛЬНО");
    }

    // Тест 3: Булево
    println!("\n--- Тест 3: Булево ---");
    let hover_булево = service
        .get_hover_info(source, 3, 5, None)
        .await
        .expect("Hover failed");

    if let Some(hover_text) = hover_булево {
        println!("Hover text:\n{}", hover_text);

        assert!(
            hover_text.contains("🟢 Known (100%)"),
            "Expected '🟢 Known (100%)' for Булево, got:\n{}",
            hover_text
        );

        println!("✅ Булево: Known (100%) - ПРАВИЛЬНО");
    }

    println!("\n✅ РЕГРЕССИЯ ПРОШЛА: Платформенные типы → Known (100%)");
}

#[tokio::test]
async fn test_resolver_direct_certainty_check() {
    // Прямая проверка TypeResolver для конфигурационных типов
    let service = setup_type_system_service().await;

    println!("=== Прямая проверка TypeResolver.resolve_expression_sync() ===");

    // Получаем AnalysisEngine из сервиса (через рефлексию невозможно, используем hover)
    // АЛЬТЕРНАТИВА: проверим через hover, что внутри вызывается правильный resolver

    let source = r#"Процедура Тест()
    Тип1 = Справочники.Контрагенты;
КонецПроцедуры"#;

    let hover = service
        .get_hover_info(source, 1, 5, None)
        .await
        .expect("Hover failed")
        .unwrap();

    println!("Hover для Справочники.Контрагенты:");
    println!("{}", hover);

    // Детальная проверка - R-3: InferredWeak вместо Inferred(0.5)
    assert!(
        hover.contains("InferredWeak (50%)"),
        "Certainty должен быть InferredWeak (50%)"
    );
    assert!(
        hover.contains("Справочник"),
        "Должен упоминаться Справочник"
    );

    println!("\n✅ TypeResolver возвращает InferredWeak (50%) для конфигурационных типов");
}
