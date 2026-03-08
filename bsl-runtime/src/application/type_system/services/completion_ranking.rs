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
    pub member_identity: Option<String>,
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
    pub member_identity: Option<String>,
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
            member_identity: candidate.member_identity,
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
            .then_with(|| a.member_identity.cmp(&b.member_identity))
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
    let member_identity = candidate
        .member_identity
        .clone()
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{}|{:?}|{}|{}|{}",
        candidate.label_lower, candidate.item.kind, scope, owner, member_identity
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
    if best.member_identity.is_none() {
        best.member_identity = other.member_identity;
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
    if a.signals.member_access && b.signals.member_access {
        return a
            .label_lower
            .cmp(&b.label_lower)
            .then_with(|| a.item.label.cmp(&b.item.label))
            .then_with(|| kind_rank(a.item.kind).cmp(&kind_rank(b.item.kind)))
            .then_with(|| scope_rank(a.scope).cmp(&scope_rank(b.scope)))
            .then_with(|| a.owner_type.cmp(&b.owner_type))
            .then_with(|| a.member_identity.cmp(&b.member_identity))
            .then_with(|| a.source_priority.cmp(&b.source_priority))
            .then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
    }

    b.score
        .partial_cmp(&a.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.source_priority.cmp(&b.source_priority))
        .then_with(|| a.label_lower.cmp(&b.label_lower))
        .then_with(|| kind_rank(a.item.kind).cmp(&kind_rank(b.item.kind)))
        .then_with(|| scope_rank(a.scope).cmp(&scope_rank(b.scope)))
        .then_with(|| a.owner_type.cmp(&b.owner_type))
        .then_with(|| a.member_identity.cmp(&b.member_identity))
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
#[path = "completion_ranking/tests.rs"]
mod tests;
