## Context
В проекте уже есть активные change для ключевых функциональных пробелов:
- `update-v2-contextual-implicit-variables`;
- `add-v2-valuetable-column-resolution`.

Однако отсутствует единый продуктовый контракт, который определяет "достаточно качественно для продаж" именно по функциональности LSP. В результате приоритеты размываются: часть проблем воспринимается как "частные баги", хотя для покупателя это критерии пригодности продукта.

## Goals / Non-Goals
- Goals:
  - Ввести формальный функциональный GA baseline для IntelliSense/LSP.
  - Привязать readiness к проверяемым сценариям на реальном корпусе (`conf_big`).
  - Зафиксировать консистентность v2 snapshot для ключевых потребителей (diagnostics/hover/completion).
- Non-Goals:
  - Описывать release/deploy/legal требования (они покрываются отдельным `add-sales-readiness-ga`).
  - Расширять scope на новые неключевые IDE-фичи.

## Decisions
- Decision: Базовый acceptance-контур строится на `examples/conf_big` как максимально реалистичном наборе.
  - Why: synthetic snippets полезны, но не гарантируют продажное качество на enterprise-конфигурациях.

- Decision: Для поддерживаемых implicit symbols в валидном контексте FP `UndeclaredVariable` считаются release-blocker.
  - Why: такие FP подрывают доверие к диагностике и ломают основной UX.

- Decision: Typed-row поведение `ТаблицаЗначений` оценивается end-to-end, а не только по внутреннему inference.
  - Why: для пользователя важны completion/hover/diagnostics как единый продуктовый результат.

- Decision: Этот change не дублирует детали уже активных change, а задает верхнеуровневый GA контракт и зависимость от них.
  - Why: избегаем конфликтующих спецификаций при архивировании.

## Risks / Trade-offs
- Риск: слишком строгий baseline может тормозить релизы.
  - Mitigation: ограничить baseline критичными для продаж сценариями; остальное вести как backlog.

- Риск: пересечение формулировок с активными change в `bsl-intellisense-v2`.
  - Mitigation: использовать e2e и quality-oriented требования, а не повторять low-level требования к алгоритмам.

- Риск: нестабильность acceptance при изменении тестовых данных `conf_big`.
  - Mitigation: закрепить минимальный smoke-набор и фиксировать его в change-specific validation docs.

## Migration Plan
1. Утвердить этот change как функциональный контракт GA.
2. Синхронизировать и завершить зависимые change по implicit symbols и value-table columns.
3. Собрать acceptance baseline на `conf_big` и зафиксировать expected diagnostics profile.
4. Использовать baseline как обязательный gate для функциональной готовности релиза.
