# Change: add ordinary form ingest v1

## Why

Нам нужен стабильный путь от ordinary-form reverse-engineering к canonical type/runtime pipeline без
дублирования semantic engine внутри `bsl-gradual-types`.

`onec-formbin` уже движется к agent-editable raw-first workspace для `Form.bin` и фиксирует
публичный versioned bundle contract `ordinary-form-bundle.v1` в `docs/workspace-contract.md`.
`bsl-gradual-types` должен уметь ingest-ить этот bundle как внешний input, materialize-ить
ordinary-form semantics в `TypeRepository`, и при этом не зависеть от внутренних Python/AST
конвенций `onec-formbin`.

## What Changes

- Add new capability `ordinary-form-ingest`.
- Define `ordinary-form-bundle.v1` from `onec-formbin/docs/workspace-contract.md` as the external
  versioned input boundary for ordinary forms.
- Require an importer that consumes the documented bundle contract, materializes a normalized
  ordinary-form snapshot, and loads additive synthetic form types into the canonical runtime.
- Keep uncertainty, read-only zones, and ownership boundaries explicit so ordinary-form ingest does
  not silently override platform/config metadata truth.

## Sequence

Этот change опирается на существующую целевую архитектуру с `TypeRepository` как single source of
truth и продолжает form/type roadmap, где ordinary-form данные должны входить в runtime через
synthetic types, а не через отдельный side semantic engine.

## External Input

- Producer repository: `onec-formbin`
- External contract id: `ordinary-form-bundle.v1`
- Source of truth for artifact names and schemas:
  - `onec-formbin/docs/workspace-contract.md`

## Impact

- Affected specs:
  - `ordinary-form-ingest`
- Affected code:
  - future ordinary-form bundle importer under the configuration/form loading path
  - `TypeRepository` loading/materialization for synthetic form types
  - form-module context seeding and related runtime resolution surfaces

## Non-Goals

- Do not parse raw `Form.bin` directly inside `bsl-gradual-types` in this change.
- Do not rewrite `onec-formbin` in Rust as part of this proposal.
- Do not treat the external bundle as a second canonical type system.
- Do not let ingested ordinary-form data override authoritative platform/config metadata outside
  documented form-context rules.
