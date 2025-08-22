//! Бенчмарки для сравнения производительности парсеров синтакс-помощника

use bsl_gradual_types::adapters::syntax_helper_parser::{OptimizationSettings, SyntaxHelperParser};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Создаёт тестовую директорию с HTML файлами
fn create_test_directory(num_files: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test");
    fs::create_dir(&test_dir).unwrap();

    // Создаём поддиректории для разных типов
    let types_dir = test_dir.join("types");
    let methods_dir = test_dir.join("methods");
    let properties_dir = test_dir.join("properties");

    fs::create_dir(&types_dir).unwrap();
    fs::create_dir(&methods_dir).unwrap();
    fs::create_dir(&properties_dir).unwrap();

    // Создаём HTML файлы типов
    for i in 0..num_files {
        let html = generate_type_html(i);
        let file_path = types_dir.join(format!("type_{}.html", i));
        fs::write(file_path, html).unwrap();
    }

    // Создаём HTML файлы методов
    for i in 0..(num_files / 2) {
        let html = generate_method_html(i);
        let file_path = methods_dir.join(format!("method_{}.html", i));
        fs::write(file_path, html).unwrap();
    }

    // Создаём HTML файлы свойств
    for i in 0..(num_files / 4) {
        let html = generate_property_html(i);
        let file_path = properties_dir.join(format!("property_{}.html", i));
        fs::write(file_path, html).unwrap();
    }

    temp_dir
}

/// Генерирует HTML для типа
fn generate_type_html(index: usize) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>ТестовыйТип{}</title>
    <meta charset="utf-8">
</head>
<body>
    <h1 class="V8SH_pagetitle">ТестовыйТип{} (TestType{})</h1>
    <div class="V8SH_descr">
        <p>Это тестовый тип номер {}. Он используется для тестирования производительности парсера.</p>
        <p>Тип поддерживает итерацию с помощью конструкции Для каждого.</p>
    </div>
    <div class="V8SH_availability">
        <span>Доступность: Сервер, Клиент, Мобильный клиент</span>
    </div>
    <div class="V8SH_version">
        <span>Версия: 8.3.{}+</span>
    </div>
    <h2>Пример использования</h2>
    <pre class="V8SH_code">
    ТестовыйОбъект = Новый ТестовыйТип{};
    ТестовыйОбъект.Свойство1 = "Значение";
    ТестовыйОбъект.ВыполнитьМетод();
    </pre>
    <h2>См. также</h2>
    <ul>
        <li><a href="type_{}.html">СвязанныйТип{}</a></li>
        <li><a href="type_{}.html">ДругойТип{}</a></li>
    </ul>
</body>
</html>"#,
        index,
        index,
        index,
        index,
        index % 20 + 1,
        index,
        (index + 1) % 100,
        index + 1,
        (index + 2) % 100,
        index + 2
    )
}

/// Генерирует HTML для метода
fn generate_method_html(index: usize) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>ТестовыйМетод{}</title>
    <meta charset="utf-8">
</head>
<body>
    <h1 class="V8SH_pagetitle">ТестовыйМетод{}</h1>
    <span class="V8SH_english">TestMethod{}</span>
    <div class="V8SH_descr">
        <p>Выполняет тестовую операцию номер {}.</p>
    </div>
    <h2>Параметры</h2>
    <table class="V8SH_params">
        <tr>
            <th>Имя</th>
            <th>Тип</th>
            <th>Обязательность</th>
            <th>Описание</th>
        </tr>
        <tr>
            <td>Параметр1</td>
            <td>Строка</td>
            <td>Обязательный</td>
            <td>Первый параметр метода</td>
        </tr>
        <tr>
            <td>Параметр2</td>
            <td>Число</td>
            <td>Необязательный</td>
            <td>Второй параметр метода</td>
        </tr>
    </table>
    <div class="V8SH_return">
        <span>Булево: Возвращает Истина в случае успеха</span>
    </div>
</body>
</html>"#,
        index, index, index, index
    )
}

/// Генерирует HTML для свойства
fn generate_property_html(index: usize) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>ТестовоеСвойство{}</title>
    <meta charset="utf-8">
</head>
<body>
    <h1 class="V8SH_pagetitle">ТестовоеСвойство{}</h1>
    <span class="V8SH_type">Строка</span>
    <div class="V8SH_descr">
        <p>Тестовое свойство номер {}. Только чтение.</p>
    </div>
</body>
</html>"#,
        index, index, index
    )
}

/// Бенчмарк однопоточного парсера
fn bench_single_threaded(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_threaded");

    for size in [10, 50, 100, 500].iter() {
        let temp_dir = create_test_directory(*size);
        let test_path = temp_dir.path().join("test");

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Используем однопоточный режим для сравнения
                let settings = OptimizationSettings {
                    max_threads: Some(1),
                    show_progress: false,
                    parallel_indexing: false,
                    ..Default::default()
                };
                let mut parser = SyntaxHelperParser::with_settings(settings);
                parser.parse_directory(black_box(&test_path)).unwrap();
            });
        });
    }

    group.finish();
}

