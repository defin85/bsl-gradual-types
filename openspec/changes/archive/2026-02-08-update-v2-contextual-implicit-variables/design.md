## Context
В текущем v2 pipeline implicit-переменные модульного контекста частично инициализируются и не синхронизированы между AST→IR и type inference. На реальных формах (`conf_big`) это приводит к FP-ошибкам `Необъявленная переменная` для валидного кода вида:

- `Проверка*.ПриСозданииНаСервереДокумент(ЭтотОбъект, Параметры);`

Для качественного IDE-grade поведения нужен единый контракт context-aware implicit bindings с учётом:
- типа модуля (`ModuleType`);
- фасета владельца метаданных;
- директив компиляции, особенно `*БезКонтекста`.

## Goals / Non-Goals
- Goals:
  - Убрать ложные `undeclared variable` для поддерживаемых implicit-переменных.
  - Сделать поведение детерминированным и единым между AST→IR, type inference и diagnostics.
  - Зафиксировать поддержку `ManagerModule` implicit `ЭтотОбъект/Объект`.
- Non-Goals:
  - Полное покрытие всех возможных системных имен 1С в любой подсистеме.
  - Расширение legacy pipeline; scope только v2.

## Decisions
- Decision: Ввести единый резолвер implicit bindings по `CodeLocation::ModuleType`.
  - Why: исключает расхождение между registration names и type seeding.

- Decision: Для `FormModule` установить:
  - `ЭтотОбъект`, `ЭтаФорма`, `Форма` -> `Формы.<Коллекция>.<Объект>.<Форма>`;
  - `Объект` -> form object type (`ДанныеФормыОбъект.<Коллекция>.<Объект>`/эквивалент из synthetic form type);
  - `Элементы` -> `ЭлементыФормы.<Коллекция>.<Объект>.<Форма>`;
  - `Параметры` -> `Структура`.
  - Why: соответствует ожиданиям типовых модулей форм и принятому решению по `Параметры`.

- Decision: Для `ManagerModule` добавить implicit `ЭтотОбъект` и `Объект` как тип менеджер-фасета (`<XxxМенеджер>.<Имя>`).
  - Why: принятое архитектурное решение для контекстного фасета менеджера.

- Decision: Для `ObjectModule` и `RecordSetModule` добавить/синхронизировать implicit `ЭтотОбъект` и `Объект` как object-фасет.
  - Why: единообразие фасетно-зависимых правил.

- Decision: В `&НаСервереБезКонтекста` и `&НаКлиентеНаСервереБезКонтекста` context-bound form bindings недоступны.
  - Why: семантика "без контекста" должна быть явно отражена в типизации и diagnostics.

## Architecture Notes
- Источник истины для правил implicit bindings должен вызываться из:
  - AST→IR seeding (имена переменных в symbol table),
  - type inference seeding (TypeResolution в env),
  - при необходимости из completion local-symbol слоя.
- Контракт должен быть case-insensitive на уровне lookup (`.to_lowercase()`), но хранить каноничное имя для отображения.

## Risks / Trade-offs
- Риск: чрезмерно широкой видимости `Параметры`.
  - Mitigation: ограничить только `FormModule` и учитывать `*БезКонтекста`.

- Риск: неполная metadata загрузка может приводить к отсутствию конкретного synthetic типа.
  - Mitigation: deterministic fallback в inferred-тип, но без деградации в undeclared.

- Риск: регрессии completion/hover из-за изменения seeding.
  - Mitigation: добавить точечные unit/integration тесты на form/manager/object контексты.

## Migration Plan
1. Утвердить spec delta (этот change).
2. Реализовать единый implicit-binding resolver в `analysis-v2`.
3. Подключить resolver в AST→IR и type inference.
4. Добавить тесты на:
  - `ЭтотОбъект`, `Объект`, `Форма`, `ЭтаФорма`, `Элементы`, `Параметры` в FormModule;
  - `ЭтотОбъект`, `Объект` в ManagerModule;
  - `*БезКонтекста` поведение;
  - отсутствие FP `Необъявленная переменная` для валидных кейсов.
