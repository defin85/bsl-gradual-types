//! Тест для демонстрации многопоточной работы парсера синтакс-помощника

use bsl_gradual_types::data::loaders::syntax_helper_parser::{
    OptimizationSettings, SyntaxHelperParser,
};
use std::fs;
use std::time::Instant;
use tempfile::TempDir;

fn main() -> anyhow::Result<()> {
    println!("=== Демонстрация многопоточной работы парсера синтакс-помощника ===\n");

    // Создаём тестовые HTML файлы
    let temp_dir = create_test_files(1000)?;
    let test_path = temp_dir.path().join("test");

    println!("📊 Система:");
    println!("   - Создано тестовых файлов: 1000");
    println!("   - Доступно ядер процессора: {}", num_cpus::get());
    println!(
        "   - Потоков rayon по умолчанию: {}",
        rayon::current_num_threads()
    );

    // Тест 1: Однопоточный режим
    println!("\n🚶 Тест 1: Однопоточный режим");
    let start = Instant::now();
    let settings_single = OptimizationSettings {
        max_threads: Some(1),
        batch_size: 10,
        show_progress: false,
        parallel_indexing: false,
        ..Default::default()
    };
    let mut parser_single = SyntaxHelperParser::with_settings(settings_single);
    parser_single.parse_directory(&test_path)?;
    let single_time = start.elapsed();
    let stats_single = parser_single.get_stats();
    println!("   ⏱️ Время выполнения: {:?}", single_time);
    println!("   📁 Обработано файлов: {}", stats_single.processed_files);
    println!("   🔧 Количество потоков: 1");

    // Тест 2: Многопоточный режим (2 потока)
    println!("\n🏃 Тест 2: Многопоточный режим (2 потока)");
    let start = Instant::now();
    let settings_multi2 = OptimizationSettings {
        max_threads: Some(2),
        batch_size: 10,
        show_progress: false,
        parallel_indexing: true,
        ..Default::default()
    };
    let mut parser_multi2 = SyntaxHelperParser::with_settings(settings_multi2);
    parser_multi2.parse_directory(&test_path)?;
    let multi2_time = start.elapsed();
    let stats_multi2 = parser_multi2.get_stats();
    println!("   ⏱️ Время выполнения: {:?}", multi2_time);
    println!("   📁 Обработано файлов: {}", stats_multi2.processed_files);
    println!("   🔧 Количество потоков: 2");

    // Тест 3: Многопоточный режим (4 потока)
    println!("\n🏃‍♂️ Тест 3: Многопоточный режим (4 потока)");
    let start = Instant::now();
    let settings_multi4 = OptimizationSettings {
        max_threads: Some(4),
        batch_size: 10,
        show_progress: false,
        parallel_indexing: true,
        ..Default::default()
    };
    let mut parser_multi4 = SyntaxHelperParser::with_settings(settings_multi4);
    parser_multi4.parse_directory(&test_path)?;
    let multi4_time = start.elapsed();
    let stats_multi4 = parser_multi4.get_stats();
    println!("   ⏱️ Время выполнения: {:?}", multi4_time);
    println!("   📁 Обработано файлов: {}", stats_multi4.processed_files);
    println!("   🔧 Количество потоков: 4");

    // Тест 4: Полная многопоточность (все доступные ядра)
    println!("\n🚀 Тест 4: Полная многопоточность (все ядра)");
    let start = Instant::now();
    let settings_multi_all = OptimizationSettings {
        max_threads: None, // Использовать все доступные ядра
        batch_size: 50,
        show_progress: false,
        parallel_indexing: true,
        ..Default::default()
    };
    let mut parser_multi_all = SyntaxHelperParser::with_settings(settings_multi_all);
    parser_multi_all.parse_directory(&test_path)?;
    let multi_all_time = start.elapsed();
    let stats_multi_all = parser_multi_all.get_stats();
    println!("   ⏱️ Время выполнения: {:?}", multi_all_time);
    println!(
        "   📁 Обработано файлов: {}",
        stats_multi_all.processed_files
    );
    println!("   🔧 Количество потоков: {} (все ядра)", num_cpus::get());

    // Анализ производительности
    println!("\n📈 Анализ производительности:");
    let speedup_2 = single_time.as_millis() as f64 / multi2_time.as_millis() as f64;
    let speedup_4 = single_time.as_millis() as f64 / multi4_time.as_millis() as f64;
    let speedup_all = single_time.as_millis() as f64 / multi_all_time.as_millis() as f64;

    println!("   🔄 Ускорение с 2 потоками: {:.2}x", speedup_2);
    println!("   🔄 Ускорение с 4 потоками: {:.2}x", speedup_4);
    println!(
        "   🔄 Ускорение с {} потоками: {:.2}x",
        num_cpus::get(),
        speedup_all
    );

    println!("\n🎯 Оптимальные настройки:");
    let times = [single_time, multi2_time, multi4_time, multi_all_time];
    let best_time = times.iter().min().unwrap();

    if best_time == &single_time {
        println!("   ✅ Лучший результат: однопоточный режим");
    } else if best_time == &multi2_time {
        println!("   ✅ Лучший результат: 2 потока");
    } else if best_time == &multi4_time {
        println!("   ✅ Лучший результат: 4 потока");
    } else {
        println!(
            "   ✅ Лучший результат: все доступные ядра ({} потоков)",
            num_cpus::get()
        );
    }

    println!("\n🔍 Детали реализации:");
    println!("   📦 Lock-free структуры: DashMap для thread-safe хранения");
    println!("   ⚛️ Атомарные счётчики: AtomicUsize для статистики");
    println!("   🔄 Параллельная обработка: rayon::par_iter() для файлов");
    println!("   🧱 Batch-обработка: размер батча {} файлов", 50);
    println!("   📚 Параллельная индексация: построение индексов в параллельном режиме");

    Ok(())
}

fn create_test_files(count: usize) -> anyhow::Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test");
    fs::create_dir(&test_dir)?;

    for i in 0..count {
        let html_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>ТестовыйТип{}</title>
</head>
<body>
    <h1 class="V8SH_pagetitle">ТестовыйТип{} (TestType{})</h1>
    <div class="V8SH_descr">
        <p>Описание тестового типа номер {}</p>
    </div>
    <div class="V8SH_availability">
        <span>Доступность: Сервер</span>
    </div>
</body>
</html>"#,
            i, i, i, i
        );

        let file_path = test_dir.join(format!("type_{}.html", i));
        fs::write(file_path, html_content)?;
    }

    Ok(temp_dir)
}
