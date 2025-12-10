//! Парсер синтакс-помощника 1С для извлечения информации о типах платформы
//!
//! Единственная актуальная версия парсера с поддержкой:
//! - Многопоточной обработки через rayon
//! - Lock-free структур данных через DashMap
//! - Полной информации о типах, методах, свойствах
//! - Двуязычности (русский/английский)
//! - Построения индексов для быстрого поиска

mod handlers;
mod parser;
mod types;

// Публичные реэкспорты
pub use parser::SyntaxHelperParser;
pub use types::ParsingStats;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::loaders::progress::{IndexingPhase, ProgressUpdate};
    use crate::data::loaders::syntax_helper::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_parsing() {
        // Создаём временную директорию с тестовыми HTML файлами
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();

        // Создаём несколько тестовых HTML файлов
        for i in 0..10 {
            let html = format!(
                r#"
                <html>
                <body>
                    <h1 class="V8SH_pagetitle">TestType{} (TestType{})</h1>
                    <p>Test description {}</p>
                </body>
                </html>
            "#,
                i, i, i
            );

            let file_path = test_dir.join(format!("type_{}.html", i));
            fs::write(file_path, html).unwrap();
        }

        // Парсим с многопоточностью
        let settings = OptimizationSettings {
            max_threads: Some(4),
            batch_size: 2,
            show_progress: false,
            ..Default::default()
        };

        let mut parser = SyntaxHelperParser::with_settings(settings);
        parser
            .parse_directory(&test_dir, None::<fn(ProgressUpdate)>)
            .unwrap();

        // Проверяем результаты
        let stats = parser.get_stats();
        assert_eq!(stats.processed_files, 10);
        assert_eq!(stats.types_count, 10);
        assert_eq!(stats.error_count, 0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let parser = Arc::new(SyntaxHelperParser::new());
        let mut handles = vec![];

        // Создаём несколько потоков для одновременного доступа
        for i in 0..10 {
            let parser_clone = Arc::clone(&parser);
            let handle = thread::spawn(move || {
                // Симулируем сохранение узла
                let type_info = TypeInfo {
                    identity: TypeIdentity {
                        russian_name: format!("Тип{}", i),
                        english_name: format!("Type{}", i),
                        catalog_path: format!("path_{}", i),
                        category_path: String::new(),
                        aliases: Vec::new(),
                    },
                    documentation: TypeDocumentation {
                        category_description: None,
                        type_description: format!("Description {}", i),
                        examples: Vec::new(),
                        availability: vec!["Сервер".to_string()],
                        since_version: "8.3.0".to_string(),
                    },
                    structure: TypeStructure {
                        collection_element: None,
                        methods: Vec::new(),
                        properties: Vec::new(),
                        constructors: Vec::new(),
                        iterable: false,
                        indexable: false,
                        enum_values: Vec::new(),
                    },
                    metadata: TypeMetadata {
                        available_facets: vec![],
                        default_facet: None,
                        serializable: true,
                        exchangeable: true,
                        xdto_namespace: None,
                        xdto_type: None,
                    },
                };

                parser_clone.save_node(SyntaxNode::Type(type_info));
            });

            handles.push(handle);
        }

        // Ждём завершения всех потоков
        for handle in handles {
            handle.join().unwrap();
        }

        // Проверяем, что все узлы сохранены
        assert_eq!(parser.nodes.len(), 10);
    }

    #[test]
    fn test_parse_with_progress_callback() {
        use std::sync::Mutex;

        // Создаём небольшую тестовую директорию
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();

        // Создаём несколько HTML файлов для теста
        for i in 0..15 {
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

        // Собираем прогресс в вектор
        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = progress_updates.clone();

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
        parser.parse_directory(&test_dir, Some(callback)).unwrap();

        // Проверяем что прогресс был отправлен
        let updates = progress_updates.lock().unwrap();

        // Должны быть обновления для всех 4 фаз
        let phases: Vec<IndexingPhase> = updates.iter().map(|u| u.phase).collect();
        assert!(
            phases.contains(&IndexingPhase::CollectingFiles),
            "Нет фазы CollectingFiles"
        );
        assert!(
            phases.contains(&IndexingPhase::ParsingFiles),
            "Нет фазы ParsingFiles"
        );
        assert!(
            phases.contains(&IndexingPhase::LinkingCategories),
            "Нет фазы LinkingCategories"
        );
        assert!(
            phases.contains(&IndexingPhase::BuildingIndexes),
            "Нет фазы BuildingIndexes"
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

    #[test]
    fn test_parse_without_callback_still_works() {
        // Проверяем что парсинг без callback продолжает работать
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test");
        fs::create_dir(&test_dir).unwrap();

        // Создаём тестовый файл
        let html = r#"
            <html>
            <head><title>TestType (TestType)</title></head>
            <body>
                <h1 class="V8SH_pagetitle">TestType (TestType)</h1>
                <p>Test description</p>
            </body>
            </html>
        "#;
        fs::write(test_dir.join("test.html"), html).unwrap();

        // Парсим БЕЗ callback (старый API)
        let settings = OptimizationSettings {
            show_progress: false,
            ..Default::default()
        };

        let mut parser = SyntaxHelperParser::with_settings(settings);
        let result = parser.parse_directory(&test_dir, None::<fn(ProgressUpdate)>);

        // Должно работать без ошибок
        assert!(result.is_ok(), "Парсинг без callback должен работать");

        // Проверяем что файл обработан
        let stats = parser.get_stats();
        assert_eq!(stats.processed_files, 1, "Должен быть обработан 1 файл");
    }
}
