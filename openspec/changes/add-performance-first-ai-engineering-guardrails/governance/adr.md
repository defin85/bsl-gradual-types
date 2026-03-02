# ADR: Performance-first guardrails для AI-assisted non-MVP изменений

## Status
accepted

## Change ID and Criticality
- change_id: `add-performance-first-ai-engineering-guardrails`
- change_criticality: `perf_critical`

## Context
Текущие p31 замеры показывают, что latency-only подход недостаточен: есть деградации, объясняемые resource contention и очередями до build-стадий.

## Options Considered
1. Оставить latency-only gates.
2. Ввести Option B: dedicated perf evaluator + versioned schema contract + process gates.

## Decision
Выбрана Option B как единственная архитектура perf-gate.

## Budgets
- Latency: relative ratio + absolute ceilings (`p95/p99`) по `small|large|churn`.
- Resource: allocations, allocated bytes, lock wait, lock contention.

## Validation Plan
- `openspec validate ... --strict`
- `scripts/check-versioned-contracts.py`
- `scripts/check-contract-compatibility-diff.py`
- perf dry-runs на `small|large|churn`

## Rollback / Supersede
При необходимости policy supersede выполняется новым ADR и major bump schema contract.

## Owners and Approvers
- ADR Owner: backend/runtime architecture
- Perf Budget Owner: intellisense-v2 owners
- Contract Owner: contracts/intellisense-perf-gate owners

