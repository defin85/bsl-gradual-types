# Legacy inference paths audit (tasks.md 1.2)

Дата: 2026-01-29

Цель: собрать полный список оставшихся legacy путей/фолбеков, которые:
- обходят v2 snapshot (salsa/`analysis-v2`) как единственный source of truth,
- или поддерживают совместимость со старым контрактом (например, completion resolve без `candidate_id`),
- и дать план полного выпиливания (см. tasks.md раздел 7.*).

## Найдено (файлы/вызовы)

### 1) Hover: legacy fallback через `crate::parsing` + прямой `TypeResolver`

Факты в коде:
- `backend/src/application/type_system/services/hover_service.rs:175`:
  при отсутствии `type_at_cursor` выполняется fallback `extract_enhanced_symbol_info(...)`.
- `backend/src/application/type_system/services/hover_service.rs:339`:
  `extract_enhanced_symbol_info` использует `crate::parsing::bsl::ast::{Expression, Statement}`.
- `backend/src/application/type_system/extractors/type_extractor.rs:6`:
  хелпер `expression_to_type_name` тоже завязан на `crate::parsing::bsl::ast::Expression`.

Почему это legacy путь:
- этот fallback способен выдать hover на основе legacy AST и `resolver.resolve_expression_sync(...)`,
  минуя v2 `type_index`/type hints и контракт snapshot-safety.
- сейчас он вызывается с `parse_result: None`, т.е. по сути работает как "resolve по слову",
  что противоречит заявленному в change принципу "v2-only, без best effort обходов".

План выпиливания:
1. Удалить из `compute_hover_info_from_ir` ветку fallback на `extract_enhanced_symbol_info`.
2. Удалить/переписать `extract_enhanced_symbol_info` и зависимые legacy-утилиты так, чтобы hover строился
   только из v2 данных: `ir_program` + `analysis.type_at_byte_offset(...)` + `type_hints` (если нужны).
3. После удаления fallback убрать зависимость на `crate::parsing` из `backend/src/application/type_system/extractors/type_extractor.rs`
   (либо удалить модуль целиком, либо перевести на `bsl_syntax::ast` если он где-то еще нужен).

### 2) Completion resolve: совместимость без `candidate_id`

Факты в коде:
- `backend/src/bin/lsp_server/handlers/completion.rs:181`:
  `handle_completion_resolve` делает `parse_candidate_id(&item)`; при отсутствии `candidate_id` возвращает item как есть.
- `backend/src/bin/lsp_server/handlers/completion.rs:1232`:
  тест `m6_completion_resolve_legacy_fallback_works_without_candidate_id` фиксирует это поведение.

Почему это legacy путь:
- сервер продолжает явно поддерживать контракт "completion item может прийти на resolve без `candidate_id`".
  Это не добавляет inference, но оставляет совместимость/поведение для старых клиентов и размывает контракт.

План выпиливания:
1. Удалить тест `m6_completion_resolve_legacy_fallback_works_without_candidate_id`.
2. Решить контрактно (в рамках tasks.md 7.2):
   - либо сделать `candidate_id` обязательным (и логировать/отказывать в resolve без него),
   - либо оставить "no-op" как безопасную совместимость, но убрать любые упоминания "legacy fallback" и
     явно документировать, что без `candidate_id` resolve ничего не делает.

### 3) LSP analysis runtime: fallback на пустой snapshot при сбое writer-thread

Факты в коде:
- `backend/src/bin/lsp_server/server/analysis_v2_runtime.rs:223`, `:229`, `:248`, `:259`:
  при ошибках IPC/отсутствии writer thread возвращается `AnalysisHostV2::default().snapshot()`
  (т.е. "пустой" snapshot вместо последнего согласованного).

Почему это может считаться bypass:
- такой fallback скрывает деградацию runtime и может приводить к ответам, вычисленным не из актуального состояния,
  что противоречит идее "все ответы IDE из одного snapshot".

План выпиливания/ужесточения:
1. Заменить fallback на:
   - возврат ошибки в LSP (и метрики), либо
   - хранение последнего валидного snapshot и отдачу его как "stale but consistent",
   - без создания нового `AnalysisHostV2` "на лету".
2. Добавить тест/проверку на отсутствие silent-fallback в runtime (если это в scope tasks.md 7.4).

### 4) Другие зависимости от `crate::parsing` (сейчас stubs, но это debt)

Факты в коде:
- `backend/src/presentation/lsp/type_hints.rs:15`: принимает `&crate::parsing::bsl::ast::Program` (stub).
- `backend/src/presentation/lsp/code_actions.rs:8`: использует `crate::parsing::bsl::ast::Program` (stub).

План:
- либо удалить эти stubs, либо перевести интерфейсы на `bsl_syntax::ast` / v2 IR (чтобы не тянуть legacy parsing).

## Не найдено (проверено поиском)

- `parse_to_ir`: по репозиторию нет совпадений `rg -n "parse_to_ir" -S ...` (на момент 2026-01-29).
- `parse_and_analyze` / `AnalysisEngine::parse_and_analyze`: совпадений нет.
- legacy signatureHelp handler: `handle_signature_help_v2` есть, других `handle_signature_help*` не найдено.

## Мини-план "полностью убрать" (связка с tasks.md 7.*)

1. (7.2) Убрать legacy-компат в completion resolve (минимум: удалить тест; лучше: контрактно обязать `candidate_id`).
2. (7.4) Выпилить hover fallback на `crate::parsing`/`TypeResolver` и оставить только v2 данные.
3. (7.4) Ужесточить поведение `AnalysisV2Runtime::snapshot*` при сбоях: без `AnalysisHostV2::default().snapshot()`.
4. (8.2) После выпиливания прогнать поисковые проверки (rg) на ключевые маркеры legacy путей и обновить tasks.md.

