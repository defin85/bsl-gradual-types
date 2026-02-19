## 1. Specification
- [ ] 1.1 Добавить в `dev-workflow` requirement: versioned contracts обязаны храниться в `contracts/**` с явным номером версии в пути.
- [ ] 1.2 Добавить в `dev-workflow` requirement: breaking изменения контракта требуют version bump и миграционную заметку.
- [ ] 1.3 Добавить в `bsl-intellisense-v2` requirement: интерактивный completion v2 должен иметь versioned contract для LSP completion surface и observability labels.

## 2. Design
- [ ] 2.1 Зафиксировать целевую структуру `contracts/**` (surface -> version -> schema/examples/changelog).
- [ ] 2.2 Зафиксировать compatibility policy (breaking/non-breaking) и правила versioning (`v1`, `v2`, ...).
- [ ] 2.3 Зафиксировать rollout policy: как внедрять contracts поэтапно без блокировки текущих change.

## 3. Implementation (follow-up)
- [ ] 3.1 Создать baseline contracts для completion v2 (минимум: trigger context, completion outcome, degraded semantics).
- [ ] 3.2 Создать baseline contracts для observability completion v2 (минимум: trigger mode/parity drift/member-access terminal-empty/fallback_unavailable).
- [ ] 3.3 Добавить CI-проверки (schema validation + compatibility gate по version policy).

## 4. Validation
- [ ] 4.1 `openspec validate add-versioned-contracts-layer --strict --no-interactive`.
- [ ] 4.2 Провести архитектурный review с владельцами backend/runtime/extension.
