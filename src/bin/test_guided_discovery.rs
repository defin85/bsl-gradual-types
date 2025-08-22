//! CLI тест для Configuration-guided Discovery парсера

use anyhow::Result;
use bsl_gradual_types::adapters::config_parser_guided_discovery::ConfigurationGuidedParser;
use bsl_gradual_types::core::platform_resolver::PlatformTypeResolver;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "test-guided-discovery")]
#[command(about = "CLI тест для Configuration-guided Discovery парсера")]
struct Args {
    /// Путь к директории конфигурации (содержащей Configuration.xml)
    config_path: String,

    /// Verbose вывод
    #[arg(short, long)]
    verbose: bool,

    /// Только парсер (без интеграции с PlatformTypeResolver)
    #[arg(long)]
    parser_only: bool,

    /// Показать детальную статистику
    #[arg(long)]
    stats: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("🚀 Тестирование Configuration-guided Discovery парсера");
    println!("📁 Путь к конфигурации: {}", args.config_path);

    if args.parser_only {
        test_parser_only(&args)?;
    } else {
        test_with_integration(&args)?;
    }

    Ok(())
}

fn test_parser_only(args: &Args) -> Result<()> {
    println!("\n=== 🔧 Тест только парсера ===");

    let mut guided_parser = ConfigurationGuidedParser::new(&args.config_path);

    if args.verbose {
        println!("✅ ConfigurationGuidedParser создан");
    }

    let start_time = std::time::Instant::now();
    let config_types = guided_parser.parse_with_configuration_guide()?;
    let elapsed = start_time.elapsed();

    println!("✅ Парсинг завершен за {:?}", elapsed);
    println!("📊 Найдено типов: {}", config_types.len());

    if args.stats {
        println!("\n📈 Детальная статистика:");
        let mut catalog_count = 0;
        let mut document_count = 0;
        let mut enum_count = 0;
        let mut register_count = 0;
        let mut other_count = 0;

        for type_resolution in &config_types {
            use bsl_gradual_types::core::types::{ConcreteType, ResolutionResult};

            if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) =
                &type_resolution.result
            {
                match config.kind {
                    bsl_gradual_types::core::types::MetadataKind::Catalog => catalog_count += 1,
                    bsl_gradual_types::core::types::MetadataKind::Document => document_count += 1,
                    bsl_gradual_types::core::types::MetadataKind::Enum => enum_count += 1,
                    bsl_gradual_types::core::types::MetadataKind::Register => register_count += 1,
                    _ => other_count += 1,
                }

                if args.verbose {
                    println!(
                        "  - {:?}: {} (атрибутов: {})",
                        config.kind,
                        config.name,
                        config.attributes.len()
                    );
                }
            }
        }

        println!("  📚 Справочники: {}", catalog_count);
        println!("  📄 Документы: {}", document_count);
        println!("  🔢 Перечисления: {}", enum_count);
        println!("  📊 Регистры: {}", register_count);
        println!("  ❓ Другое: {}", other_count);
    }

    Ok(())
}

fn test_with_integration(args: &Args) -> Result<()> {
    println!("\n=== 🔗 Тест с интеграцией в PlatformTypeResolver ===");

    let start_time = std::time::Instant::now();
    let resolver = PlatformTypeResolver::with_guided_config(&args.config_path)?;
    let elapsed = start_time.elapsed();

    println!(
        "✅ PlatformTypeResolver с guided discovery создан за {:?}",
        elapsed
    );
    println!(
        "📊 Platform globals: {}",
        resolver.get_platform_globals_count()
    );

    // Тест основных platform globals
    let globals_to_test = vec![
        "Справочники",
        "Документы",
        "Перечисления",
        "Catalogs",
        "Documents",
        "Enums",
    ];

    println!("\n🔍 Проверка platform globals:");
    for global in &globals_to_test {
        if resolver.has_platform_global(global) {
            println!("  ✅ {}", global);
        } else {
            println!("  ❌ {}", global);
        }
    }

    if args.stats {
        println!("\n📊 Подробная статистика resolver'а:");
        println!(
            "  - Platform globals: {}",
            resolver.get_platform_globals_count()
        );

        // Дополнительные статистики можно добавить позже
    }

    Ok(())
}
