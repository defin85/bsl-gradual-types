//! Completion ranking - contextual scoring and deduplication (M4).

use crate::application::type_system::services::completion_service::CompletionContext;
use crate::system::SymbolScope;
use bsl_shared::domain::{CompletionItem, CompletionKind};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixMatch {
    Exact,
    StartsWith,
    Contains,
    None,
}

#[derive(Debug, Clone)]
pub struct RankingSignals {
    pub prefix_match: PrefixMatch,
    pub source_priority: u8,
    pub scope: Option<SymbolScope>,
    pub kind: CompletionKind,
    pub member_access: bool,
    pub has_owner: bool,
}

#[derive(Debug, Clone)]
pub struct RankedCandidate {
    pub item: CompletionItem,
    pub owner_type: Option<String>,
    pub label_lower: String,
    pub source_priority: u8,
    pub scope: Option<SymbolScope>,
    pub score: f32,
    pub signals: RankingSignals,
    pub origin_sources: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RankingOutput {
    pub candidates: Vec<RankedCandidate>,
    pub total_candidates: usize,
    pub dedup_removed: usize,
    pub score_samples: Vec<f32>,
    pub summary: RankingSummary,
}

#[derive(Debug, Clone, Default)]
pub struct RankingSummary {
    pub prefix_exact: usize,
    pub prefix_starts: usize,
    pub prefix_contains: usize,
    pub prefix_none: usize,
    pub member_access: usize,
    pub has_owner: usize,
}

#[derive(Debug, Clone)]
pub struct RankingCandidate {
    pub item: CompletionItem,
    pub owner_type: Option<String>,
    pub label_lower: String,
    pub source_priority: u8,
    pub scope: Option<SymbolScope>,
}

#[allow(dead_code)]
pub fn rank_candidates(
    candidates: Vec<RankingCandidate>,
    context: &CompletionContext,
) -> RankingOutput {
    rank_candidates_with_trace(candidates, context, None)
}

pub fn rank_candidates_with_trace(
    mut candidates: Vec<RankingCandidate>,
    context: &CompletionContext,
    request_id: Option<u64>,
) -> RankingOutput {
    let trace_enabled = request_id.is_some();
    let total_candidates = candidates.len();
    let prefix = context.current_word.to_lowercase();
    let member_access = context.member_access;

    let filter_span = if let Some(request_id) = request_id {
        tracing::debug_span!(
            "completion.filter",
            request_id = request_id,
            prefix = %prefix
        )
    } else {
        Span::none()
    };
    let _filter_guard = filter_span.enter();
    let filter_started = if trace_enabled {
        Some(Instant::now())
    } else {
        None
    };

    let mut filtered = Vec::with_capacity(candidates.len());
    for candidate in candidates.drain(..) {
        let prefix_match = match_prefix(&candidate.label_lower, &prefix);
        if !prefix.is_empty() && prefix_match == PrefixMatch::None {
            continue;
        }
        filtered.push(candidate);
    }

    if let (Some(request_id), Some(started)) = (request_id, filter_started) {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            debug!(
                request_id = request_id,
                stage = "filter",
                elapsed_ms = started.elapsed().as_millis(),
                total_candidates = total_candidates,
                filtered = filtered.len()
            );
        } else {
            tracing::info!(
                request_id = request_id,
                stage = "filter",
                elapsed_ms = started.elapsed().as_millis(),
                total_candidates = total_candidates,
                filtered = filtered.len()
            );
        }
    }
    drop(_filter_guard);

    let rank_span = if let Some(request_id) = request_id {
        tracing::debug_span!(
            "completion.rank",
            request_id = request_id,
            filtered = filtered.len()
        )
    } else {
        Span::none()
    };
    let _rank_guard = rank_span.enter();
    let rank_started = if trace_enabled {
        Some(Instant::now())
    } else {
        None
    };

