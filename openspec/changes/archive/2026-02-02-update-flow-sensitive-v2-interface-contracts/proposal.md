# Изменение: update-flow-sensitive-v2-interface-contracts

## Why
В текущей реализации flow-sensitive v2 есть несколько несогласованностей контрактов между интерфейсами (Web API / LSP / MCP), которые создают:
- непредсказуемость поведения “по умолчанию”;
- сложности для клиентов (разные имена флагов);
- невозможность надёжно понять по ответу MCP, был ли включён flow-sensitive режим.

Это изменение приводит контракты к однозначному и проверяемому состоянию.

## What Changes
1) **Web API**: для всех flow-sensitive флагов в JSON остаётся **только** вариант `includeFlowSensitive` (camelCase).
   - Вариант `include_flow_sensitive` считается ошибочным и должен приводить к `400 Bad Request`.

2) **LSP (`bsl.getSemanticTree`)**: если поле `include_flow_sensitive` **не передано**, сервер определяет эффективный режим по workspace setting `enableFlowSensitive`.
   - Если поле передано явно, оно имеет приоритет над настройкой.

3) **MCP (bsl-agent)**: ответы инструментов, которые могут работать в flow-sensitive режиме, добавляют явное поле `flow_sensitive_enabled: bool`, отражающее эффективный режим.

## Impact
1) Web API: payload с `include_flow_sensitive` больше не принимается (ожидается `400`).
2) MCP: добавляется новое поле в ответы (сериализация/десериализация клиентов может потребовать обновления).

## Не в scope
- Изменение семантики самого flow-sensitive анализа (CFG, narrowing, null-safety) — только контракты и wiring.
- Унификация именования флагов между MCP и Web API (MCP остаётся snake_case как сегодня).

## План внедрения / миграция
- Обновить серверную валидацию и DTO.
- Обновить тесты контрактов (LSP/Web/MCP).
- Обновить документацию (Web API / MCP) с явным описанием breaking изменений и примерами.

## Зависимости
- Предполагается, что базовый wiring flow-sensitive v2 уже реализован (см. change `integrate-flow-sensitive-v2-wiring`).
