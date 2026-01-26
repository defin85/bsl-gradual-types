# Design: MCP tools для discovery типов (bsl-agent)

## Цель
Дать LLM/MCP-клиенту быстрые read-only primitives, чтобы ориентироваться в “видимой через MCP” конфигурации:
найти нужный тип, получить его реквизиты/табличные части и понять поверхность API типа.

## Текущее состояние (наблюдения по коду)
- В `bsl-agent` stdio MCP tools сейчас покрывают diagnostics/symbols/definition/members/references/context_pack, но нет tools для list/search/get типов.
- В `bsl-agent` HTTP UI уже имеет parity эндпоинты `/api/mcp/types` и `/api/mcp/search`, которые возвращают `AnalysisResultDto` (включая `TypeDto.properties` и `TypeDto.tabularSections`).

## Предлагаемый toolset (stdio MCP)
Все tools read-only и следуют текущему паттерну `*_start` (асинхронно через job).

### 1) `bsl_types_list_start`
Назначение: получить список типов с пагинацией и фильтрами (аналог `/api/mcp/types`).

Параметры (черновик):
- `session_id: string`
- `page?: u32` (default 1, min 1)
- `limit?: u32` (default 50, clamp 1..=1000)
- `source?: "platform" | "configuration"`
- `category?: string`
- `certainty_level?: u8` (0..=100)
- `flow_sensitive_only?: bool` (default false)
- `view?: "names_only" | "summary" | "full"` (default "summary")

Ответ:
- при `view="names_only"`: JSON массив `string[]` (имена типов);
- иначе: `AnalysisResultDto`

Правило `view`:
- `names_only`: вернуть только имена типов (payload-minimal, LLM-friendly).
- `summary`: сервер возвращает типы без “тяжёлых” полей (например, `methods=[]`, `tabularSections=[]`), но с `methodsCount`/`attributesCount`, чтобы payload был стабильным.
- `full`: сервер возвращает полный `TypeDto` как в parity API (для UI/debug).

### 2) `bsl_types_search_start`
Назначение: найти тип(ы) по строке (аналог `/api/mcp/search`).

Параметры (черновик):
- `session_id: string`
- `query: string`
- `limit?: u32` (default 200, clamp 1..=1000)
- `source?: "platform" | "configuration"`
- `view?: "names_only" | "summary" | "full"` (default "summary")

Ответ:
- при `view="names_only"`: JSON массив `string[]` (имена типов);
- иначе: `AnalysisResultDto`

### 3) `bsl_type_get_start`
Назначение: получить детали конкретного типа по имени (без необходимости сканировать большие списки).

Параметры (черновик):
- `session_id: string`
- `type_name: string` (точное имя типа, как в `TypeDto.name`)
- `source?: "platform" | "configuration"`
- `include_methods?: bool` (default false)

Ответ: `TypeDto`

Минимально обязательные поля в ответе:
- `properties[]` (реквизиты/свойства),
- `tabularSections[]` (табличные части и атрибуты),
- `methodsCount` (всегда), `methods[]` (только если `include_methods=true`).

## Нефункциональные требования
- Детерминизм: одинаковый snapshot → одинаковый порядок типов (например, сортировка по `(category, name)`).
- Лимиты: `limit` и дополнительные server-side ограничения по размеру ответа, чтобы tool-call не “взрывал” токены.
- Ошибки:
  - сессия не ready → предсказуемая ошибка,
  - тип не найден → предсказуемая ошибка (NOT_FOUND/INVALID_PARAMS),
  - неизвестный `view` → INVALID_PARAMS.

## Открытые вопросы
- Нужны ли дополнительные фильтры по `facets`?
- Должен ли `bsl_type_get_start` поддерживать поиск по `id` (если `id != name` в будущем)?
