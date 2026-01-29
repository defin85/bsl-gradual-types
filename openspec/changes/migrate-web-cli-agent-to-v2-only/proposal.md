# Предложение изменений: v2-only для Web API + CLI + bsl-agent (без альтернативных inference путей)

## Контекст и проблема

Change `refactor-type-inference-v2-only` привёл IDE/LSP путь к v2 pipeline (`bsl-analysis-v2`) и убрал legacy inference в hot path.
Однако в репозитории остаются альтернативные пути, которые обходят v2 pipeline для non-LSP клиентов:

- Web API использует `TypeInferenceService` для операций с типами/DTO.
- CLI использует `TypeInferenceService` для type completions/type details, а также использует `AnalysisEngine` (legacy фасад) для части анализа.
- `bsl-agent` использует `TypeInferenceService` для web-like операций (по deps snapshot).

Это нарушает требование "v2-only" для всего продукта (не только IDE), создаёт риск расхождений и усложняет развитие правил.

## Цель (конечное состояние)

1) Web API, CLI и `bsl-agent` используют v2-only архитектуру:
   - все операции, которые являются "type inference" / "semantic analysis", выполняются через `bsl-analysis-v2`,
     используя единый deps snapshot (`SemanticDeps`) и единый слой позиционирования (`bsl-line-index`).
2) В дереве кода отсутствуют альтернативные inference пути, которые не проходят через v2 pipeline:
   - `TypeInferenceService` не используется (и при возможности удалён как лишний фасад),
   - CLI не использует `bsl_shared::engine::AnalysisEngine` для вычисления семантики/диагностик.
3) Поведение внешних интерфейсов (HTTP API, CLI команды, MCP/agent endpoints) сохраняется,
   кроме случаев, где legacy поведение явно противоречит v2-only контракту.

## Что меняется

- Web API handlers переводятся на v2-only источники данных (deps snapshot + v2 helpers), без `TypeInferenceService`.
- CLI команды переводятся на v2-only:
  - type completions/type details: через v2-only helpers (deps snapshot),
  - анализ/диагностики: через `AnalysisHostV2`/`AnalysisV2`, без `AnalysisEngine`.
- `bsl-agent` переводится на v2-only аналогично Web API (deps snapshot + v2 helpers).
- Добавляются тесты/проверки, что в этих крейтах нет использования legacy фасадов.

## Не цели

- Переписывать публичный формат Web API (контракты/DTO) без необходимости.
- Оптимизировать производительность, кроме устранения очевидных лишних проходов/дублирования.
- Дорабатывать правила вывода типов (фокус на унификации источника истины).

## Влияние и риски

- Возможны мелкие регрессии поведения Web API/CLI из-за удаления legacy логики (если она была неэквивалентна v2).
- Нужно аккуратно сохранить совместимость выходных DTO и текстовых форматов CLI.
- Потребуется переосмыслить ответственность "поиск типов / детали типов": это не IR-инференс,
  но сейчас реализовано через отдельный сервис (который выглядит как альтернативный слой).

## Критерии готовности

- В `backend/`, `cli/`, `bsl-agent/` нет использования `TypeInferenceService`.
- В `cli/` нет использования `bsl_shared::engine::AnalysisEngine` для семантики/диагностик.
- Все релевантные тесты проходят (`cargo test -p bsl-backend -p bsl-cli -p bsl-agent`).
- `rg -n "TypeInferenceService\\b|bsl_shared::engine::AnalysisEngine\\b" -S backend/ cli/ bsl-agent/` не находит матчей.
