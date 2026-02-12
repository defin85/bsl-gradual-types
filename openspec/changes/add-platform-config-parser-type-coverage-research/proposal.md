# Change: Полное исследование покрытия типов в parser pipeline (platform + configuration)

## Why
Сейчас есть реальные кейсы, где тип существует в платформе, но не резолвится в IntelliSense v2 (пример: `БиблиотекаКартинок`).
Это означает, что где-то в цепочке `парсинг -> нормализация -> индексация -> фасетный lookup` есть потери или несовместимость контрактов.

Без воспроизводимого покрытия по всем типам платформы и конфигурации мы получаем "слепые зоны":
- сложно локализовать корневую причину (parser, mapper, repository, facet lookup),
- трудно оценить масштаб проблемы,
- исправления делаются точечно и не защищают от регрессий.

## What Changes
- Добавить в `bsl-intellisense-v2` требования на обязательный end-to-end аудит покрытия типов для platform/config parser pipeline.
- Зафиксировать обязательную матрицу стадий жизненного цикла типа:
  - `source -> parsed -> normalized -> indexed -> facet-projected -> lookup-resolvable`.
- Зафиксировать обязательную классификацию gaps с указанием первой "падающей" стадии и причины.
- Зафиксировать, что аудит учитывает фасеты как отдельное измерение (а не только факт наличия типа в индексе).
- Зафиксировать минимальный baseline-кейс `БиблиотекаКартинок`, который должен быть явно виден в отчёте.
- Определить выходные артефакты исследования (машиночитаемый отчёт + человекочитаемая аналитика) и правила приоритезации follow-up изменений.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
- Affected code (follow-up implementation scope):
  - platform parser и loader (`bsl-runtime` / `shared` parser paths)
  - configuration metadata parser и type ingestion path
  - `TypeRepository` / `TypeMetadataLookup` / facet projection paths
  - diagnostics/reporting tooling для coverage audit

## Non-Goals
- Немедленное исправление всех обнаруженных gaps в рамках этого change.
- Полная эмуляция runtime-поведения платформы 1С за пределами parser/type-index контрактов.
- Изменение UX-политик hover/completion, не связанных с корневой причиной нерезолва типа.
