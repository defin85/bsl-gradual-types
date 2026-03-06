#[derive(Debug, Clone)]
struct SemanticProgressSnapshot {
    phase: String,
    percent: u8,
}

#[derive(Clone)]
pub(crate) struct SemanticJobProgress {
    ctx: JobContext,
    operation: &'static str,
    last: Arc<std::sync::Mutex<Option<SemanticProgressSnapshot>>>,
}

impl SemanticJobProgress {
    pub(crate) fn new(ctx: JobContext, operation: &'static str) -> Self {
        Self {
            ctx,
            operation,
            last: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub(crate) async fn stage(&self, stage: &str, percent: u8) {
        let phase = format!("{}/{}", self.operation, stage);
        self.emit(phase, percent).await;
    }

    pub(crate) async fn batch_stage(
        &self,
        stage: &str,
        completed: usize,
        total: usize,
        start_percent: u8,
        end_percent: u8,
    ) {
        let percent =
            interpolate_semantic_progress(completed, total, start_percent, end_percent);
        self.stage(stage, percent).await;
    }

    async fn emit(&self, phase: String, percent: u8) {
        let next_percent = {
            let mut last = self
                .last
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let next_percent = last
                .as_ref()
                .map(|snapshot| snapshot.percent.max(percent.min(99)))
                .unwrap_or_else(|| percent.min(99));
            if last
                .as_ref()
                .is_some_and(|snapshot| snapshot.phase == phase && snapshot.percent == next_percent)
            {
                return;
            }

            *last = Some(SemanticProgressSnapshot {
                phase: phase.clone(),
                percent: next_percent,
            });
            next_percent
        };

        self.ctx.set_progress(phase, next_percent).await;
    }
}

async fn report_job_stage(progress: Option<&SemanticJobProgress>, stage: &str, percent: u8) {
    if let Some(progress) = progress {
        progress.stage(stage, percent).await;
    }
}

async fn report_batch_progress(
    progress: Option<&SemanticJobProgress>,
    stage: &str,
    completed: usize,
    total: usize,
    start_percent: u8,
    end_percent: u8,
) {
    if let Some(progress) = progress {
        progress
            .batch_stage(stage, completed, total, start_percent, end_percent)
            .await;
    }
}

fn interpolate_semantic_progress(
    completed: usize,
    total: usize,
    start_percent: u8,
    end_percent: u8,
) -> u8 {
    let start = start_percent.min(99);
    let end = end_percent.min(99);
    if total == 0 || end <= start {
        return end.max(start);
    }

    let completed = completed.min(total);
    let span = u32::from(end - start);
    let numerator = span.saturating_mul(completed as u32);
    let rounded = (numerator + (total as u32 / 2)) / total as u32;
    start.saturating_add(rounded as u8).min(end)
}

#[cfg(test)]
mod progress_tests {
    use super::interpolate_semantic_progress;

    #[test]
    fn interpolate_progress_is_monotonic_and_bounded() {
        let mut last = 0;
        for completed in 0..=10 {
            let current = interpolate_semantic_progress(completed, 10, 15, 85);
            assert!(
                current >= last,
                "progress must be monotonic: last={last} current={current}"
            );
            assert!(
                (15..=85).contains(&current),
                "progress must stay inside range: {current}"
            );
            last = current;
        }
        assert_eq!(last, 85);
    }

    #[test]
    fn interpolate_progress_handles_empty_batches() {
        assert_eq!(interpolate_semantic_progress(0, 0, 20, 80), 80);
        assert_eq!(interpolate_semantic_progress(5, 0, 20, 80), 80);
    }
}
