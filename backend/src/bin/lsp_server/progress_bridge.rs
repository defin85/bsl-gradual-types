//! Progress bridge (server -> LSP WorkDoneProgress)
//!
//! Цель: единый и честный прогресс для CPU/IO heavy операций:
//! - проценты монотонны
//! - throttling по времени
//! - последняя отправка не теряется (flush перед end)

use std::time::{Duration, Instant};
use tower_lsp::lsp_types::{
    notification::Progress as ProgressNotification, request::WorkDoneProgressCreate,
    ProgressParams, ProgressParamsValue, ProgressToken, WorkDoneProgress, WorkDoneProgressBegin,
    WorkDoneProgressCreateParams, WorkDoneProgressEnd, WorkDoneProgressReport,
};
use tower_lsp::Client;

#[derive(Debug, Clone, Copy)]
pub struct StageRange {
    pub start: u32,
    pub end: u32,
}

impl StageRange {
    pub fn new(start: u32, end: u32) -> Self {
        debug_assert!(start <= end);
        Self { start, end }
    }

    pub fn map_percent_0_100(&self, p: u32) -> u32 {
        let p = p.min(100);
        let span = self.end.saturating_sub(self.start);
        self.start.saturating_add(((p as f32 / 100.0) * (span as f32)).round() as u32)
    }

    pub fn map_current_total(&self, current: usize, total: usize) -> u32 {
        if total == 0 {
            return self.start;
        }
        let fraction = (current as f32 / total as f32).min(1.0);
        let span = self.end.saturating_sub(self.start) as f32;
        self.start.saturating_add((fraction * span).round() as u32)
    }
}

#[derive(Debug, Clone)]
pub struct ProgressPlan {
    pub validation: StageRange,
    pub discovery: StageRange,
    pub load_types: StageRange,
    pub index_bsl_modules: StageRange,
}

impl ProgressPlan {
    pub fn parse_configuration() -> Self {
        Self {
            validation: StageRange::new(0, 10),
            discovery: StageRange::new(10, 30),
            load_types: StageRange::new(30, 90),
            index_bsl_modules: StageRange::new(90, 99),
        }
    }

    pub fn incremental_update() -> Self {
        // MVP: пока делаем по сути "полный reload", раскладка совпадает с parse_configuration.
        Self::parse_configuration()
    }

    pub fn build_index() -> Self {
        // MVP: buildIndex тоже приводит репозиторий в актуальное состояние.
        Self::parse_configuration()
    }
}

#[tower_lsp::async_trait]
pub trait ProgressReporter {
    async fn begin(&mut self, title: String, message: Option<String>);
    async fn report(&mut self, percentage: u32, message: Option<String>);
    async fn end(&mut self, message: Option<String>);
}

#[derive(Debug, Clone)]
struct PendingReport {
    percentage: u32,
    message: Option<String>,
}

/// Адаптер: ProgressReporter -> LSP WorkDoneProgress ($/progress).
///
/// Особенности:
/// - проценты монотонны (не уменьшаются)
/// - throttling: репорт не чаще, чем throttle_interval (кроме 100%)
/// - последняя "пропущенная" отправка будет сделана при end()
pub struct LspWorkDoneReporter {
    client: Client,
    token: ProgressToken,
    throttle_interval: Duration,
    last_sent: Option<Instant>,
    last_percentage: u32,
    pending: Option<PendingReport>,
}

impl LspWorkDoneReporter {
    pub async fn create(client: Client, token_prefix: &str) -> Self {
        let token = ProgressToken::String(format!(
            "{token_prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0))
                .as_millis()
        ));

        let _ = client
            .send_request::<WorkDoneProgressCreate>(WorkDoneProgressCreateParams {
                token: token.clone(),
            })
            .await;

        Self {
            client,
            token,
            throttle_interval: Duration::from_millis(75),
            last_sent: None,
            last_percentage: 0,
            pending: None,
        }
    }

    pub fn set_throttle_interval(&mut self, throttle_interval: Duration) {
        self.throttle_interval = throttle_interval;
    }

    async fn send_report_unthrottled(&mut self, percentage: u32, message: Option<String>) {
        let percentage = percentage.max(self.last_percentage);
        self.last_percentage = percentage;

        let _ = self
            .client
            .send_notification::<ProgressNotification>(ProgressParams {
                token: self.token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Report(
                    WorkDoneProgressReport {
                        message,
                        percentage: Some(percentage),
                        cancellable: Some(false),
                    },
                )),
            })
            .await;

        self.last_sent = Some(Instant::now());
    }

    async fn flush_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            self.send_report_unthrottled(pending.percentage, pending.message)
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl ProgressReporter for LspWorkDoneReporter {
    async fn begin(&mut self, title: String, message: Option<String>) {
        self.last_sent = None;
        self.last_percentage = 0;
        self.pending = None;

        let _ = self
            .client
            .send_notification::<ProgressNotification>(ProgressParams {
                token: self.token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                    title,
                    message,
                    percentage: Some(0),
                    cancellable: Some(false),
                })),
            })
            .await;
    }

    async fn report(&mut self, percentage: u32, message: Option<String>) {
        // Всегда держим монотонность даже для pending.
        let percentage = percentage.max(self.last_percentage);

        // 100% не отправляем как Report — End закрывает прогресс.
        let percentage = percentage.min(99);

        let now = Instant::now();
        let should_throttle = self
            .last_sent
            .is_some_and(|last| now.duration_since(last) < self.throttle_interval);

        if should_throttle {
            self.pending = Some(PendingReport { percentage, message });
            return;
        }

        self.send_report_unthrottled(percentage, message).await;
        self.pending = None;
    }

    async fn end(&mut self, message: Option<String>) {
        // "last update always delivered"
        self.flush_pending().await;

        let _ = self
            .client
            .send_notification::<ProgressNotification>(ProgressParams {
                token: self.token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message,
                })),
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_range_maps_percent_into_range() {
        let r = StageRange::new(10, 30);
        assert_eq!(r.map_percent_0_100(0), 10);
        assert_eq!(r.map_percent_0_100(50), 20);
        assert_eq!(r.map_percent_0_100(100), 30);
        assert_eq!(r.map_percent_0_100(1000), 30);
    }

    #[test]
    fn stage_range_maps_current_total_into_range() {
        let r = StageRange::new(30, 90);
        assert_eq!(r.map_current_total(0, 0), 30);
        assert_eq!(r.map_current_total(0, 10), 30);
        assert_eq!(r.map_current_total(5, 10), 60);
        assert_eq!(r.map_current_total(10, 10), 90);
        assert_eq!(r.map_current_total(999, 10), 90);
    }
}