    let mut ranked = Vec::with_capacity(filtered.len());
    for candidate in filtered {
        let prefix_match = match_prefix(&candidate.label_lower, &prefix);
        let signals = RankingSignals {
            prefix_match,
            source_priority: candidate.source_priority,
            scope: candidate.scope,
            kind: candidate.item.kind,
            member_access,
            has_owner: candidate.owner_type.is_some(),
        };
        let score = score_candidate(&signals, &candidate.label_lower);

        ranked.push(RankedCandidate {
            item: candidate.item,
            owner_type: candidate.owner_type,
            label_lower: candidate.label_lower,
            source_priority: candidate.source_priority,
            scope: candidate.scope,
            score,
            signals,
            origin_sources: vec![candidate.source_priority],
        });
    }

    ranked.sort_by(|a, b| {
        a.label_lower
            .cmp(&b.label_lower)
            .then_with(|| kind_rank(a.item.kind).cmp(&kind_rank(b.item.kind)))
            .then_with(|| scope_rank(a.scope).cmp(&scope_rank(b.scope)))
            .then_with(|| a.owner_type.cmp(&b.owner_type))
            .then_with(|| a.source_priority.cmp(&b.source_priority))
    });

    let mut dedup_map: HashMap<String, RankedCandidate> = HashMap::new();
    for candidate in ranked {
        let key = dedup_key(&candidate);
        match dedup_map.remove(&key) {
            Some(existing) => {
                let (best, other) = if is_better(
                    candidate.score,
                    candidate.source_priority,
                    &candidate.label_lower,
                    &existing,
                ) {
                    (candidate, existing)
                } else {
                    (existing, candidate)
                };
                dedup_map.insert(key, merge_candidates(best, other));
            }
            None => {
                dedup_map.insert(key, candidate);
            }
        }
    }

    let mut unique: Vec<RankedCandidate> = dedup_map.into_values().collect();
    unique.sort_by(stable_order);

    let mut summary = RankingSummary::default();
    let score_samples = unique.iter().take(200).map(|item| item.score).collect();

    for candidate in &unique {
        match candidate.signals.prefix_match {
            PrefixMatch::Exact => summary.prefix_exact += 1,
            PrefixMatch::StartsWith => summary.prefix_starts += 1,
            PrefixMatch::Contains => summary.prefix_contains += 1,
            PrefixMatch::None => summary.prefix_none += 1,
        }
        if candidate.signals.member_access {
            summary.member_access += 1;
        }
        if candidate.signals.has_owner {
            summary.has_owner += 1;
        }
    }

    let dedup_removed = total_candidates.saturating_sub(unique.len());
    if let (Some(request_id), Some(started)) = (request_id, rank_started) {
        if tracing::level_filters::STATIC_MAX_LEVEL >= tracing::level_filters::LevelFilter::DEBUG {
            debug!(
                request_id = request_id,
                stage = "rank",
                elapsed_ms = started.elapsed().as_millis(),
                total_candidates = total_candidates,
                dedup_removed = dedup_removed,
                unique = unique.len()
            );
        } else {
            tracing::info!(
                request_id = request_id,
                stage = "rank",
                elapsed_ms = started.elapsed().as_millis(),
                total_candidates = total_candidates,
                dedup_removed = dedup_removed,
                unique = unique.len()
            );
        }
    }

    RankingOutput {
        dedup_removed,
        total_candidates,
        candidates: unique,
        score_samples,
        summary,
    }
}

fn match_prefix(label: &str, prefix: &str) -> PrefixMatch {
    if prefix.is_empty() {
        return PrefixMatch::None;
    }
    if label == prefix {
        return PrefixMatch::Exact;
    }
    if label.starts_with(prefix) {
        return PrefixMatch::StartsWith;
    }
    if label.contains(prefix) {
        return PrefixMatch::Contains;
    }
    PrefixMatch::None
}

fn score_candidate(signals: &RankingSignals, label_lower: &str) -> f32 {
    let mut score = 0.0;

    score += match signals.prefix_match {
        PrefixMatch::Exact => 1.0,
        PrefixMatch::StartsWith => 0.7,
        PrefixMatch::Contains => 0.4,
        PrefixMatch::None => 0.0,
    };

    score += match signals.source_priority {
        0 => 0.20,
        1 => 0.15,
        2 => 0.10,
        3 => 0.05,
        _ => 0.0,
    };

    score += match signals.scope {
        Some(SymbolScope::Local) => 0.15,
        Some(SymbolScope::Module) => 0.10,
        Some(SymbolScope::Global) => 0.05,
        None => 0.0,
    };

    if signals.member_access {
        score += match signals.kind {
            CompletionKind::Method | CompletionKind::Field | CompletionKind::Property => 0.10,
            _ => 0.0,
        };
    }

    if signals.has_owner {
        score += 0.05;
    }

    let length_penalty = (label_lower.chars().count() as f32 / 100.0).min(0.05);
    score = (score - length_penalty).max(0.0);

    score.min(1.0)
}

