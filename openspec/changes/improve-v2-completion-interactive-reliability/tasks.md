## 1. Specification
- [ ] 1.1 Обновить requirement `Инкрементальность и корректность позиций в v2 pipeline (MUST)` в `bsl-intellisense-v2` с явным first-trigger контрактом.
- [ ] 1.2 Добавить requirement про trigger-aware completion parity (`TriggerCharacter='.'` vs `Invoked`) в `bsl-intellisense-v2`.
- [ ] 1.3 Добавить requirement про bounded interactive latency + degraded useful fallback в `bsl-intellisense-v2`.
- [ ] 1.4 Обновить requirement про VS Code IDE-интеграцию в `bsl-intellisense` (требование к trigger-character completion и user-visible warning при отключённых автотриггерах).

## 2. Implementation
- [ ] 2.1 Убрать блокирующую зависимость completion от ожидания предыдущей версии текста в hot path `didChange`/completion; ввести согласованный source-of-truth для latest document text/version в LSP адаптере.
- [ ] 2.2 Добавить trigger-aware ветвление в completion handler на основе `CompletionParams.context` (`TriggerCharacter`, `Invoked`, `TriggerForIncompleteCompletions`).
- [ ] 2.3 Гарантировать полезный первый ответ в member-access контексте при transient stale state: non-empty candidates или явно деградированный `isIncomplete=true` fallback вместо terminal-empty ответа.
- [ ] 2.4 Развязать вычисление member owner hint от строгой зависимости на готовность IR, чтобы `expr.` не деградировал в keyword-only path при transient-cancel.
- [ ] 2.5 Добавить проверку в VS Code extension: если effective completion auto-trigger по символам отключён, показать явный warning и путь к исправлению, не меняя пользовательские настройки автоматически.

## 3. Validation
- [ ] 3.1 Добавить/обновить контрактные тесты LSP: первый completion после `didChange` в member-access контексте не требует повторного `Ctrl+Space`.
- [ ] 3.2 Добавить/обновить тесты parity: `TriggerCharacter='.'` и `Invoked` дают согласованную semantic выдачу на одной ревизии.
- [ ] 3.3 Добавить/обновить тесты latency/fallback: под серией `didChange` completion остаётся bounded и не зависает.
- [ ] 3.4 Добавить/обновить extension-тест на warning при отключённом `suggestOnTriggerCharacters`.
- [ ] 3.5 Прогнать профильные тестовые наборы и `openspec validate improve-v2-completion-interactive-reliability --strict --no-interactive`.
