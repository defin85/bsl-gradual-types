use bsl_agent::http_ui::start_http_ui;
use bsl_agent::jobs::JobManager;
use bsl_agent::server::BslAgentHandler;
use bsl_agent::session::SessionManager;
use bsl_agent::ui_discovery::HttpUiDiscoveryRecord;
use rmcp::{transport::stdio, ServiceExt};
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use clap::{Parser, Subcommand};

fn parse_http_addr(raw: &str) -> anyhow::Result<SocketAddr> {
    let raw = raw.trim();
    if raw.is_empty() {
        anyhow::bail!("empty address");
    }

    let normalized = if let Some(port) = raw.strip_prefix(':') {
        format!("127.0.0.1:{port}")
    } else if let Some(port) = raw.strip_prefix("localhost:") {
        format!("127.0.0.1:{port}")
    } else {
        raw.to_string()
    };

    Ok(normalized.parse()?)
}

#[derive(Debug, Parser)]
#[command(name = "bsl-agent")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Ui {
        #[command(subcommand)]
        command: UiCommand,
    },
}

#[derive(Debug, Subcommand)]
enum UiCommand {
    /// List discovered HTTP UI instances.
    List {
        /// Include non-live instances (stale registry records).
        #[arg(long)]
        all: bool,
    },
    /// Print HTTP UI URL for exactly one instance.
    Url {
        /// Select by instance id (from /api/mcp/status).
        #[arg(long, conflicts_with_all = ["pid", "roots"])]
        instance_id: Option<String>,
        /// Select by process id.
        #[arg(long, conflicts_with_all = ["instance_id", "roots"])]
        pid: Option<u32>,
        /// Select by strict root path match (from /api/mcp/sessions).
        #[arg(long, conflicts_with_all = ["instance_id", "pid"])]
        roots: Option<String>,
    },
}

async fn ui_list(all: bool) -> anyhow::Result<()> {
    let records = bsl_agent::ui_discovery::read_all_registry_records();
    let client = reqwest::Client::new();
    let timeout = Duration::from_millis(300);

    let mut rows = Vec::new();
    for record in records {
        let live = bsl_agent::ui_discovery::is_live_instance(&client, &record, timeout).await;
        if !all && !live {
            continue;
        }
        rows.push((record, live));
    }

    let mut out = std::io::BufWriter::new(std::io::stdout());
    writeln!(&mut out, "instance_id\tpid\tui_url\tstatus")?;
    for (record, live) in rows {
        writeln!(
            &mut out,
            "{}\t{}\t{}\t{}",
            record.instance_id,
            record.pid,
            record.ui_url,
            if live { "live" } else { "dead" }
        )?;
    }
    out.flush()?;
    Ok(())
}

async fn ui_url(instance_id: Option<String>, pid: Option<u32>, roots: Option<String>) -> ! {
    let records = bsl_agent::ui_discovery::read_all_registry_records();
    let client = reqwest::Client::new();
    let timeout = Duration::from_millis(300);

    let mut candidates = Vec::new();
    for record in records {
        if let Some(ref want) = instance_id {
            if record.instance_id != *want {
                continue;
            }
        }
        if let Some(want) = pid {
            if record.pid != want {
                continue;
            }
        }

        let live = bsl_agent::ui_discovery::is_live_instance(&client, &record, timeout).await;
        if !live {
            continue;
        }

        if let Some(ref root_path) = roots {
            let matches =
                bsl_agent::ui_discovery::matches_root_path(&client, &record, root_path, timeout)
                    .await;
            if !matches {
                continue;
            }
        }

        candidates.push(record);
    }

    match candidates.len() {
        0 => {
            eprintln!("No matching live HTTP UI instances found.");
            std::process::exit(1);
        }
        1 => {
            println!("{}", candidates[0].ui_url);
            std::process::exit(0);
        }
        _ => {
            eprintln!("Multiple live HTTP UI instances found. Use --instance-id/--pid/--roots.");
            eprintln!("instance_id\tpid\tui_url");
            for record in candidates {
                eprintln!("{}\t{}\t{}", record.instance_id, record.pid, record.ui_url);
            }
            std::process::exit(2);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bsl_agent=debug,info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .init();

    if let Some(command) = cli.command {
        match command {
            Command::Ui {
                command: UiCommand::List { all },
            } => return ui_list(all).await,
            Command::Ui {
                command:
                    UiCommand::Url {
                        instance_id,
                        pid,
                        roots,
                    },
            } => ui_url(instance_id, pid, roots).await,
        }
    }

    tracing::info!("bsl-agent starting...");

    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());
    let handler =
        BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

    if let Ok(addr_raw) = std::env::var("BSL_AGENT_HTTP_ADDR") {
        match parse_http_addr(&addr_raw) {
            Ok(addr) => {
                let static_dir = std::env::var("BSL_AGENT_HTTP_STATIC_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| PathBuf::from("target/site"));
                let instance_id = Uuid::new_v4().to_string();
                let cache_dir = std::env::var("BSL_CACHE_DIR").ok();

                match start_http_ui(
                    addr,
                    static_dir,
                    instance_id.clone(),
                    cache_dir,
                    handler.session_manager(),
                    handler.job_manager(),
                )
                .await
                {
                    Ok(handle) => {
                        let record = HttpUiDiscoveryRecord::new(
                            instance_id.clone(),
                            handle.addr.to_string(),
                            handle.ui_url.clone(),
                        );
                        match bsl_agent::ui_discovery::write_http_ui_registry(&record) {
                            Ok(path) => {
                                tracing::info!("http ui registry written to {}", path.display());
                            }
                            Err(err) => {
                                tracing::warn!("http ui registry write failed: {err}");
                            }
                        }
                        tracing::info!("http ui listening on {}", handle.ui_url);
                    }
                    Err(err) => {
                        tracing::warn!("http ui disabled: {err}");
                    }
                }
            }
            Err(err) => {
                tracing::warn!("http ui disabled: invalid BSL_AGENT_HTTP_ADDR={addr_raw:?}: {err}");
            }
        }
    }

    let service = handler
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("server error: {:?}", e))?;
    service.waiting().await?;

    Ok(())
}
