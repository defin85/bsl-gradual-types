## ADDED Requirements

### Requirement: Ordinary-form ingest consumes only the documented versioned bundle contract (MUST)

Система MUST принимать ordinary-form input только через documented external bundle contract
`ordinary-form-bundle.v1`.

Producer-side source of truth for artifact names and bundle-local schema versions MUST быть
`onec-formbin/docs/workspace-contract.md`.

Importer MUST:

- validate bundle version/schema compatibility explicitly;
- use only documented bundle artifacts and file-local schemas;
- fail closed for unsupported bundle versions or missing mandatory artifacts;
- avoid direct dependency on `onec-formbin` Python APIs or undocumented workspace files.

#### Scenario: Supported `ordinary-form-bundle.v1` is accepted through the documented contract
- **GIVEN** importer receives an external bundle that declares `ordinary-form-bundle.v1`
- **AND** the bundle contains the mandatory documented artifacts for that version
- **WHEN** the runtime starts ordinary-form ingest
- **THEN** the importer accepts the bundle through the documented contract boundary
- **AND** ingest does not require direct calls into `onec-formbin` internals

#### Scenario: Unsupported bundle version fails closed
- **GIVEN** importer receives an external bundle with an unsupported bundle version or incompatible
  schema set
- **WHEN** the runtime validates the bundle
- **THEN** ingest fails closed with deterministic diagnostics
- **AND** the runtime does not guess semantics from partially understood artifacts

### Requirement: Ordinary-form ingest materializes runtime-owned synthetic form semantics (MUST)

Система MUST normalize ingested ordinary-form bundle data into runtime-owned structures and MUST
materialize additive synthetic form semantics inside `TypeRepository` rather than treating external
bundle files as the runtime semantic model.

When the bundle contains sufficient identity and binding data, the runtime MUST support synthetic
ordinary-form type materialization for:

- form context types (`Формы.<...>`);
- form data object types (`ДанныеФормыОбъект.<...>`);
- form elements container types (`ЭлементыФормы.<...>`).

The importer MUST NOT require raw payload parsing or producer AST traversal conventions to derive
those runtime-owned types.

#### Scenario: Bundle with form identity and bindings produces synthetic form types
- **GIVEN** importer receives a supported ordinary-form bundle with form identity, attributes, and
  control bindings
- **WHEN** the bundle is ingested into the runtime
- **THEN** `TypeRepository` contains the corresponding synthetic form-related types
- **AND** those types are runtime-owned artifacts rather than borrowed external bundle objects

#### Scenario: Runtime materialization does not depend on raw or AST internals
- **GIVEN** importer receives a supported ordinary-form bundle whose normalized semantic files are
  sufficient for ingest
- **WHEN** synthetic form semantics are materialized
- **THEN** the runtime does not need to parse `raw/form.raw` or `ast/form.ast.json` to build the
  canonical ordinary-form snapshot

### Requirement: Ordinary-form ingest preserves uncertainty and metadata ownership boundaries (MUST)

Система MUST treat ingested ordinary-form semantics as additive and uncertainty-aware.

The runtime MUST:

- preserve uncertainty/read-only/unsupported markers from the external bundle as explicit metadata;
- keep ordinary-form semantics scoped to the relevant form contexts;
- MUST NOT silently upgrade bridge/uncertain bundle data into authoritative platform/config truth;
- MUST NOT let ordinary-form ingest override canonical platform/config metadata outside documented
  form-context rules.

#### Scenario: Bundle uncertainty is preserved instead of promoted to exact truth
- **GIVEN** the external bundle marks part of the ordinary-form model as uncertain or read-only
- **WHEN** the runtime ingests that bundle
- **THEN** the resulting runtime metadata preserves that uncertainty explicitly
- **AND** downstream services can distinguish uncertain ordinary-form semantics from exact truth

#### Scenario: Ordinary-form ingest stays additive to configuration metadata
- **GIVEN** configuration metadata already defines canonical object-level members for the same
  business object
- **AND** the external ordinary-form bundle provides additional form-only semantics
- **WHEN** the runtime materializes ordinary-form ingest results
- **THEN** form-only semantics become available in form-related contexts
- **AND** the existing canonical platform/config metadata is not overridden outside those contexts
