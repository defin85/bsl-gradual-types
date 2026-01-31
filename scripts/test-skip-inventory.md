# Инвентаризация пропусков тестов (`#[ignore]`, `tests/disabled`)

Контекст: change `openspec/changes/reduce-test-skip-surface/`.

Цель файла: зафиксировать текущий список пропускаемых/отключённых тестов и причины, чтобы дальше принять решение по каждому (smoke / manual-heave / удалить).

## `#[ignore]` (Rust tests)

Примечание: эти тесты существуют в активном test-suite, но не запускаются по умолчанию `cargo test` (их нужно запускать через `-- --ignored` или по имени).

| Путь | Тест | Причина (как в коде) | Prerequisites | Команда запуска |
|---|---|---|---|---|
| `backend/tests/debug_tablica_znacheniy.rs` | `debug_tablica_znacheniy_ast` | Debug test uses a large fixture and is not part of CI | локальный файл `examples/bsl/test_hover_milestone_2_11.bsl` | `cargo test -p bsl-backend --test debug_tablica_znacheniy -- --ignored debug_tablica_znacheniy_ast` |
| `backend/tests/debug_tree_sitter_structure.rs` | `debug_tree_sitter_ast_structure` | Debug test uses a large fixture and is not part of CI | локальный файл `examples/bsl/test_hover_milestone_2_11.bsl` | `cargo test -p bsl-backend --test debug_tree_sitter_structure -- --ignored debug_tree_sitter_ast_structure` |
| `backend/tests/semantic_visualization_test.rs` | `test_semantic_visualization_basic_hover_m8_v2_pipeline` | TODO: Полная интеграция с v2 entrypoints | зависит от наличия корректных entrypoints/интеграции в тесте | `cargo test -p bsl-backend --test semantic_visualization_test -- --ignored test_semantic_visualization_basic_hover_m8_v2_pipeline` |
| `backend/tests/semantic_visualization_test.rs` | `test_semantic_visualization_http_server_roundtrip` | Требуется запущенный LSP server | нужен запущенный сервер (см. тест) | `cargo test -p bsl-backend --test semantic_visualization_test -- --ignored test_semantic_visualization_http_server_roundtrip` |
| `backend/tests/lsp_search_types_test.rs` | `test_search_types_production_syntax_helper` | Требует наличия Syntax Helper (examples/syntax_helper) | локальный каталог `examples/syntax_helper` (hbk/распакованные данные) | `cargo test -p bsl-backend --test lsp_search_types_test -- --ignored test_search_types_production_syntax_helper` |

## `tests/disabled/*` (архив/отключённые тесты)

Статус: каталог `tests/disabled/` очищен (legacy тесты удалены как устаревшие/неиспользуемые).
