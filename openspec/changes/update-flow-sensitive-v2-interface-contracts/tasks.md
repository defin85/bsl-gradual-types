## 1. Web API (унификация флага)
- [x] 1.1 Привести все Web API запросы к единому JSON флагу `includeFlowSensitive` (camelCase).
- [x] 1.2 Явно отклонять payload с `include_flow_sensitive` (возвращать `400 Bad Request` с понятным сообщением).
- [x] 1.3 Обновить Web API smoke/интеграционные тесты на новую форму запроса.
- [x] 1.4 Обновить документацию Web API (если есть) и dev guide примеры.

## 2. LSP (`bsl.getSemanticTree`)
- [x] 2.1 Сделать `include_flow_sensitive` опциональным параметром запроса.
- [x] 2.2 Если параметр отсутствует — подчинять поведение workspace setting `enableFlowSensitive`.
- [x] 2.3 Если параметр задан явно — он имеет приоритет над `enableFlowSensitive`.
- [x] 2.4 Добавить/обновить тесты на приоритет и default поведение.

## 3. MCP (bsl-agent)
- [x] 3.1 Добавить `flow_sensitive_enabled: bool` в ответы `bsl_type_at_position_start`, `bsl_members_start`, `bsl_diagnostics_start`.
- [x] 3.2 Убедиться, что поле отражает эффективный режим (учитывает переданный `include_flow_sensitive`).
- [x] 3.3 Обновить stdio integration тесты на проверку нового поля.

## 4. Регрессии и документация
- [x] 4.1 Обновить changelog/заметки (если применимо) с перечислением breaking изменений.
- [x] 4.2 Прогнать `openspec validate update-flow-sensitive-v2-interface-contracts --strict --no-interactive`.
- [x] 4.3 Прогнать `cargo test --workspace` (после реализации).
