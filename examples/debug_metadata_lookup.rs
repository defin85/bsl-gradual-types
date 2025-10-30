//! Простой example для отладки TypeMetadataLookup

use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::types::{
    Certainty, ConcreteType, PlatformType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use bsl_shared::domain::TypeMetadataLookup;
use std::sync::Arc;

fn main() {
    println!("🔧 Debug TypeMetadataLookup\n");

    // 1. Парсим
    println!("📚 Parsing syntax helper...");
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("examples/syntax_helper", None::<fn(_)>)
        .expect("Failed to parse");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    println!("✅ Parsed {} types\n", parsed_types.len());

    // 2. Repository
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository.load_types(parsed_types).expect("Failed to load");
    println!("📊 Repository ready\n");

    // 3. Sample content
    let all_types = repository.get_all_types();
    println!("📋 Types with methods (first 10):");
    for (i, t) in all_types
        .iter()
        .filter(|t| !t.methods.is_empty())
        .take(10)
        .enumerate()
    {
        println!("  {}. {} - {} methods", i + 1, t.name, t.methods.len());
    }

    // 4. Test "Массив"
    println!("\n🔍 Testing 'Массив' type:");
    match repository.find_type("Массив") {
        Some(rt) => {
            println!("  ✅ Found in repository!");
            println!("     Methods: {}", rt.methods.len());
            println!("     Properties: {}", rt.properties.len());
            if !rt.methods.is_empty() {
                println!("     First method: {}", rt.methods[0].name);
            }

            // Create resolution
            let resolution = TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                    name: "Массив".to_string(),
                })),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            };

            // Test lookup
            let lookup = TypeMetadataLookup::new(repository.clone());
            let methods = lookup.get_methods(&resolution);
            println!("\n  🔍 TypeMetadataLookup result:");
            println!("     Methods found: {}", methods.len());

            if methods.len() != rt.methods.len() {
                println!(
                    "  ❌ MISMATCH! Expected {} but got {}",
                    rt.methods.len(),
                    methods.len()
                );
            } else {
                println!("  ✅ PASS!");
            }
        }
        None => {
            println!("  ❌ NOT FOUND in repository");

            // Search for similar
            let similar: Vec<_> = all_types
                .iter()
                .filter(|t| {
                    t.name.contains("Масс") || t.english_name.to_lowercase().contains("array")
                })
                .take(5)
                .collect();

            if !similar.is_empty() {
                println!("\n  📝 Similar types:");
                for t in similar {
                    println!("     - {} (en: {})", t.name, t.english_name);
                }
            }
        }
    }
}