fn dedup_key(candidate: &RankedCandidate) -> String {
    let scope = candidate
        .scope
        .map(|scope| format!("{:?}", scope))
        .unwrap_or_else(|| "none".to_string());
    let owner = candidate
        .owner_type
        .as_deref()
        .map(|owner| owner.to_lowercase())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{}|{:?}|{}|{}",
        candidate.label_lower, candidate.item.kind, scope, owner
    )
}

fn merge_candidates(mut best: RankedCandidate, other: RankedCandidate) -> RankedCandidate {
    if best.item.detail.is_none() {
        best.item.detail = other.item.detail;
    }
    if best.item.documentation.is_none() {
        best.item.documentation = other.item.documentation;
    }
    if best.item.insert_text.is_none() {
        best.item.insert_text = other.item.insert_text;
    }
    if best.item.filter_text.is_none() {
        best.item.filter_text = other.item.filter_text;
    }
    if best.item.sort_text.is_none() {
        best.item.sort_text = other.item.sort_text;
    }
    if best.owner_type.is_none() {
        best.owner_type = other.owner_type;
    }

    best.origin_sources = merge_sources(&best.origin_sources, &other.origin_sources);

    best
}

fn merge_sources(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut merged = Vec::with_capacity(left.len() + right.len());
    merged.extend_from_slice(left);
    merged.extend_from_slice(right);
    merged.sort_unstable();
    merged.dedup();
    merged
}

fn stable_order(a: &RankedCandidate, b: &RankedCandidate) -> std::cmp::Ordering {
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.source_priority.cmp(&b.source_priority))
        .then_with(|| a.label_lower.cmp(&b.label_lower))
        .then_with(|| kind_rank(a.item.kind).cmp(&kind_rank(b.item.kind)))
        .then_with(|| scope_rank(a.scope).cmp(&scope_rank(b.scope)))
        .then_with(|| a.owner_type.cmp(&b.owner_type))
}

fn kind_rank(kind: CompletionKind) -> u8 {
    match kind {
        CompletionKind::Method => 0,
        CompletionKind::Function => 1,
        CompletionKind::Constructor => 2,
        CompletionKind::Field => 3,
        CompletionKind::Property => 4,
        CompletionKind::Variable => 5,
        CompletionKind::Constant => 6,
        CompletionKind::Type | CompletionKind::Class | CompletionKind::Struct => 7,
        CompletionKind::Module | CompletionKind::Global => 8,
        CompletionKind::Keyword => 9,
        _ => 20,
    }
}

fn scope_rank(scope: Option<SymbolScope>) -> u8 {
    match scope {
        Some(SymbolScope::Local) => 0,
        Some(SymbolScope::Module) => 1,
        Some(SymbolScope::Global) => 2,
        None => 3,
    }
}

