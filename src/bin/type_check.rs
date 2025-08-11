//! Simple CLI for testing type resolution

use clap::Parser;
use bsl_gradual_types::core::platform_resolver::PlatformTypeResolver;
use bsl_gradual_types::core::types::Certainty;

#[derive(Parser, Debug)]
#[command(name = "type-check")]
#[command(about = "BSL Type Checker - test type resolution for expressions")]
struct Args {
    /// Expression to resolve (e.g., "Справочники.Контрагенты")
    /// Or use --complete to get completions
    expression: String,
    
    /// Path to configuration XML (optional)
    #[arg(short, long)]
    config: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Get completions for the expression
    #[arg(long)]
    complete: bool,
}

fn main() {
    let args = Args::parse();
    
    // Create resolver
    let mut resolver = if let Some(config_path) = args.config {
        println!("Loading configuration from: {}", config_path);
        match PlatformTypeResolver::with_config(&config_path) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to load config: {}", e);
                PlatformTypeResolver::new()
            }
        }
    } else {
        println!("Using platform types only (no configuration)");
        PlatformTypeResolver::new()
    };
    
    // Check if we need completions
    if args.complete {
        show_completions(&resolver, &args.expression);
        return;
    }
    
    // Resolve expression
    println!("\nResolving: {}", args.expression);
    println!("{}", "─".repeat(50));
    
    let resolution = resolver.resolve_expression(&args.expression);
    
    // Display result
    match resolution.certainty {
        Certainty::Known => {
            println!("✅ Type is KNOWN");
        }
        Certainty::Inferred(confidence) => {
            println!("🔍 Type is INFERRED (confidence: {:.0}%)", confidence * 100.0);
        }
        Certainty::Unknown => {
            println!("❓ Type is UNKNOWN");
        }
    }
    
    // Show facet information
    if let Some(active) = &resolution.active_facet {
        println!("  Active facet: {:?}", active);
    }
    
    if !resolution.available_facets.is_empty() {
        println!("  Available facets: {:?}", resolution.available_facets);
    }
    
    if args.verbose {
        println!("\nDetails:");
        println!("  Source: {:?}", resolution.source);
        
        if let Some(file) = &resolution.metadata.file {
            println!("  File: {}", file);
        }
        
        if !resolution.metadata.notes.is_empty() {
            println!("  Notes:");
            for note in &resolution.metadata.notes {
                println!("    - {}", note);
            }
        }
        
        println!("\n  Result: {:#?}", resolution.result);
    } else {
        // Simple output
        use bsl_gradual_types::core::types::{ResolutionResult, ConcreteType};
        
        match &resolution.result {
            ResolutionResult::Concrete(concrete) => {
                match concrete {
                    ConcreteType::Platform(p) => {
                        println!("  Type: Platform.{}", p.name);
                        if !p.methods.is_empty() {
                            println!("  Methods: {}", p.methods.len());
                        }
                    }
                    ConcreteType::Configuration(c) => {
                        println!("  Type: {:?}.{}", c.kind, c.name);
                    }
                    ConcreteType::Primitive(p) => {
                        println!("  Type: Primitive.{:?}", p);
                    }
                    ConcreteType::Special(s) => {
                        println!("  Type: Special.{:?}", s);
                    }
                }
            }
            ResolutionResult::Union(types) => {
                println!("  Type: Union of {} types", types.len());
            }
            ResolutionResult::Dynamic => {
                println!("  Type: Dynamic (runtime)");
            }
            _ => {
                println!("  Type: Complex");
            }
        }
    }
    
    // Test some examples if no expression provided
    if args.expression.is_empty() {
        println!("\n\nExamples:");
        println!("{}", "─".repeat(50));
        
        for expr in &[
            "Справочники",
            "Справочники.Контрагенты", 
            "Документы.ЗаказПокупателя",
            "Перечисления.СтатусыЗаказов",
            "НеизвестныйТип",
        ] {
            let res = resolver.resolve_expression(expr);
            let icon = match res.certainty {
                Certainty::Known => "✅",
                Certainty::Inferred(_) => "🔍",
                Certainty::Unknown => "❓",
            };
            println!("{} {} -> {:?}", icon, expr, res.certainty);
        }
    }
}

fn show_completions(resolver: &PlatformTypeResolver, prefix: &str) {
    use bsl_gradual_types::core::platform_resolver::CompletionKind;
    
    println!("\nCompletions for: {}", prefix);
    println!("{}", "─".repeat(50));
    
    let completions = resolver.get_completions(prefix);
    
    if completions.is_empty() {
        println!("No completions found");
        return;
    }
    
    // Group by kind
    let mut by_kind = std::collections::HashMap::new();
    for item in completions {
        by_kind.entry(item.kind.clone()).or_insert(Vec::new()).push(item);
    }
    
    // Display grouped
    for (kind, items) in by_kind {
        let kind_name = match kind {
            CompletionKind::Global => "🌐 Globals",
            CompletionKind::Catalog => "📁 Catalogs",
            CompletionKind::Document => "📄 Documents",
            CompletionKind::Enum => "📝 Enums",
            CompletionKind::Method => "🔧 Methods",
            CompletionKind::Property => "📌 Properties",
        };
        
        println!("\n{}:", kind_name);
        for item in items {
            print!("  • {}", item.label);
            if let Some(detail) = &item.detail {
                print!(" - {}", detail);
            }
            println!();
        }
    }
}