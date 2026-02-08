## Context
Текущие активные change (`add-tpm-lease-licensing`, `update-v2-contextual-implicit-variables`, `add-v2-valuetable-column-resolution`) закрывают важные продуктовые и технические функции. Для коммерческого запуска не хватает слоя "операционной готовности к продаже":
- четко оформленного buyer-facing пакета документов;
- предсказуемого и проверяемого release-процесса;
- discoverability критичных runtime-настроек в VS Code extension.

Без этого возрастает риск отказов на этапах trial, security review и procurement.

## Goals / Non-Goals
- Goals:
  - Зафиксировать минимальный GA baseline для продаж и enterprise-пилотов.
  - Устранить разрыв между фактическими runtime-настройками и UX/docs расширения.
  - Сделать релизный контур проверяемым и воспроизводимым.
- Non-Goals:
  - Реализовать licensing backend или биллинг.
  - Изменять inference/diagnostics логику в рамках этого change.

## Decisions
- Decision: Ввести отдельную capability `sales-readiness` как продуктовый контракт перед GA.
  - Why: требования выходят за рамки одной подсистемы и включают docs/process/security artifacts.

- Decision: Настройки Runtime Overrides считать частью публичного UX-контракта extension.
  - Why: если пользователь не видит/не понимает override-ключи в Settings UI, runtime-config де-факто недоступен.

- Decision: Release integrity сделать обязательной частью dev-workflow (tag-driven VSIX + checksums + SBOM).
  - Why: это минимальный уровень доверия для enterprise и внутреннего контроля качества поставки.

## Alternatives Considered
- Альтернатива: ограничиться правками README без формальной спецификации.
  - Rejected: не предотвращает повторный дрейф docs/настроек и не задает acceptance-критерии для GA.

- Альтернатива: включить требования продаж в существующую `dev-workflow` без новой capability.
  - Rejected: смешивает инженерные гейты и коммерческий контракт; теряется прозрачность для product/sales.

## Risks / Trade-offs
- Риск: scope creep (слишком широкий "продажный" change).
  - Mitigation: ограничить требования минимально необходимым baseline без внедрения новых runtime-фич.

- Риск: дублирование с `add-tpm-lease-licensing`.
  - Mitigation: явно сослаться на лицензирование как зависимый контракт, не копируя protocol-level детали.

- Риск: рост операционной нагрузки на релиз.
  - Mitigation: автоматизировать проверяемые части (CI checks, artifact generation), оставить ручные sign-off только для бизнес-решений.

## Migration Plan
1. Утвердить этот change и его дельты.
2. Реализовать docs/settings выравнивание и release workflow.
3. Прогнать dry-run релиза на тестовом теге.
4. Обновить go-to-market материалы и включить GA-checklist в релизный процесс.

## Open Questions
- Нужно ли для первого коммерческого релиза делать обязательную цифровую подпись VSIX, или достаточно checksums + SBOM?
- Какой SLA на security-disclosure принимаем для первой версии (`SECURITY.md`)?
