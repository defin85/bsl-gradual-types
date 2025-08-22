//! Simple CLI for testing type resolution (target-only)

use bsl_gradual_types::architecture::presentation::{LspCompletionRequest, LspHoverRequest};
use bsl_gradual_types::target::system::{CentralSystemConfig, CentralTypeSystem};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "type-check")]
#[command(about = "BSL Type Checker - test type resolution for expressions (target)")]
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
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut cfg = CentralSystemConfig::default();
        if let Some(ref path) = args.config {
            cfg.configuration_path = Some(path.clone());
        }
        let central = CentralTypeSystem::new(cfg);
        if let Err(e) = central.initialize().await {
            eprintln!("Initialization error: {}", e);
        }
        if args.complete {
            let req = LspCompletionRequest {
                file_path: "cli".to_string(),
                line: 0,
                column: 0,
                prefix: args.expression.clone(),
                trigger_character: None,
            };
            match central.lsp_interface().handle_completion_request(req).await {
                Ok(resp) => {
                    if resp.items.is_empty() {
                        println!("No completions found");
                    } else {
                        println!("Completions:");
                        for it in resp.items {
                            println!("  • {}", it.label);
                        }
                    }
                }
                Err(e) => eprintln!("Completion error: {}", e),
            }
        } else {
            let req = LspHoverRequest {
                file_path: "cli".to_string(),
                line: 0,
                column: 0,
                expression: args.expression.clone(),
            };
            match central.lsp_interface().handle_hover_request(req).await {
                Ok(Some(h)) => {
                    println!("{}", h.contents.join("\n\n"));
                }
                Ok(None) => println!("Type unknown"),
                Err(e) => eprintln!("Hover error: {}", e),
            }
        }
    });
}
