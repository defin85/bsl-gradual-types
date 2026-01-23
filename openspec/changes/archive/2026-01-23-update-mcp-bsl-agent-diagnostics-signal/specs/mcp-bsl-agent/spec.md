## ADDED Requirements

### Requirement: Асинхронное получение диагностики по проекту/фокусу (`bsl_diagnostics_start`)
Система SHALL поддерживать `bsl_diagnostics_start` для разных областей анализа (`scope`) и возвращать результат через job‑модель (`job_status/job_wait/job_result`).

`scope` SHALL поддерживать:
- строковые значения `project|hot` (LLM-friendly),
- tagged значение `{ "kind": "file", "document": <DocumentRef> }` для диагностики одного файла.

Система SHALL возвращать `INVALID_PARAMS` (fail-fast) для неоднозначных/неполных scope’ов (например, `scope="file"` как строка) и SHOULD включать в сообщение подсказку корректного формата tagged file scope.

#### Scenario: Tagged file scope даёт диагностику одного файла
- **GIVEN** сессия открыта и `workspace_status.ready=true`
- **WHEN** клиент вызывает `bsl_diagnostics_start` с `scope={kind:file, document:{path:\"/abs/.../Module.bsl\"}}` и получает результат через `job_result`
- **THEN** сервер возвращает diagnostics только по указанному документу

### Requirement: Диагностика не должна шуметь на динамических типах (`Dynamic.*`)
Система SHALL избегать малоинформативных ошибок “несуществующий метод/свойство” для receiver’ов, чей тип является dynamic-like (например, `Dynamic` или `Dynamic.*`), поскольку такие ошибки часто являются следствием ограничения статического вывода типов.

#### Scenario: Dynamic-like receiver не генерирует “NonExistentProperty/Method”
- **GIVEN** анализатор вывел тип receiver как `Dynamic.<Facet>` (например, `Dynamic.Объект`)
- **WHEN** вычисляется семантическая диагностика для обращения к члену (метод/свойство)
- **THEN** диагностика “член не существует” не добавляется только на основании dynamic-like типа receiver’а

### Requirement: Unknown member access severity деградирует до Warning при неполной инференции
Система SHALL классифицировать “unknown member access” так, чтобы случаи неполной инференции не доминировали над реальными ошибками:
- `UndeclaredVariable` и `TypeNotFound` остаются `Error`,
- `ConfigurationNotLoaded` подавляется (graceful degradation),
- прочие unknown причины маркируются как `Warning`.

#### Scenario: Unknown member access становится Warning
- **GIVEN** тип receiver не выведен (unknown), но причина неизвестности не является `UndeclaredVariable`/`TypeNotFound`
- **WHEN** вычисляется семантическая диагностика обращения к члену
- **THEN** диагностика возвращается как `Warning`, а не `Error`
