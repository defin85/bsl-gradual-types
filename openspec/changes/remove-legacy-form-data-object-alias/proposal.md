# Change: Удалить legacy алиас `ДанныеФормыОбъект.*` и перейти на контекстную модель implicit-symbols в v2

## Why
В текущем v2 pipeline переменная `Объект` в модуле формы может типизироваться через внутренний synthetic алиас `ДанныеФормыОбъект.*`, который не является платформенным типом 1С. Это приводит к ложным diagnostics (например, для `Объект.Ссылка`), рассинхронизации с платформенной моделью form data и утечке internal-типов в пользовательские сообщения.

## What Changes
- Ввести контекстную модель implicit-symbols для модулей (`FormModule`, `ManagerModule`, `ObjectModule`, `RecordSetModule`) как единый источник правил.
- Для `FormModule` представлять `Объект` через платформенную семантику form data (`ДанныеФормыСтруктура` и связанные form-data типы), а не через legacy алиас `ДанныеФормыОбъект.*`.
- Добавить проекцию гарантированных членов applied-объекта в form-data контексте (минимум `Ссылка` для документных форм), без ложных `NonExistentProperty`.
- Полностью убрать `ДанныеФормыОбъект.*` из публичных v2 outputs (diagnostics/hover/completion/type-at-position).
- Обновить регрессионные тесты на матрицу контекстов и проверить отсутствие legacy-имен в выдаче.

## Impact
- Affected specs: `bsl-intellisense-v2`
- Affected code:
  - `analysis-v2/src/implicit_bindings.rs`
  - `analysis-v2/src/type_inference_v2.rs`
  - `shared/src/domain/metadata_lookup/*`
  - `bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs`
  - интеграционные тесты `backend/tests/*`
- Поведенческое изменение: user-facing типы в hover/diagnostics для form-object контекста перестанут содержать legacy имя `ДанныеФормыОбъект.*`.
