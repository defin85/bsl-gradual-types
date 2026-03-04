fn unix_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn compute_index_fetch_wait_ms(
    index_fetch_ms: u128,
    index_parse_result_ms: u128,
    index_build_total_ms: u128,
) -> u128 {
    index_fetch_ms.saturating_sub(index_parse_result_ms.saturating_add(index_build_total_ms))
}

fn compute_index_fetch_salsa_event_edges_ms(
    index_fetch_ms: u128,
    first_event_elapsed_ms: Option<u128>,
    last_event_elapsed_ms: Option<u128>,
) -> (u128, u128) {
    let first_event_elapsed_ms = first_event_elapsed_ms
        .unwrap_or(index_fetch_ms)
        .min(index_fetch_ms);
    let last_event_elapsed_ms = last_event_elapsed_ms
        .unwrap_or(first_event_elapsed_ms)
        .min(index_fetch_ms)
        .max(first_event_elapsed_ms);
    let pre_first_salsa_event_wait_ms = first_event_elapsed_ms;
    let post_last_salsa_event_tail_ms = index_fetch_ms.saturating_sub(last_event_elapsed_ms);
    (pre_first_salsa_event_wait_ms, post_last_salsa_event_tail_ms)
}

fn compute_index_fetch_inside_salsa_window_ms(
    index_fetch_ms: u128,
    pre_first_salsa_event_wait_ms: u128,
    post_last_salsa_event_tail_ms: u128,
) -> u128 {
    index_fetch_ms
        .saturating_sub(pre_first_salsa_event_wait_ms)
        .saturating_sub(post_last_salsa_event_tail_ms)
}

fn compute_index_fetch_event_delta_ms(
    index_fetch_ms: u128,
    from_elapsed_ms: Option<u128>,
    to_elapsed_ms: Option<u128>,
) -> u128 {
    match (from_elapsed_ms, to_elapsed_ms) {
        (Some(from_elapsed_ms), Some(to_elapsed_ms)) => to_elapsed_ms
            .min(index_fetch_ms)
            .saturating_sub(from_elapsed_ms.min(index_fetch_ms)),
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct FirstTypeIndexTimelineSnapshot {
    first_will_execute_type_index_elapsed_ms: Option<u128>,
    last_event_before_first_will_execute_type_index_elapsed_ms: Option<u128>,
    last_will_check_before_first_will_execute_type_index_elapsed_ms: Option<u128>,
    last_will_execute_parse_result_before_first_will_execute_type_index_elapsed_ms: Option<u128>,
    events_before_first_will_execute_type_index_total: u64,
    will_check_before_first_will_execute_type_index_total: u64,
    will_execute_parse_result_before_first_will_execute_type_index_total: u64,
    first_will_execute_type_index_seen_total: u64,
}

fn compute_first_type_index_timeline_snapshot(
    events: &[SalsaEventTimelineEvent],
    index_fetch_ms: u128,
) -> FirstTypeIndexTimelineSnapshot {
    let mut snapshot = FirstTypeIndexTimelineSnapshot::default();
    for event in events {
        let elapsed_ms = event.elapsed_ms.min(index_fetch_ms);
        if matches!(
            event.kind,
            SalsaEventTimelineEventKind::WillExecute(SalsaEventKeyKind::TypeIndex)
        ) {
            snapshot.first_will_execute_type_index_elapsed_ms = Some(elapsed_ms);
            snapshot.first_will_execute_type_index_seen_total = 1;
            break;
        }

        snapshot.events_before_first_will_execute_type_index_total = snapshot
            .events_before_first_will_execute_type_index_total
            .saturating_add(1);
        snapshot.last_event_before_first_will_execute_type_index_elapsed_ms = Some(elapsed_ms);
        if matches!(
            event.kind,
            SalsaEventTimelineEventKind::WillCheckCancellation
        ) {
            snapshot.last_will_check_before_first_will_execute_type_index_elapsed_ms =
                Some(elapsed_ms);
            snapshot.will_check_before_first_will_execute_type_index_total = snapshot
                .will_check_before_first_will_execute_type_index_total
                .saturating_add(1);
        }
        if matches!(
            event.kind,
            SalsaEventTimelineEventKind::WillExecute(SalsaEventKeyKind::ParseResult)
        ) {
            snapshot
                .last_will_execute_parse_result_before_first_will_execute_type_index_elapsed_ms =
                Some(elapsed_ms);
            snapshot.will_execute_parse_result_before_first_will_execute_type_index_total =
                snapshot
                    .will_execute_parse_result_before_first_will_execute_type_index_total
                    .saturating_add(1);
        }
    }
    snapshot
}

fn slow_index_fetch_log_threshold_ms() -> Option<u128> {
    static THRESHOLD: OnceLock<Option<u128>> = OnceLock::new();
    *THRESHOLD.get_or_init(|| {
        std::env::var("BSL_V2_TYPE_INDEX_FETCH_SLOW_LOG_MS")
            .ok()
            .and_then(|raw| raw.trim().parse::<u128>().ok())
            .filter(|value| *value > 0)
    })
}

fn env_flag_enabled(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|raw| {
        let normalized = raw.trim().to_ascii_lowercase();
        !matches!(normalized.as_str(), "0" | "false" | "off" | "no")
    })
}

fn slow_index_fetch_log_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        env_flag_enabled("BSL_V2_TYPE_INDEX_FETCH_SLOW_LOG")
            .unwrap_or_else(|| std::env::var_os("BSL_V2_SCALE_AWARE_GATE_REPORT").is_none())
    })
}

