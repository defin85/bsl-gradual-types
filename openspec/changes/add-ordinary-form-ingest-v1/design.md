## Context

`onec-formbin` owns raw-first ordinary-form decoding, editable workspace export, and safe
write-back. `bsl-gradual-types` owns `TypeRepository`, type resolution, and canonical semantic
runtime behavior.

Для интеграции ordinary forms нужен жёсткий boundary: `bsl-gradual-types` ingest-ит только
documented external bundle contract, а не Python API или внутренние AST conventions
`onec-formbin`.

Producer-side contract сейчас фиксируется как `ordinary-form-bundle.v1` в
`onec-formbin/docs/workspace-contract.md`.

## Goals / Non-Goals

- Goals:
  - принимать ordinary forms через versioned external bundle input;
  - materialize-ить additive synthetic types для form context inside `TypeRepository`;
  - сохранять provenance/uncertainty как явную runtime metadata, а не скрытую эвристику;
  - fail-closed обрабатывать unsupported bundle versions и missing mandatory artifacts.
- Non-Goals:
  - не переписывать producer-side parsing/write-back logic;
  - не требовать raw `Form.bin` parsing в runtime `bsl-gradual-types`;
  - не объявлять producer bundle canonical source of truth для platform/config types;
  - не реализовывать в этом change всю IDE/UI wiring поверх новых типов.

## Decisions

### 1. External boundary = `ordinary-form-bundle.v1`

Importer binds to the versioned external directory bundle `ordinary-form-bundle.v1`, whose source
of truth lives in `onec-formbin/docs/workspace-contract.md`.

Version and schema checks must be explicit. Unsupported bundle versions or missing mandatory
artifacts fail closed with deterministic diagnostics instead of best-effort guesses.

### 2. Import only documented bundle artifacts, not producer internals

The importer must consume only the documented bundle files and file-local schemas.

Preferred downstream entrypoint:
- `support/integration.json`, when present for the negotiated bundle version.

Documented additive inputs:
- `semantic/form.meta.json`
- `semantic/events.json`
- `semantic/commands.json`
- `semantic/attributes.json`
- `semantic/controls.tree.json`
- `semantic/layout.json`
- `semantic/strings.json`
- `support/uncertainty.json`
- `support/capabilities.json`

The importer must not require private Python objects, hidden AST traversal rules, or undocumented
workspace files. `support/provenance.json` may be stored or surfaced as evidence metadata, but it
must not become the semantic source of truth for runtime type resolution.

### 3. Canonical semantics stay inside `TypeRepository`

Imported bundle data is normalized into an internal ordinary-form snapshot and then materialized
into canonical runtime structures.

Where the bundle contains enough identity/binding data, the runtime should be able to build
synthetic types such as:
- `Формы.<...>`
- `ДанныеФормыОбъект.<...>`
- `ЭлементыФормы.<...>`

These types remain runtime-owned artifacts of `bsl-gradual-types`, not borrowed external types.

### 4. Ordinary-form semantics are additive and uncertainty-aware

Imported ordinary-form semantics augment platform/config metadata in form-related contexts. They do
not replace authoritative metadata truth for applied objects, platform types, or non-form module
contexts.

If the external bundle marks a region as `bridge`, `read_only`, `unsupported`, or otherwise
uncertain, `bsl-gradual-types` must preserve that distinction in its internal metadata instead of
silently upgrading it to exact platform/config truth.

## Alternatives Considered

### 1. Parse `Form.bin` directly inside `bsl-gradual-types`

Rejected. This duplicates low-level codec ownership and pulls reverse-engineering concerns into the
canonical semantic runtime before the external contract is stable.

### 2. Bind directly to `onec-formbin` Python API

Rejected. That creates a fragile cross-repo coupling and bypasses the versioned contract boundary.

### 3. Consume raw/AST artifacts as the primary ingest surface

Rejected. Raw and AST layers are useful evidence and write-back anchors, but the runtime needs a
documented normalized ingest surface rather than producer-specific bridge details.

## Risks / Trade-offs

- Producer contract churn may force importer updates while the bundle is still stabilizing.
- Overfitting importer logic to bridge-era semantics can freeze temporary heuristics into runtime.
- Synthetic type materialization may blur ownership boundaries if uncertainty is not preserved.
- Partial bundles may tempt best-effort fallback behavior that lies about certainty.

## Mitigations

- Keep the input versioned and fail closed on unsupported contract versions.
- Treat `support/integration.json` and `support/uncertainty.json` as first-class contract inputs.
- Preserve provenance/uncertainty separately from canonical type definitions.
- Keep ordinary-form ingest additive and scoped to form-related contexts.
