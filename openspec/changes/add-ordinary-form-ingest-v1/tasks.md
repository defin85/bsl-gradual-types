## 1. Contract And Capability

- [ ] 1.1 Add capability `ordinary-form-ingest` with `ordinary-form-bundle.v1` as the external
      versioned input boundary and `onec-formbin/docs/workspace-contract.md` as the producer-side
      source of truth for artifact names and schema versions.
- [ ] 1.2 Define fail-closed compatibility rules for unsupported bundle versions, missing
      mandatory artifacts, and incomplete support metadata.

## 2. Import Pipeline

- [ ] 2.1 Introduce an ordinary-form bundle loader that consumes only documented bundle artifacts
      and does not depend on `onec-formbin` Python internals.
- [ ] 2.2 Define the internal normalized ordinary-form snapshot used between ingest and runtime
      materialization.
- [ ] 2.3 Materialize additive synthetic form types and related bindings into `TypeRepository`
      from the normalized snapshot.

## 3. Runtime Semantics Boundary

- [ ] 3.1 Keep imported ordinary-form semantics additive to platform/config metadata rather than a
      replacement for canonical runtime truth.
- [ ] 3.2 Preserve uncertainty/read-only markers from the external bundle as explicit runtime
      metadata for downstream services.

## 4. Validation

- [ ] 4.1 Add tests for version gating, minimal bundle ingest, synthetic type loading, and
      uncertainty propagation.
- [ ] 4.2 Run `openspec validate add-ordinary-form-ingest-v1 --strict --no-interactive`.
