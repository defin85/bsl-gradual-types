//! Интеграционный тест: Проверка последовательности инициализации LSP Server
//!
//! Эмулирует точную последовательность вызовов из lsp_server.rs:
//! 1. main() → coordinator.start() → создаёт TypeRepository с базовыми типами (4 типа)
//! 2. initialized() → coordinator.start_with_paths() → создаёт НОВЫЙ TypeRepository (3927 типов)
//! 3. Проверяем, что TypeSystemService использует ОБНОВЛЁННЫЙ репозиторий

use bsl_backend::system::SystemCoordinator;
use std::path::Path;

#[tokio::test]
async fn test_lsp_server_init_sequence_with_double_start() {
    let coordinator = SystemCoordinator::new();

    // ✅ ШАГ 1: Эмулируем вызов из main() → coordinator.start()
    // Создаёт TypeRepository с fallback типами платформы (включая коллекции вроде Массив),
    // но без методов/документации из Syntax Helper.
    println!("📍 ШАГ 1: coordinator.start() - базовые типы");
    coordinator
        .start()
        .await
        .expect("Failed to start coordinator with basic types");

    // Получаем TypeSystemService после первой инициализации
    let type_service_v1 = coordinator
        .type_service()
        .expect("TypeSystemService not available after first start");

    // Проверяем, что в этот момент у нас только базовые типы
    println!("📍 Проверяем состояние после start()...");
    let source_basic = r#"
Функция Тест()
    Перем МассивДанных;
    МассивДанных = Новый Массив;
    Возврат МассивДанных;
КонецФункции
"#;

    let hover_v1 = type_service_v1
        .get_hover_info(source_basic, 3, 10, None)
        .await
        .expect("get_hover_info failed");

    // На этом этапе Массив ДОЛЖЕН быть найден (fallback types),
    // но методы должны быть недоступны без Syntax Helper.
    if let Some(hover_text) = hover_v1 {
        println!("🔍 Hover после start(): {}", hover_text);
        assert!(
            hover_text.contains("**Тип:**") && hover_text.contains("Массив"),
            "После start() Массив должен резолвиться из fallback types. Actual:\n{}",
            hover_text
        );
        assert!(
            hover_text.contains("Методы недоступны") || hover_text.contains("Детали типа недоступны"),
            "После start() должны быть недоступны методы/документация без Syntax Helper. Actual:\n{}",
            hover_text
        );
    } else {
        panic!("Hover не вернул информацию после start()");
    }

    println!("✅ ШАГ 1 завершён: Массив найден через fallback (ожидаемо)");

    // ✅ ШАГ 2: Эмулируем вызов из initialized() → coordinator.start_with_paths()
    // Создаёт НОВЫЙ TypeRepository с полными типами из Syntax Helper (3927 типов)
    println!("\n📍 ШАГ 2: coordinator.start_with_paths() - полные типы из Syntax Helper");
    // Используем абсолютный путь от корня workspace (где находится Cargo.toml)
    let syntax_path = Path::new("../examples/syntax_helper");

    // DEBUG: Проверяем, существует ли путь
    if syntax_path.exists() {
        println!(
            "✅ Путь к Syntax Helper существует: {}",
            syntax_path.display()
        );
    } else {
        println!(
            "❌ Путь к Syntax Helper НЕ существует: {}",
            syntax_path.display()
        );
        println!("   Current dir: {:?}", std::env::current_dir());
        panic!("Syntax Helper path not found!");
    }

    coordinator
        .start_with_paths(Some(syntax_path), None, None)
        .await
        .expect("Failed to reload types from Syntax Helper");

    println!("✅ ШАГ 2 завершён: Типы перезагружены");

    // DEBUG: Проверяем, сколько типов загружено после reload через TypeRepository
    let analysis_engine = coordinator
        .get_analysis_engine()
        .expect("AnalysisEngine не доступен после reload");
    let repository = analysis_engine.get_repository();
    let stats = repository.get_stats();
    println!("📊 TypeRepository stats после reload: {:?}", stats);

    // Проверим, резолвится ли тип "Массив"
    let массив_resolution = analysis_engine.resolve_type("Массив");
    println!(
        "🔍 Resolution для 'Массив': certainty={:?}, result={:?}",
        массив_resolution.certainty, массив_resolution.result
    );

    use bsl_shared::domain::types::Certainty;
    if !matches!(массив_resolution.certainty, Certainty::Unknown) {
        println!("✅ Тип 'Массив' найден в TypeRepository!");
    } else {
        println!("❌ Тип 'Массив' НЕ НАЙДЕН в TypeRepository после reload!");
    }

    // ✅ ШАГ 3: НЕ ИСПОЛЬЗУЕМ СТАРЫЙ type_service_v1!
    // Вместо этого, получаем НОВЫЙ TypeSystemService ПОСЛЕ перезагрузки типов
    // SystemCoordinator.start_with_paths() очистил кеш type_service_cache,
    // поэтому type_service() создаст НОВЫЙ экземпляр с ОБНОВЛЁННЫМ AnalysisEngine
    println!("\n📍 ШАГ 3: Проверяем, что TypeSystemService использует ОБНОВЛЁННЫЙ репозиторий");
    let type_service_v2 = coordinator
        .type_service()
        .expect("TypeSystemService not available after reload");

    // ⚠️ ВАЖНО: НЕ ИСПОЛЬЗУЕМ type_service_v1! Это старый экземпляр с пустым репозиторием!

    // Проверяем, что теперь Массив НАЙДЕН (из полного Syntax Helper)
    let source_reloaded = r#"
Функция Тест()
    Перем МассивДанных;
    МассивДанных = Новый Массив;
    Возврат МассивДанных;
КонецФункции
"#;

    let hover_v2 = type_service_v2
        .get_hover_info(source_reloaded, 3, 10, None)
        .await
        .expect("get_hover_info failed after reload");

    if let Some(hover_text) = hover_v2 {
        println!("🔍 Hover после start_with_paths(): {}", hover_text);

        // ✅ КРИТИЧЕСКАЯ ПРОВЕРКА: Массив ДОЛЖЕН быть найден!
        assert!(
            hover_text.contains("Массив") || hover_text.contains("Array"),
            "После start_with_paths() Массив ДОЛЖЕН быть найден! Actual:\n{}",
            hover_text
        );

        // ❌ НЕ ДОЛЖНО быть предупреждения "Тип не найден"
        assert!(
            !hover_text.contains("Тип не найден в TypeRepository"),
            "После start_with_paths() НЕ ДОЛЖНО быть предупреждения о несуществующем типе! Actual:\n{}",
            hover_text
        );

        // ✅ ДОЛЖНЫ быть методы типа Массив
        assert!(
            hover_text.contains("Добавить") || hover_text.contains("Количество"),
            "Hover ДОЛЖЕН показывать методы типа Массив. Actual:\n{}",
            hover_text
        );

        println!("✅ УСПЕХ: Массив найден с корректными методами!");
    } else {
        panic!("Hover не вернул информацию после reload!");
    }

    println!("\n✅ ВСЕ ШАГИ ПРОЙДЕНЫ: LSP init sequence работает корректно!");
}
