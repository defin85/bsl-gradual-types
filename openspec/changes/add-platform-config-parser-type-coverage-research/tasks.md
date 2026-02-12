## 1. Specification & Design
- [ ] 1.1 Добавить spec delta в `bsl-intellisense-v2` для end-to-end аудита покрытия типов platform/config.
- [ ] 1.2 Зафиксировать единую модель стадий type lifecycle и формат gap-классификации.
- [ ] 1.3 Зафиксировать обязательный facet-aware срез (по фасетам и контекстам модулей).

## 2. Inventory Baseline
- [ ] 2.1 Сформировать полный baseline-список типов из platform source (парсер платформы) и configuration metadata.
- [ ] 2.2 Сформировать baseline-список типов, доступных в `TypeRepository` после ingestion/normalization.
- [ ] 2.3 Добавить машиночитаемую матрицу покрытия: `source -> parsed -> normalized -> indexed -> lookup`.

## 3. Gap Analysis
- [ ] 3.1 Выявить типы, теряющиеся на каждой стадии pipeline, и зафиксировать first-failed-stage + reason.
- [ ] 3.2 Провести facet-aware анализ (тип есть в индексе, но нерезолвится в конкретном фасете/контексте).
- [ ] 3.3 Зафиксировать отдельный baseline-кейс `БиблиотекаКартинок` с полной трассировкой по стадиям.

## 4. Deliverables & Follow-up
- [ ] 4.1 Подготовить человекочитаемый отчёт исследования с метриками покрытия и top-gap списком.
- [ ] 4.2 Подготовить приоритезированный backlog follow-up changes (по корневым причинам и impact).
- [ ] 4.3 Зафиксировать regression-набор целевых кейсов для контроля исправлений.

## 5. Validation
- [ ] 5.1 `openspec validate add-platform-config-parser-type-coverage-research --strict --no-interactive`
- [ ] 5.2 Провести review change с владельцами parser/metadata/facet-lookup подсистем.
