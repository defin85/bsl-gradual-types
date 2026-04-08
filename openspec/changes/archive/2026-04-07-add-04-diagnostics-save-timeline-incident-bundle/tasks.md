## 1. Contract / Design
- [x] 1.1 Зафиксировать request-centric diagnostics save timeline contract и bundle projection в spec delta.
- [x] 1.2 Явно описать grouping между `save_fastlane` first publish и `idle_heavy` follow-up в одном `didSave` refresh cycle.

## 2. Backend
- [x] 2.1 Добавить bounded server-side retention для diagnostics save traces.
- [x] 2.2 Добавить custom request для чтения diagnostics save timeline без raw text и high-cardinality полей.
- [x] 2.3 Привязать trace milestones к реальному diagnostics runtime, а не к derived metrics reconstruction.
- [x] 2.4 Добавить regression на monotonic save-cycle grouping: fastlane и heavy follow-up экспортируются как один refresh cycle.

## 3. VS Code Extension / Bundle
- [x] 3.1 Добавить client-side request/DTO для diagnostics save timeline.
- [x] 3.2 Экспортировать `raw/diagnostics_save_timeline.json` в incident bundle как отдельный authoritative source.
- [x] 3.3 Обновить `summary.md`/`incident.json`, чтобы diagnostics save timeline отображался request-centric и fail-closed деградировал на старом сервере.
- [x] 3.4 Добавить extension tests на `available|unsupported|unavailable` semantics для нового source.

## 4. Validation
- [x] 4.1 Прогнать backend tests для request contract и save-cycle grouping.
- [x] 4.2 Прогнать extension tests для incident bundle summary/raw attachments.
- [x] 4.3 Прогнать `openspec validate add-04-diagnostics-save-timeline-incident-bundle --strict --no-interactive`.
