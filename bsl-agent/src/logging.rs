use anyhow::Context;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const DEFAULT_LOG_DIR_NAME: &str = ".bsl-agent";
pub const DEFAULT_LOG_FILE_NAME: &str = "mcp.log";
static CURRENT_LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoggingEnvOverrides {
    pub log_dir: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLogPath {
    pub cwd: PathBuf,
    pub log_path: PathBuf,
}

#[derive(Clone)]
struct SharedFileMakeWriter {
    file: Arc<Mutex<File>>,
}

struct SharedFileGuard<'a> {
    guard: MutexGuard<'a, File>,
}

impl SharedFileMakeWriter {
    fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }

    fn lock(&self) -> MutexGuard<'_, File> {
        match self.file.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl<'a> MakeWriter<'a> for SharedFileMakeWriter {
    type Writer = SharedFileGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        SharedFileGuard { guard: self.lock() }
    }
}

impl Write for SharedFileGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.guard.flush()
    }
}

fn path_from_env(var_name: &str) -> Option<PathBuf> {
    let value = std::env::var_os(var_name)?;
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn env_overrides_from_process() -> LoggingEnvOverrides {
    LoggingEnvOverrides {
        log_dir: path_from_env("BSL_AGENT_LOG_DIR"),
        log_file: path_from_env("BSL_AGENT_LOG_FILE"),
    }
}

pub fn resolve_log_path(cwd: &Path, overrides: &LoggingEnvOverrides) -> PathBuf {
    if let Some(log_file) = overrides.log_file.as_ref() {
        return log_file.clone();
    }

    if let Some(log_dir) = overrides.log_dir.as_ref() {
        return log_dir.join(DEFAULT_LOG_FILE_NAME);
    }

    cwd.join(DEFAULT_LOG_DIR_NAME).join(DEFAULT_LOG_FILE_NAME)
}

pub fn resolve_log_path_from_process() -> anyhow::Result<ResolvedLogPath> {
    let cwd = std::env::current_dir().context("resolve current working directory for log path")?;
    let log_path = resolve_log_path(&cwd, &env_overrides_from_process());
    Ok(ResolvedLogPath { cwd, log_path })
}

pub fn init_stdio_logging(resolved: &ResolvedLogPath) -> anyhow::Result<()> {
    if let Some(parent) = resolved.log_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create log directory {}", parent.display()))?;
    }

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&resolved.log_path)
        .with_context(|| format!("open log file {}", resolved.log_path.display()))?;

    let file_writer = SharedFileMakeWriter::new(log_file);
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "bsl_agent=debug,info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(file_writer.clone()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(std::io::stderr),
        )
        .try_init()
        .context("initialize tracing subscriber")?;

    let _ = CURRENT_LOG_FILE_PATH.set(resolved.log_path.clone());
    Ok(())
}

pub fn current_log_file_path() -> Option<&'static Path> {
    CURRENT_LOG_FILE_PATH.get().map(PathBuf::as_path)
}

#[cfg(test)]
mod tests {
    use super::{
        resolve_log_path, LoggingEnvOverrides, DEFAULT_LOG_DIR_NAME, DEFAULT_LOG_FILE_NAME,
    };
    use std::path::Path;

    #[test]
    fn resolve_log_path_uses_process_cwd_by_default() {
        let cwd = Path::new("/tmp/workspace");
        let resolved = resolve_log_path(cwd, &LoggingEnvOverrides::default());

        assert_eq!(
            resolved,
            cwd.join(DEFAULT_LOG_DIR_NAME).join(DEFAULT_LOG_FILE_NAME)
        );
    }

    #[test]
    fn resolve_log_path_prefers_log_file_override_over_log_dir() {
        let cwd = Path::new("/tmp/workspace");
        let resolved = resolve_log_path(
            cwd,
            &LoggingEnvOverrides {
                log_dir: Some(Path::new("/tmp/log-dir").to_path_buf()),
                log_file: Some(Path::new("/tmp/custom.log").to_path_buf()),
            },
        );

        assert_eq!(resolved, Path::new("/tmp/custom.log"));
    }

    #[test]
    fn resolve_log_path_uses_log_dir_override_with_stable_filename() {
        let cwd = Path::new("/tmp/workspace");
        let resolved = resolve_log_path(
            cwd,
            &LoggingEnvOverrides {
                log_dir: Some(Path::new("/tmp/log-dir").to_path_buf()),
                log_file: None,
            },
        );

        assert_eq!(
            resolved,
            Path::new("/tmp/log-dir").join(DEFAULT_LOG_FILE_NAME)
        );
    }
}
