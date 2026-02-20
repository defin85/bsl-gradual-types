## 1. Specification
- [x] 1.1 Обновить requirement `Инкрементальность и корректность позиций в v2 pipeline (MUST)` в `bsl-intellisense-v2` с явным first-trigger контрактом.
- [x] 1.2 Добавить requirement про trigger-aware completion parity (`TriggerCharacter='.'` vs `Invoked`) в `bsl-intellisense-v2`.
- [x] 1.3 Добавить requirement про bounded interactive latency + degraded useful fallback в `bsl-intellisense-v2`.
- [x] 1.4 Обновить requirement про VS Code IDE-интеграцию в `bsl-intellisense` (требование к trigger-character completion и user-visible warning при отключённых автотриггерах).
- [x] 1.5 Зафиксировать границу scope: текущий change реализуется как runtime-centric улучшение; полная event-driven rearchitecture ведётся в `refactor-v2-completion-event-driven-pipeline`.
- [x] 1.6 Уточнить observability requirement: обязательный trigger-aware разрез метрик (`trigger mode`, `parity drift`, `member-access terminal-empty`).
- [x] 1.7 Зафиксировать числовые acceptance gates (latency/first-trigger/terminal-empty/parity mismatch) в proposal/design.

## 2. Implementation
- [x] 2.1 Убрать блокирующую зависимость completion от ожидания предыдущей версии текста в hot path `didChange`; внедрить согласованный source-of-truth latest document text/version в LSP адаптере.
- [x] 2.2 Добавить trigger-aware ветвление в completion handler на основе `CompletionParams.context` (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`) с совместимостью для `context=None`.
- [x] 2.3 Реализовать fallback ladder для member-access: stale non-empty cache -> degraded `isIncomplete=true` -> terminal-empty только если контекст реально не даёт кандидатов.
- [x] 2.4 Развязать вычисление member owner hint от строгой зависимости на готовность IR, чтобы `expr.` не деградировал в keyword-only path при transient-cancel.
- [x] 2.5 Добавить проверку в VS Code extension: если effective completion auto-trigger по символам отключён, показать явный warning и путь к исправлению, не меняя пользовательские настройки автоматически.
- [x] 2.6 Добавить observability разрезы: trigger mode, parity drift, member-access terminal-empty, fallback_unavailable.

## 3. Validation
- [x] 3.1 Добавить/обновить контрактные тесты LSP: первый completion после `didChange` в member-access контексте не требует повторного `Ctrl+Space`.
- [x] 3.2 Добавить/обновить тесты parity: `TriggerCharacter='.'` и `Invoked` дают согласованную semantic выдачу на одной ревизии.
- [x] 3.3 Добавить/обновить тесты latency/fallback: под серией `didChange` completion остаётся bounded и не зависает, а member-access не уходит в transient terminal-empty.
- [x] 3.4 Добавить/обновить extension-тест на warning при отключённом `suggestOnTriggerCharacters`.
- [x] 3.5 Добавить/обновить тесты trigger-context обработки (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`, `context=None`).
- [x] 3.6 Проверить acceptance gates на профильном наборе: `p95 <= 300ms`, `p99 <= 800ms`, `first-trigger >= 99%`, `terminal-empty <= 0.5%`, `parity mismatch <= 1%`. (Артефакты: `backend/tests/perf/reports/improve-v2-completion-interactive-reliability-gate.json`, `backend/tests/perf/reports/improve-v2-completion-interactive-reliability-gate.md`)
- [x] 3.7 Прогнать профильные тестовые наборы и `openspec validate improve-v2-completion-interactive-reliability --strict --no-interactive`. (Лог: `backend/tests/perf/reports/improve-v2-completion-interactive-reliability-openspec-validate.log`, запуск: `scripts/validate-v2-completion-gates.sh`)
