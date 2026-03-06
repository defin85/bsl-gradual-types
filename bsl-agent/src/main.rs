use bsl_agent::http_ui::start_http_ui;
use bsl_agent::jobs::JobManager;
use bsl_agent::logging::{init_stdio_logging, resolve_log_path_from_process, ResolvedLogPath};
use bsl_agent::server::BslAgentHandler;
use bsl_agent::session::SessionManager;
use bsl_agent::ui_discovery::HttpUiDiscoveryRecord;
use bsl_runtime::system::runtime_config::{global_runtime_config, RuntimeKey};
use rmcp::{transport::stdio, ServiceExt};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
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

fn log_startup_record(resolved_log: &ResolvedLogPath) {
    let cache_dir = global_runtime_config()
        .get_pathbuf(RuntimeKey::CacheDir)
        .map(|path| path.display().to_string());
    let http_addr = global_runtime_config().get_string(RuntimeKey::AgentHttpAddr);

    tracing::info!(
        package = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        profile = option_env!("BSL_AGENT_PROFILE").unwrap_or("unknown"),
        target = option_env!("BSL_AGENT_TARGET").unwrap_or("unknown"),
        git_sha = option_env!("BSL_AGENT_GIT_SHA").unwrap_or("unknown"),
        git_describe = option_env!("BSL_AGENT_GIT_DESCRIBE").unwrap_or("unknown"),
        build_unix_secs = option_env!("BSL_AGENT_BUILD_UNIX_SECS").unwrap_or("unknown"),
        pid = std::process::id(),
        cwd = %resolved_log.cwd.display(),
        log_path = %resolved_log.log_path.display(),
        bsl_cache_dir = ?cache_dir,
        bsl_agent_http_addr = ?http_addr,
        "bsl-agent starting"
    );
}

async fn run_stdio_agent() -> anyhow::Result<()> {
    let session_manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());
    let mut handler =
        BslAgentHandler::with_state(Arc::clone(&session_manager), Arc::clone(&job_manager));

    if let Some(addr_raw) = global_runtime_config().get_string(RuntimeKey::AgentHttpAddr) {
        match parse_http_addr(&addr_raw) {
            Ok(addr) => {
                let static_dir_override =
                    global_runtime_config().get_pathbuf(RuntimeKey::AgentHttpStaticDir);
                let instance_id = Uuid::new_v4().to_string();
                let cache_dir = global_runtime_config()
                    .get_pathbuf(RuntimeKey::CacheDir)
                    .map(|p| p.to_string_lossy().to_string());

                match start_http_ui(
                    addr,
                    static_dir_override,
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
                        handler.set_ui_url(handle.ui_url.clone());
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
        .inspect_err(|err| tracing::error!(error = ?err, "failed to start stdio MCP service"))?;
    service.waiting().await.inspect_err(
        |err| tracing::error!(error = ?err, "stdio MCP service terminated with error"),
    )?;
    tracing::info!("stdio transport closed");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
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

    let resolved_log = match resolve_log_path_from_process() {
        Ok(resolved) => resolved,
        Err(err) => {
            eprintln!("bsl-agent file logging bootstrap failed: {err}");
            return Err(err);
        }
    };

    if let Err(err) = init_stdio_logging(&resolved_log) {
        eprintln!(
            "bsl-agent file logging bootstrap failed for {}: {err}",
            resolved_log.log_path.display()
        );
        return Err(err);
    }

    log_startup_record(&resolved_log);

    if let Err(err) = run_stdio_agent().await {
        tracing::error!(error = ?err, "bsl-agent stdio server failed");
        return Err(err);
    }

    Ok(())
}