/// Бенчмарк многопоточного парсера с разными настройками
fn bench_multi_threaded(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_threaded");

    // Тестируем разные размеры батчей
    let batch_sizes = [10, 25, 50, 100];

    for size in [100, 500].iter() {
        let temp_dir = create_test_directory(*size);
        let test_path = temp_dir.path().join("test");

        for batch_size in batch_sizes.iter() {
            let bench_id = format!("{}_files_batch_{}", size, batch_size);

            group.bench_with_input(BenchmarkId::new("rayon", &bench_id), size, |b, _| {
                b.iter(|| {
                    let settings = OptimizationSettings {
                        batch_size: *batch_size,
                        show_progress: false,
                        parallel_indexing: true,
                        ..Default::default()
                    };

                    let mut parser = SyntaxHelperParser::with_settings(settings);
                    parser.parse_directory(black_box(&test_path)).unwrap();
                });
            });
        }
    }

    group.finish();
}

/// Бенчмарк сравнения однопоточного и многопоточного
fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");

    let temp_dir = create_test_directory(200);
    let test_path = temp_dir.path().join("test");

    // Однопоточный
    group.bench_function("single_200_files", |b| {
        b.iter(|| {
            let settings = OptimizationSettings {
                max_threads: Some(1),
                show_progress: false,
                parallel_indexing: false,
                ..Default::default()
            };
            let mut parser = SyntaxHelperParser::with_settings(settings);
            parser.parse_directory(black_box(&test_path)).unwrap();
        });
    });

    // Многопоточный с оптимальными настройками
    group.bench_function("multi_200_files_optimal", |b| {
        b.iter(|| {
            let settings = OptimizationSettings {
                batch_size: 50,
                show_progress: false,
                parallel_indexing: true,
                ..Default::default()
            };

            let mut parser = SyntaxHelperParser::with_settings(settings);
            parser.parse_directory(black_box(&test_path)).unwrap();
        });
    });

    // Многопоточный с 1 потоком (для проверки overhead)
    group.bench_function("multi_200_files_1_thread", |b| {
        b.iter(|| {
            let settings = OptimizationSettings {
                max_threads: Some(1),
                batch_size: 50,
                show_progress: false,
                parallel_indexing: false,
                ..Default::default()
            };

            let mut parser = SyntaxHelperParser::with_settings(settings);
            parser.parse_directory(black_box(&test_path)).unwrap();
        });
    });

    // Многопоточный с 4 потоками
    group.bench_function("multi_200_files_4_threads", |b| {
        b.iter(|| {
            let settings = OptimizationSettings {
                max_threads: Some(4),
                batch_size: 50,
                show_progress: false,
                parallel_indexing: true,
                ..Default::default()
            };

            let mut parser = SyntaxHelperParser::with_settings(settings);
            parser.parse_directory(black_box(&test_path)).unwrap();
        });
    });

    group.finish();
}

/// Бенчмарк построения индексов
fn bench_indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing");

    // Создаём парсер и загружаем данные
    let temp_dir = create_test_directory(500);
    let test_path = temp_dir.path().join("test");

    // Подготавливаем данные
    let settings = OptimizationSettings {
        max_threads: Some(1),
        show_progress: false,
        parallel_indexing: false,
        ..Default::default()
    };
    let mut parser_single = SyntaxHelperParser::with_settings(settings);
    parser_single.parse_directory(&test_path).unwrap();
    let database = parser_single.export_database();

    group.bench_function("sequential_indexing", |b| {
        b.iter(|| {
            let settings = OptimizationSettings {
                max_threads: Some(1),
                show_progress: false,
                parallel_indexing: false,
                ..Default::default()
            };
            let parser = SyntaxHelperParser::with_settings(settings);
            // Импортируем узлы напрямую в DashMap
            for (key, node) in &database.nodes {
                parser.nodes.insert(key.clone(), node.clone());
            }
            // Экспортируем статистику для замера
            black_box(parser.get_stats());
        });
    });

    group.bench_function("parallel_indexing", |b| {
        b.iter(|| {
            let settings = OptimizationSettings {
                show_progress: false,
                parallel_indexing: true,
                ..Default::default()
            };

            let parser = SyntaxHelperParser::with_settings(settings);
            // Импортируем узлы
            for (key, node) in &database.nodes {
                parser.nodes.insert(key.clone(), node.clone());
            }
            // Экспортируем статистику для замера
            black_box(parser.get_stats());
        });
    });

    group.finish();
}

/// Бенчмарк использования памяти
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory");
    group.sample_size(10); // Меньше замеров для memory-intensive тестов

    for size in [100, 500, 1000].iter() {
        let temp_dir = create_test_directory(*size);
        let test_path = temp_dir.path().join("test");

        group.bench_with_input(BenchmarkId::new("optimized", size), size, |b, _| {
            b.iter(|| {
                let settings = OptimizationSettings {
                    batch_size: 50,
                    show_progress: false,
                    parallel_indexing: true,
                    ..Default::default()
                };

                let mut parser = SyntaxHelperParser::with_settings(settings);
                parser.parse_directory(black_box(&test_path)).unwrap();

                // Возвращаем статистику для предотвращения оптимизации
                parser.get_stats()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_threaded,
    bench_multi_threaded,
    bench_comparison,
    bench_indexing,
    bench_memory_usage
);

criterion_main!(benches);