fn type_index_fetch_timeline_max_events() -> usize {
    static MAX_EVENTS: OnceLock<usize> = OnceLock::new();
    *MAX_EVENTS.get_or_init(|| {
        let default_max_events =
            if slow_index_fetch_log_enabled() && slow_index_fetch_log_threshold_ms().is_some() {
                128
            } else {
                0
            };
        std::env::var("BSL_V2_TYPE_INDEX_FETCH_TIMELINE_MAX_EVENTS")
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok())
            .map(|value| value.min(4096))
            .unwrap_or(default_max_events)
    })
}

fn revision_to_u64(revision: salsa::Revision) -> u64 {
    let debug = format!("{revision:?}");
    debug
        .strip_prefix('R')
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}

fn current_revision_u64(db: &AnalysisDatabase) -> u64 {
    revision_to_u64(salsa::plumbing::current_revision(db))
}

fn compute_index_fetch_key_kind_other_total(
    total: u64,
    type_index_total: u64,
    parse_result_total: u64,
) -> u64 {
    total.saturating_sub(type_index_total.saturating_add(parse_result_total))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SalsaEventCounters {
    will_block_on_total: u64,
    will_block_on_type_index: u64,
    will_block_on_parse_result: u64,
    will_execute_total: u64,
    will_execute_type_index: u64,
    will_execute_parse_result: u64,
    will_iterate_cycle_total: u64,
    did_validate_memoized_total: u64,
    did_validate_memoized_type_index: u64,
    did_validate_memoized_parse_result: u64,
    will_check_cancellation_total: u64,
    did_set_cancellation_flag_total: u64,
    did_discard_total: u64,
    did_discard_accumulated_total: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct SalsaEventTimeline {
    active: bool,
    started_at: Option<Instant>,
    first_event_elapsed_ms: Option<u128>,
    last_event_elapsed_ms: Option<u128>,
    first_will_execute_type_index_elapsed_ms: Option<u128>,
    last_will_execute_type_index_elapsed_ms: Option<u128>,
    first_will_execute_parse_result_elapsed_ms: Option<u128>,
    last_will_execute_parse_result_elapsed_ms: Option<u128>,
    first_will_execute_other_elapsed_ms: Option<u128>,
    last_will_execute_other_elapsed_ms: Option<u128>,
    first_will_iterate_cycle_elapsed_ms: Option<u128>,
    last_will_iterate_cycle_elapsed_ms: Option<u128>,
    first_will_check_cancellation_elapsed_ms: Option<u128>,
    last_will_check_cancellation_elapsed_ms: Option<u128>,
    event_capture_limit: usize,
    event_total: u64,
    event_truncated: bool,
    events: Vec<SalsaEventTimelineEvent>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct SalsaEventTimelineSnapshot {
    first_event_elapsed_ms: Option<u128>,
    last_event_elapsed_ms: Option<u128>,
    first_will_execute_type_index_elapsed_ms: Option<u128>,
    last_will_execute_type_index_elapsed_ms: Option<u128>,
    first_will_execute_parse_result_elapsed_ms: Option<u128>,
    last_will_execute_parse_result_elapsed_ms: Option<u128>,
    first_will_execute_other_elapsed_ms: Option<u128>,
    last_will_execute_other_elapsed_ms: Option<u128>,
    first_will_iterate_cycle_elapsed_ms: Option<u128>,
    last_will_iterate_cycle_elapsed_ms: Option<u128>,
    first_will_check_cancellation_elapsed_ms: Option<u128>,
    last_will_check_cancellation_elapsed_ms: Option<u128>,
    event_capture_limit: usize,
    event_total: u64,
    event_truncated: bool,
    events: Vec<SalsaEventTimelineEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SalsaEventTimelineEvent {
    elapsed_ms: u128,
    kind: SalsaEventTimelineEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SalsaEventTimelineEventKind {
    WillBlockOn(SalsaEventKeyKind),
    WillExecute(SalsaEventKeyKind),
    DidValidateMemoized(SalsaEventKeyKind),
    WillCheckCancellation,
    WillIterateCycle,
    DidSetCancellationFlag,
    DidDiscard,
    DidDiscardAccumulated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SalsaEventTimelineMarker {
    Generic,
    WillExecuteTypeIndex,
    WillExecuteParseResult,
    WillExecuteOther,
    WillIterateCycle,
    WillCheckCancellation,
}

struct ActiveTypeIndexFetchGuard;

impl ActiveTypeIndexFetchGuard {
    fn enter() -> u64 {
        ANALYSIS_V2_ACTIVE_TYPE_INDEX_FETCHES
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }
}

impl Drop for ActiveTypeIndexFetchGuard {
    fn drop(&mut self) {
        ANALYSIS_V2_ACTIVE_TYPE_INDEX_FETCHES.fetch_sub(1, Ordering::Relaxed);
    }
}

thread_local! {
    static ANALYSIS_V2_SALSA_EVENT_COUNTERS: Cell<SalsaEventCounters> = const {
        Cell::new(SalsaEventCounters {
            will_block_on_total: 0,
            will_block_on_type_index: 0,
            will_block_on_parse_result: 0,
            will_execute_total: 0,
            will_execute_type_index: 0,
            will_execute_parse_result: 0,
            will_iterate_cycle_total: 0,
            did_validate_memoized_total: 0,
            did_validate_memoized_type_index: 0,
            did_validate_memoized_parse_result: 0,
            will_check_cancellation_total: 0,
            did_set_cancellation_flag_total: 0,
            did_discard_total: 0,
            did_discard_accumulated_total: 0,
        })
    };
    static ANALYSIS_V2_SALSA_EVENT_TIMELINE: RefCell<SalsaEventTimeline> = RefCell::new(SalsaEventTimeline::default());
}

static ANALYSIS_V2_ACTIVE_TYPE_INDEX_FETCHES: AtomicU64 = AtomicU64::new(0);
static ANALYSIS_V2_GLOBAL_DID_SET_CANCELLATION_FLAG_TOTAL: AtomicU64 = AtomicU64::new(0);

fn salsa_event_counters_snapshot() -> SalsaEventCounters {
    ANALYSIS_V2_SALSA_EVENT_COUNTERS.with(Cell::get)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SalsaEventKeyKind {
    TypeIndex,
    ParseResult,
    Other,
}

fn salsa_event_key_kind(database_key: salsa::DatabaseKeyIndex) -> SalsaEventKeyKind {
    let database_key_debug = format!("{database_key:?}").to_ascii_lowercase();
    if database_key_debug.contains("type_index") {
        SalsaEventKeyKind::TypeIndex
    } else if database_key_debug.contains("parse_result") {
        SalsaEventKeyKind::ParseResult
    } else {
        SalsaEventKeyKind::Other
    }
}

fn update_salsa_event_counters(op: impl FnOnce(&mut SalsaEventCounters)) {
    ANALYSIS_V2_SALSA_EVENT_COUNTERS.with(|cell| {
        let mut counters = cell.get();
        op(&mut counters);
        cell.set(counters);
    });
}

fn begin_salsa_event_timeline() {
    let event_capture_limit = type_index_fetch_timeline_max_events();
    ANALYSIS_V2_SALSA_EVENT_TIMELINE.with(|cell| {
        *cell.borrow_mut() = SalsaEventTimeline {
            active: true,
            started_at: Some(Instant::now()),
            first_event_elapsed_ms: None,
            last_event_elapsed_ms: None,
            first_will_execute_type_index_elapsed_ms: None,
            last_will_execute_type_index_elapsed_ms: None,
            first_will_execute_parse_result_elapsed_ms: None,
            last_will_execute_parse_result_elapsed_ms: None,
            first_will_execute_other_elapsed_ms: None,
            last_will_execute_other_elapsed_ms: None,
            first_will_iterate_cycle_elapsed_ms: None,
            last_will_iterate_cycle_elapsed_ms: None,
            first_will_check_cancellation_elapsed_ms: None,
            last_will_check_cancellation_elapsed_ms: None,
            event_capture_limit,
            event_total: 0,
            event_truncated: false,
            events: Vec::new(),
        };
    });
}

fn finish_salsa_event_timeline() -> SalsaEventTimelineSnapshot {
    ANALYSIS_V2_SALSA_EVENT_TIMELINE.with(|cell| {
        let mut timeline = cell.borrow_mut();
        let snapshot = SalsaEventTimelineSnapshot {
            first_event_elapsed_ms: timeline.first_event_elapsed_ms,
            last_event_elapsed_ms: timeline.last_event_elapsed_ms,
            first_will_execute_type_index_elapsed_ms: timeline
                .first_will_execute_type_index_elapsed_ms,
            last_will_execute_type_index_elapsed_ms: timeline
                .last_will_execute_type_index_elapsed_ms,
            first_will_execute_parse_result_elapsed_ms: timeline
                .first_will_execute_parse_result_elapsed_ms,
            last_will_execute_parse_result_elapsed_ms: timeline
                .last_will_execute_parse_result_elapsed_ms,
            first_will_execute_other_elapsed_ms: timeline.first_will_execute_other_elapsed_ms,
            last_will_execute_other_elapsed_ms: timeline.last_will_execute_other_elapsed_ms,
            first_will_iterate_cycle_elapsed_ms: timeline.first_will_iterate_cycle_elapsed_ms,
            last_will_iterate_cycle_elapsed_ms: timeline.last_will_iterate_cycle_elapsed_ms,
            first_will_check_cancellation_elapsed_ms: timeline
                .first_will_check_cancellation_elapsed_ms,
            last_will_check_cancellation_elapsed_ms: timeline
                .last_will_check_cancellation_elapsed_ms,
            event_capture_limit: timeline.event_capture_limit,
            event_total: timeline.event_total,
            event_truncated: timeline.event_truncated,
            events: std::mem::take(&mut timeline.events),
        };
        *timeline = SalsaEventTimeline::default();
        snapshot
    })
}

fn record_salsa_event_timeline_marker(
    marker: SalsaEventTimelineMarker,
    event_kind: Option<SalsaEventTimelineEventKind>,
) {
    ANALYSIS_V2_SALSA_EVENT_TIMELINE.with(|cell| {
        let mut timeline = cell.borrow_mut();
        if !timeline.active {
            return;
        }
        let Some(started_at) = timeline.started_at else {
            return;
        };
        let elapsed_ms = started_at.elapsed().as_millis();
        if timeline.first_event_elapsed_ms.is_none() {
            timeline.first_event_elapsed_ms = Some(elapsed_ms);
        }
        timeline.last_event_elapsed_ms = Some(elapsed_ms);
        match marker {
            SalsaEventTimelineMarker::Generic => {}
            SalsaEventTimelineMarker::WillExecuteTypeIndex => {
                if timeline.first_will_execute_type_index_elapsed_ms.is_none() {
                    timeline.first_will_execute_type_index_elapsed_ms = Some(elapsed_ms);
                }
                timeline.last_will_execute_type_index_elapsed_ms = Some(elapsed_ms);
            }
            SalsaEventTimelineMarker::WillExecuteParseResult => {
                if timeline
                    .first_will_execute_parse_result_elapsed_ms
                    .is_none()
                {
                    timeline.first_will_execute_parse_result_elapsed_ms = Some(elapsed_ms);
                }
                timeline.last_will_execute_parse_result_elapsed_ms = Some(elapsed_ms);
            }
            SalsaEventTimelineMarker::WillExecuteOther => {
                if timeline.first_will_execute_other_elapsed_ms.is_none() {
                    timeline.first_will_execute_other_elapsed_ms = Some(elapsed_ms);
                }
                timeline.last_will_execute_other_elapsed_ms = Some(elapsed_ms);
            }
            SalsaEventTimelineMarker::WillIterateCycle => {
                if timeline.first_will_iterate_cycle_elapsed_ms.is_none() {
                    timeline.first_will_iterate_cycle_elapsed_ms = Some(elapsed_ms);
                }
                timeline.last_will_iterate_cycle_elapsed_ms = Some(elapsed_ms);
            }
            SalsaEventTimelineMarker::WillCheckCancellation => {
                if timeline.first_will_check_cancellation_elapsed_ms.is_none() {
                    timeline.first_will_check_cancellation_elapsed_ms = Some(elapsed_ms);
                }
                timeline.last_will_check_cancellation_elapsed_ms = Some(elapsed_ms);
            }
        }

        if let Some(event_kind) = event_kind {
            timeline.event_total = timeline.event_total.saturating_add(1);
            if timeline.events.len() < timeline.event_capture_limit {
                timeline.events.push(SalsaEventTimelineEvent {
                    elapsed_ms,
                    kind: event_kind,
                });
            } else if timeline.event_capture_limit > 0 {
                timeline.event_truncated = true;
            }
        }
    });
}

fn salsa_event_key_kind_label(kind: SalsaEventKeyKind) -> &'static str {
    match kind {
        SalsaEventKeyKind::TypeIndex => "type_index",
        SalsaEventKeyKind::ParseResult => "parse_result",
        SalsaEventKeyKind::Other => "other",
    }
}

fn append_salsa_event_timeline_event_kind_label(
    output: &mut String,
    kind: SalsaEventTimelineEventKind,
) {
    match kind {
        SalsaEventTimelineEventKind::WillBlockOn(key_kind) => {
            output.push_str("will_block_on(");
            output.push_str(salsa_event_key_kind_label(key_kind));
            output.push(')');
        }
        SalsaEventTimelineEventKind::WillExecute(key_kind) => {
            output.push_str("will_execute(");
            output.push_str(salsa_event_key_kind_label(key_kind));
            output.push(')');
        }
        SalsaEventTimelineEventKind::DidValidateMemoized(key_kind) => {
            output.push_str("did_validate_memoized(");
            output.push_str(salsa_event_key_kind_label(key_kind));
            output.push(')');
        }
        SalsaEventTimelineEventKind::WillCheckCancellation => {
            output.push_str("will_check_cancellation");
        }
        SalsaEventTimelineEventKind::WillIterateCycle => {
            output.push_str("will_iterate_cycle");
        }
        SalsaEventTimelineEventKind::DidSetCancellationFlag => {
            output.push_str("did_set_cancellation_flag");
        }
        SalsaEventTimelineEventKind::DidDiscard => {
            output.push_str("did_discard");
        }
        SalsaEventTimelineEventKind::DidDiscardAccumulated => {
            output.push_str("did_discard_accumulated");
        }
    }
}

fn format_salsa_event_timeline(snapshot: &SalsaEventTimelineSnapshot) -> String {
    if snapshot.event_capture_limit == 0 {
        return "disabled".to_string();
    }
    if snapshot.events.is_empty() {
        return "empty".to_string();
    }

    let mut output = String::new();
    let mut previous_elapsed_ms = 0_u128;
    for (index, event) in snapshot.events.iter().enumerate() {
        if index > 0 {
            output.push('|');
        }
        let delta_ms = if index == 0 {
            event.elapsed_ms
        } else {
            event.elapsed_ms.saturating_sub(previous_elapsed_ms)
        };
        output.push_str(&event.elapsed_ms.to_string());
        output.push_str("ms:+");
        output.push_str(&delta_ms.to_string());
        output.push(':');
        append_salsa_event_timeline_event_kind_label(&mut output, event.kind);
        previous_elapsed_ms = event.elapsed_ms;
    }
    output
}

fn record_analysis_database_salsa_event(event: salsa::Event) {
    match event.kind {
        salsa::EventKind::WillBlockOn { database_key, .. } => {
            let kind = salsa_event_key_kind(database_key);
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::Generic,
                Some(SalsaEventTimelineEventKind::WillBlockOn(kind)),
            );
            update_salsa_event_counters(|counters| {
                counters.will_block_on_total = counters.will_block_on_total.saturating_add(1);
                match kind {
                    SalsaEventKeyKind::TypeIndex => {
                        counters.will_block_on_type_index =
                            counters.will_block_on_type_index.saturating_add(1);
                    }
                    SalsaEventKeyKind::ParseResult => {
                        counters.will_block_on_parse_result =
                            counters.will_block_on_parse_result.saturating_add(1);
                    }
                    SalsaEventKeyKind::Other => {}
                }
            });
        }
        salsa::EventKind::WillExecute { database_key } => {
            let kind = salsa_event_key_kind(database_key);
            let marker = match kind {
                SalsaEventKeyKind::TypeIndex => SalsaEventTimelineMarker::WillExecuteTypeIndex,
                SalsaEventKeyKind::ParseResult => SalsaEventTimelineMarker::WillExecuteParseResult,
                SalsaEventKeyKind::Other => SalsaEventTimelineMarker::WillExecuteOther,
            };
            record_salsa_event_timeline_marker(
                marker,
                Some(SalsaEventTimelineEventKind::WillExecute(kind)),
            );
            update_salsa_event_counters(|counters| {
                counters.will_execute_total = counters.will_execute_total.saturating_add(1);
                match kind {
                    SalsaEventKeyKind::TypeIndex => {
                        counters.will_execute_type_index =
                            counters.will_execute_type_index.saturating_add(1);
                    }
                    SalsaEventKeyKind::ParseResult => {
                        counters.will_execute_parse_result =
                            counters.will_execute_parse_result.saturating_add(1);
                    }
                    SalsaEventKeyKind::Other => {}
                }
            });
        }
        salsa::EventKind::DidValidateMemoizedValue { database_key } => {
            let kind = salsa_event_key_kind(database_key);
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::Generic,
                Some(SalsaEventTimelineEventKind::DidValidateMemoized(kind)),
            );
            update_salsa_event_counters(|counters| {
                counters.did_validate_memoized_total =
                    counters.did_validate_memoized_total.saturating_add(1);
                match kind {
                    SalsaEventKeyKind::TypeIndex => {
                        counters.did_validate_memoized_type_index =
                            counters.did_validate_memoized_type_index.saturating_add(1);
                    }
                    SalsaEventKeyKind::ParseResult => {
                        counters.did_validate_memoized_parse_result = counters
                            .did_validate_memoized_parse_result
                            .saturating_add(1);
                    }
                    SalsaEventKeyKind::Other => {}
                }
            });
        }
        salsa::EventKind::WillCheckCancellation => {
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::WillCheckCancellation,
                Some(SalsaEventTimelineEventKind::WillCheckCancellation),
            );
            update_salsa_event_counters(|counters| {
                counters.will_check_cancellation_total =
                    counters.will_check_cancellation_total.saturating_add(1);
            });
        }
        salsa::EventKind::WillIterateCycle { .. } => {
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::WillIterateCycle,
                Some(SalsaEventTimelineEventKind::WillIterateCycle),
            );
            update_salsa_event_counters(|counters| {
                counters.will_iterate_cycle_total =
                    counters.will_iterate_cycle_total.saturating_add(1);
            });
        }
        salsa::EventKind::DidSetCancellationFlag => {
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::Generic,
                Some(SalsaEventTimelineEventKind::DidSetCancellationFlag),
            );
            update_salsa_event_counters(|counters| {
                counters.did_set_cancellation_flag_total =
                    counters.did_set_cancellation_flag_total.saturating_add(1);
            });
            ANALYSIS_V2_GLOBAL_DID_SET_CANCELLATION_FLAG_TOTAL.fetch_add(1, Ordering::Relaxed);
        }
        salsa::EventKind::DidDiscard { .. } => {
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::Generic,
                Some(SalsaEventTimelineEventKind::DidDiscard),
            );
            update_salsa_event_counters(|counters| {
                counters.did_discard_total = counters.did_discard_total.saturating_add(1);
            });
        }
        salsa::EventKind::DidDiscardAccumulated { .. } => {
            record_salsa_event_timeline_marker(
                SalsaEventTimelineMarker::Generic,
                Some(SalsaEventTimelineEventKind::DidDiscardAccumulated),
            );
            update_salsa_event_counters(|counters| {
                counters.did_discard_accumulated_total =
                    counters.did_discard_accumulated_total.saturating_add(1);
            });
        }
        _ => {}
    }
}

