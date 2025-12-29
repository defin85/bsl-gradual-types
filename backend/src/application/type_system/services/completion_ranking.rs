//! Completion ranking - contextual scoring and deduplication (M4).

use crate::application::type_system::services::completion_service::CompletionContext;
use crate::system::SymbolScope;
use bsl_shared::domain::{CompletionItem, CompletionKind};
use std::collections::HashMap;

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

pub fn rank_candidates(
    mut candidates: Vec<RankingCandidate>,
    context: &CompletionContext,
) -> RankingOutput {
    let total_candidates = candidates.len();
    let prefix = context.current_word.to_lowercase();
    let member_access = context.member_access;

    let mut ranked = Vec::with_capacity(candidates.len());
    for candidate in candidates.drain(..) {
        let prefix_match = match_prefix(&candidate.label_lower, &prefix);
        if !prefix.is_empty() && prefix_match == PrefixMatch::None {
            continue;
        }

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
        match dedup_map.get(&key) {
            Some(existing)
                if is_better(
                    candidate.score,
                    candidate.source_priority,
                    &candidate.label_lower,
                    existing,
                ) =>
            {
                dedup_map.insert(key, candidate);
            }
            None => {
                dedup_map.insert(key, candidate);
            }
            _ => {}
        }
    }

    let mut unique: Vec<RankedCandidate> = dedup_map.into_values().collect();
    unique.sort_by(|a, b| stable_order(a, b));

    let mut summary = RankingSummary::default();
    let score_samples = unique
        .iter()
        .take(200)
        .map(|item| item.score)
        .collect();

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

    RankingOutput {
        dedup_removed: total_candidates.saturating_sub(unique.len()),
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
    let owner = candidate.owner_type.clone().unwrap_or_default();
    format!(
        "{}|{:?}|{}|{}",
        candidate.label_lower, candidate.item.kind, scope, owner
    )
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
}
