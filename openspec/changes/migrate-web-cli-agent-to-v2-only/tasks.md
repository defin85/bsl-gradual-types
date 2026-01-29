## 1. Разведка (code-first)
- [ ] 1.1 Инвентаризировать все места использования `TypeInferenceService` и `AnalysisEngine` в `backend/` (web), `cli/`, `bsl-agent/` (с путями файлов).
- [ ] 1.2 Зафиксировать список публичных Web API эндпоинтов и CLI команд, которые должны сохранить поведение.

## 2. v2-only API для non-LSP клиентов
- [ ] 2.1 Определить/зафиксировать v2-only "helper" слой для операций, которые НЕ требуют IR (поиск типов/детали типов/поиск методов):
  - вход: `Arc<SemanticDeps>` и параметры запроса,
  - выход: те же DTO/структуры, что и сейчас в Web API/CLI/bsl-agent.
- [ ] 2.2 Запретить прямое использование `TypeInferenceService` в этих путях (замена на новый helper слой).

## 3. Миграция Web API
- [ ] 3.1 Перевести `backend/src/presentation/web/*` на v2-only helper слой.
- [ ] 3.2 Добавить минимальные тесты на сохранение формата ответа для ключевых эндпоинтов (или обновить существующие).

## 4. Миграция CLI
- [ ] 4.1 Перевести `cli` команды type completions/type details на v2-only helper слой.
- [ ] 4.2 Перевести `cli` команды анализа/диагностик на `AnalysisHostV2`/`AnalysisV2` (без `AnalysisEngine`).
- [ ] 4.3 Обновить тексты/help/README при необходимости (если архитектурные утверждения стали неверны).

## 5. Миграция bsl-agent
- [ ] 5.1 Перевести `bsl-agent` на v2-only helper слой (без `TypeInferenceService`).
- [ ] 5.2 Добавить/обновить тесты для агента, подтверждающие отсутствие legacy путей.

## 6. Удаление legacy слоя (если возможно без боли)
- [ ] 6.1 Если после миграции нет пользователей, удалить `backend/src/application/type_inference_service.rs` и связанные экспорты.

## 7. Валидация
- [ ] 7.1 `cargo test -p bsl-backend -p bsl-cli -p bsl-agent` проходит.
- [ ] 7.2 `rg -n "TypeInferenceService\\b|bsl_shared::engine::AnalysisEngine\\b" -S backend/ cli/ bsl-agent/` не находит матчей.
- [ ] 7.3 `openspec validate migrate-web-cli-agent-to-v2-only --strict --no-interactive`.
