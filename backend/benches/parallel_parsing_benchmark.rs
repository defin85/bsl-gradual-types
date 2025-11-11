//! Бенчмарк параллельного парсинга метаданных конфигурации
//!
//! Запуск: cargo bench --bench parallel_parsing_benchmark

use bsl_backend::data::loaders::config_metadata_parser::ConfigurationDiscovery;
use bsl_backend::data::loaders::progress::ProgressUpdate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

fn benchmark_parallel_parsing(c: &mut Criterion) {
    let config_path = PathBuf::from("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Тестовая конфигурация не найдена для бенчмарка");
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path);

    // Обнаруживаем конфигурации один раз (вне бенчмарка)
    let configurations = discovery
        .discover_all_configurations()
        .expect("Не удалось обнаружить конфигурации");

    let config_info = &configurations[0];

    c.bench_function("parallel_parsing_without_callback", |b| {
        b.iter(|| {
            let metadata = discovery
                .discover_metadata_in_configuration(
                    black_box(config_info),
                    None::<fn(ProgressUpdate)>,
                )
                .expect("Не удалось распарсить метаданные");

            black_box(metadata)
        })
    });

    c.bench_function("parallel_parsing_with_callback", |b| {
        b.iter(|| {
            let metadata = discovery
                .discover_metadata_in_configuration(
                    black_box(config_info),
                    Some(|_: ProgressUpdate| {
                        // Пустой callback для измерения overhead
                    }),
                )
                .expect("Не удалось распарсить метаданные");

            black_box(metadata)
        })
    });
}

fn benchmark_configuration_discovery(c: &mut Criterion) {
    let config_path = PathBuf::from("../examples/conf");

    if !config_path.exists() {
        eprintln!("⚠️ Папка с конфигурациями не найдена для бенчмарка");
        return;
    }

    let discovery = ConfigurationDiscovery::new(config_path);

    c.bench_function("discover_all_configurations", |b| {
        b.iter(|| {
            let configurations = discovery
                .discover_all_configurations()
                .expect("Не удалось обнаружить конфигурации");

            black_box(configurations)
        })
    });
}

criterion_group!(
    benches,
    benchmark_parallel_parsing,
    benchmark_configuration_discovery
);
criterion_main!(benches);