fn is_better(
    score: f32,
    source_priority: u8,
    label_lower: &str,
    existing: &RankedCandidate,
) -> bool {
    score > existing.score
        || (score == existing.score
            && (source_priority, label_lower)
                < (existing.source_priority, existing.label_lower.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::SymbolScope;
    use bsl_shared::domain::{CompletionItem, CompletionKind};
    use std::cmp::Ordering;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl SharedWriter {
        fn contents(&self) -> String {
            let guard = self.0.lock().expect("lock log buffer");
            String::from_utf8_lossy(&guard).to_string()
        }
    }

    struct SharedWriterGuard(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedWriterGuard {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut guard = self
                .0
                .lock()
                .map_err(|_| std::io::Error::other("log buffer poisoned"))?;
            guard.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for SharedWriter {
        type Writer = SharedWriterGuard;

        fn make_writer(&'a self) -> Self::Writer {
            SharedWriterGuard(self.0.clone())
        }
    }

    fn ctx(prefix: &str, member_access: bool) -> CompletionContext {
        CompletionContext {
            current_word: prefix.to_string(),
            member_access,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        }
    }

    fn candidate(
        label: &str,
        kind: CompletionKind,
        source_priority: u8,
        scope: Option<SymbolScope>,
        owner_type: Option<&str>,
    ) -> RankingCandidate {
        RankingCandidate {
            item: CompletionItem::new(label.to_string(), kind),
            owner_type: owner_type.map(|value| value.to_string()),
            label_lower: label.to_lowercase(),
            source_priority,
            scope,
        }
    }

    fn ranked(
        label: &str,
        kind: CompletionKind,
        source_priority: u8,
        scope: Option<SymbolScope>,
        owner_type: Option<&str>,
        score: f32,
    ) -> RankedCandidate {
        let owner_type = owner_type.map(|value| value.to_string());
        RankedCandidate {
            item: CompletionItem::new(label.to_string(), kind),
            owner_type: owner_type.clone(),
            label_lower: label.to_lowercase(),
            source_priority,
            scope,
            score,
            signals: RankingSignals {
                prefix_match: PrefixMatch::None,
                source_priority,
                scope,
                kind,
                member_access: false,
                has_owner: owner_type.is_some(),
            },
            origin_sources: vec![source_priority],
        }
    }

    #[test]
    fn rank_prefers_exact_prefix() {
        let ctx = CompletionContext {
            current_word: "тест".to_string(),
            member_access: false,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![
            RankingCandidate {
                item: CompletionItem::new("тест".to_string(), CompletionKind::Function),
                owner_type: None,
                label_lower: "тест".to_string(),
                source_priority: 2,
                scope: None,
            },
            RankingCandidate {
                item: CompletionItem::new("тестирование".to_string(), CompletionKind::Function),
                owner_type: None,
                label_lower: "тестирование".to_string(),
                source_priority: 2,
                scope: None,
            },
        ];

        let ranked = rank_candidates(candidates, &ctx);
        assert_eq!(ranked.candidates.first().unwrap().label_lower, "тест");
    }

    #[test]
    fn rank_prefers_local_scope() {
        let ctx = CompletionContext {
            current_word: "a".to_string(),
            member_access: false,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Variable),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 2,
                scope: Some(SymbolScope::Global),
            },
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Variable),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 2,
                scope: Some(SymbolScope::Local),
            },
        ];

        let ranked = rank_candidates(candidates, &ctx);
        assert_eq!(
            ranked.candidates.first().unwrap().scope,
            Some(SymbolScope::Local)
        );
    }

    #[test]
    fn dedup_keeps_best_score() {
        let ctx = CompletionContext {
            current_word: "a".to_string(),
            member_access: false,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Function),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 3,
                scope: Some(SymbolScope::Global),
            },
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Function),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 1,
                scope: Some(SymbolScope::Global),
            },
        ];

        let ranked = rank_candidates(candidates, &ctx);
        assert_eq!(ranked.candidates.len(), 1);
        assert_eq!(ranked.candidates[0].source_priority, 1);
    }

    #[test]
    fn ordering_is_deterministic_for_equal_scores() {
        let ctx = CompletionContext {
            current_word: "a".to_string(),
            member_access: false,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Function),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 2,
                scope: Some(SymbolScope::Global),
            },
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Function),
                owner_type: Some("Owner".to_string()),
                label_lower: "abc".to_string(),
                source_priority: 2,
                scope: Some(SymbolScope::Global),
            },
        ];

        let ranked_first = rank_candidates(candidates.clone(), &ctx);
        let ranked_second = rank_candidates(candidates, &ctx);

        assert_eq!(
            ranked_first.candidates.first().unwrap().owner_type,
            ranked_second.candidates.first().unwrap().owner_type
        );
    }

    #[test]
    fn dedup_merges_details_from_weaker_candidate() {
        let ctx = CompletionContext {
            current_word: "a".to_string(),
            member_access: false,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![
            RankingCandidate {
                item: CompletionItem::new("abc".to_string(), CompletionKind::Function),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 0,
                scope: Some(SymbolScope::Global),
            },
            RankingCandidate {
                item: CompletionItem::with_details(
                    "abc".to_string(),
                    CompletionKind::Function,
                    Some("detail".to_string()),
                    Some("doc".to_string()),
                ),
                owner_type: None,
                label_lower: "abc".to_string(),
                source_priority: 3,
                scope: Some(SymbolScope::Global),
            },
        ];

        let ranked = rank_candidates(candidates, &ctx);
        let item = &ranked.candidates[0].item;
        assert_eq!(ranked.candidates.len(), 1);
        assert_eq!(item.detail.as_deref(), Some("detail"));
        assert_eq!(item.documentation.as_deref(), Some("doc"));
    }

    #[test]
    fn match_prefix_empty_returns_none() {
        assert_eq!(match_prefix("abc", ""), PrefixMatch::None);
    }

    #[test]
    fn match_prefix_variants() {
        assert_eq!(match_prefix("abc", "abc"), PrefixMatch::Exact);
        assert_eq!(match_prefix("abcd", "abc"), PrefixMatch::StartsWith);
        assert_eq!(match_prefix("xabc", "abc"), PrefixMatch::Contains);
        assert_eq!(match_prefix("xyz", "abc"), PrefixMatch::None);
    }

    #[test]
    fn filtering_drops_non_matching_when_prefix_present() {
        let ctx = ctx("ab", false);
        let candidates = vec![
            candidate("ab", CompletionKind::Function, 1, None, None),
            candidate("cab", CompletionKind::Function, 1, None, None),
            candidate("xyz", CompletionKind::Function, 1, None, None),
        ];

        let ranked = rank_candidates(candidates, &ctx);
        let labels: Vec<&str> = ranked
            .candidates
            .iter()
            .map(|item| item.label_lower.as_str())
            .collect();

        assert!(labels.contains(&"ab"));
        assert!(labels.contains(&"cab"));
        assert!(!labels.contains(&"xyz"));
        assert_eq!(ranked.summary.prefix_none, 0);
    }

    #[test]
    fn filtering_keeps_all_when_prefix_empty() {
        let ctx = ctx("", false);
        let candidates = vec![
            candidate("ab", CompletionKind::Function, 1, None, None),
            candidate("xyz", CompletionKind::Function, 1, None, None),
        ];

        let ranked = rank_candidates(candidates, &ctx);
        assert_eq!(ranked.candidates.len(), 2);
        assert_eq!(ranked.summary.prefix_none, 2);
    }

    #[test]
    fn score_candidate_member_access_bonus_only_for_member_kinds() {
        let mut signals = RankingSignals {
            prefix_match: PrefixMatch::StartsWith,
            source_priority: 2,
            scope: Some(SymbolScope::Global),
            kind: CompletionKind::Keyword,
            member_access: true,
            has_owner: false,
        };
        let keyword_score = score_candidate(&signals, "abc");
        signals.kind = CompletionKind::Method;
        let method_score = score_candidate(&signals, "abc");

        assert!(method_score > keyword_score);
    }

    #[test]
    fn score_candidate_clamps_to_one() {
        let signals = RankingSignals {
            prefix_match: PrefixMatch::Exact,
            source_priority: 0,
            scope: Some(SymbolScope::Local),
            kind: CompletionKind::Method,
            member_access: true,
            has_owner: true,
        };
        let score = score_candidate(&signals, "x");
        assert!((score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn score_candidate_does_not_go_negative() {
        let signals = RankingSignals {
            prefix_match: PrefixMatch::None,
            source_priority: 10,
            scope: None,
            kind: CompletionKind::Keyword,
            member_access: false,
            has_owner: false,
        };
        let label = "a".repeat(1000);
        let score = score_candidate(&signals, &label);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn dedup_key_differs_by_scope_kind_and_owner() {
        let base = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Local),
            None,
            0.5,
        );
        let diff_scope = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );
        let diff_kind = ranked(
            "abc",
            CompletionKind::Method,
            1,
            Some(SymbolScope::Local),
            None,
            0.5,
        );
        let diff_owner = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Local),
            Some("Owner"),
            0.5,
        );

        assert_ne!(dedup_key(&base), dedup_key(&diff_scope));
        assert_ne!(dedup_key(&base), dedup_key(&diff_kind));
        assert_ne!(dedup_key(&base), dedup_key(&diff_owner));
    }

    #[test]
    fn rank_keeps_candidates_with_different_owner_types() {
        let ctx = CompletionContext {
            current_word: "a".to_string(),
            member_access: true,
            member_base: None,
            trigger_char: Some('.'),
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![
            candidate(
                "abc",
                CompletionKind::Property,
                1,
                Some(SymbolScope::Global),
                Some("TypeA"),
            ),
            candidate(
                "abc",
                CompletionKind::Property,
                1,
                Some(SymbolScope::Global),
                Some("TypeB"),
            ),
        ];

        let ranked = rank_candidates(candidates, &ctx);
        assert_eq!(ranked.candidates.len(), 2);
    }

    #[test]
    fn merge_sources_dedup_and_sort() {
        let merged = merge_sources(&[3, 1, 1], &[2, 3]);
        assert_eq!(merged, vec![1, 2, 3]);
    }

    #[test]
    fn stable_order_prefers_lower_source_priority() {
        let a = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );
        let b = ranked(
            "abc",
            CompletionKind::Function,
            2,
            Some(SymbolScope::Global),
            None,
            0.5,
        );

        assert_eq!(stable_order(&a, &b), Ordering::Less);
    }

    #[test]
    fn stable_order_prefers_lexicographic_label() {
        let a = ranked(
            "aaa",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );
        let b = ranked(
            "bbb",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );

        assert_eq!(stable_order(&a, &b), Ordering::Less);
    }

    #[test]
    fn stable_order_prefers_kind_rank() {
        let a = ranked(
            "abc",
            CompletionKind::Method,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );
        let b = ranked(
            "abc",
            CompletionKind::Keyword,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );

        assert_eq!(stable_order(&a, &b), Ordering::Less);
    }

    #[test]
    fn stable_order_prefers_scope_rank() {
        let a = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Local),
            None,
            0.5,
        );
        let b = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );

        assert_eq!(stable_order(&a, &b), Ordering::Less);
    }

    #[test]
    fn stable_order_prefers_none_owner_type() {
        let a = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            None,
            0.5,
        );
        let b = ranked(
            "abc",
            CompletionKind::Function,
            1,
            Some(SymbolScope::Global),
            Some("Owner"),
            0.5,
        );

        assert_eq!(stable_order(&a, &b), Ordering::Less);
    }

    #[test]
    fn summary_counts_prefix_member_and_owner() {
        let ctx = ctx("ab", true);
        let candidates = vec![
            candidate("ab", CompletionKind::Function, 1, None, None),
            candidate("abx", CompletionKind::Function, 1, None, None),
            candidate("cab", CompletionKind::Function, 1, None, Some("Owner")),
        ];

        let ranked = rank_candidates(candidates, &ctx);

        assert_eq!(ranked.summary.prefix_exact, 1);
        assert_eq!(ranked.summary.prefix_starts, 1);
        assert_eq!(ranked.summary.prefix_contains, 1);
        assert_eq!(ranked.summary.prefix_none, 0);
        assert_eq!(ranked.summary.member_access, 3);
        assert_eq!(ranked.summary.has_owner, 1);
    }

    #[test]
    fn rank_candidates_emits_trace_logs_with_request_id() {
        let ctx = CompletionContext {
            current_word: "a".to_string(),
            member_access: false,
            member_base: None,
            trigger_char: None,
            can_add_statements: true,
            expects_type: false,
            can_add_functions: true,
        };

        let candidates = vec![RankingCandidate {
            item: CompletionItem::new("abc".to_string(), CompletionKind::Function),
            owner_type: None,
            label_lower: "abc".to_string(),
            source_priority: 2,
            scope: Some(SymbolScope::Global),
        }];

        let writer = SharedWriter::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(writer.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            let _ = rank_candidates_with_trace(candidates, &ctx, Some(7));
        });

        let output = writer.contents();
        assert!(
            output.contains("request_id=7"),
            "expected request_id in trace logs: {}",
            output
        );
        assert!(
            output.contains("stage=\"filter\"") || output.contains("stage=filter"),
            "expected filter stage in trace logs: {}",
            output
        );
        assert!(
            output.contains("stage=\"rank\"") || output.contains("stage=rank"),
            "expected rank stage in trace logs: {}",
            output
        );
    }
}
