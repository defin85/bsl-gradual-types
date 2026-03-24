---
name: openspec-delivery-matrix
description: Build a Requirement -> Artifact -> Validation matrix for an approved OpenSpec change before handoff. Use when preparing delivery evidence or checking traceability for requirements in `openspec/changes/<change-id>`.
---

# OpenSpec Delivery Matrix

Используй этот skill перед handoff для OpenSpec change.

## Steps

1. Открой `openspec/changes/<change-id>/specs/**/spec.md`.
2. Для каждого MUST собери строку вида `Requirement -> Artifact -> Validation`.
3. Artifact фиксируй только concrete repo paths.
4. Validation фиксируй только проверками, которые реально запускались или versioned evidence artifacts.
5. Если для requirement нет validation, handoff блокируется.

## Output Contract

Минимальный формат:

```text
Requirement -> Artifact -> Validation
```

Опорный документ: `docs/agent/task-artifacts.md`.
