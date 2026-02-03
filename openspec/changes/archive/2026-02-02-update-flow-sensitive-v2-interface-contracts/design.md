# Дизайн: update-flow-sensitive-v2-interface-contracts

## 1) Web API: только `includeFlowSensitive`
Цель — исключить двусмысленность и “тихие” несовпадения между endpoints.

### Контракт
- Сервер принимает **только** `includeFlowSensitive` (camelCase) в JSON body.
- Любое присутствие ключа `include_flow_sensitive` в JSON должно приводить к `400 Bad Request`.

### Мотивация
“Тихое игнорирование” старого ключа опасно: клиент думает, что включил flow-sensitive, но получает базовый режим.

## 2) LSP: `bsl.getSemanticTree` подчиняется `enableFlowSensitive` при отсутствии поля
### Контракт
- `include_flow_sensitive` в запросе `bsl.getSemanticTree` становится `Option<bool>`.
- Если `None`, effective = workspace `enableFlowSensitive`.
- Если `Some(v)`, effective = `v` (override).

### Мотивация
Стабилизировать поведение “по умолчанию” и убрать особый случай, когда semantic-tree неожиданно включает flow-sensitive без согласования с настройкой workspace.

## 3) MCP: явное поле `flow_sensitive_enabled` в ответах
### Контракт
Добавляется `flow_sensitive_enabled: bool` в ответы инструментов:
- `bsl_type_at_position_start`
- `bsl_members_start`
- `bsl_diagnostics_start`

Поле отражает effective режим для конкретного вызова (в первую очередь — `include_flow_sensitive` входного запроса).

### Мотивация
Клиенты должны уметь различать:
- “режим выключен” vs “режим включён, но narrowing не применился”.

## Совместимость
Изменение намеренно breaking для Web API:
- старый ключ должен приводить к ошибке и заставить клиента обновиться.
