# Roadmap: Overview/Type Repository панели (реальные данные вместо хардкода)

**Статус:** 🟡 В ПРОЦЕССЕ  
**Приоритет:** HIGH  
**Цель:** сделать панель Overview и Type Repository “честной”: данные берутся из LSP сервера и TypeRepository, без захардкоженных значений.

---

## Контекст (по репозиторию)

### Что было до изменений
- `vscode-extension/src/providers/overviewProvider.ts` содержит хардкод: "BSL Files: Scanning...", "Last Analysis: Never", "Issues Found: 0", "UnifiedBslIndex: Loading...", "Platform: 8.3.25".
- `bsl.getAllTypes` возвращал только платформенные типы из `TypeInferenceService.get_all_platform_globals()` → конфигурационные типы не попадали в дерево.

### Что изменено
- LSP добавляет `bsl.getWorkspaceStats` и ведёт счетчик диагностик по открытым документам.
- `bsl.getAllTypes` теперь собирается напрямую из `TypeRepository` (RawTypeData → TypeDto).
- Overview читает LSP stats и прогресс, а не хардкод.

---

## Проблемы/риски
1) **Неполная статистика диагностик** — учитываются только открытые документы (LSP не хранит полный workspace diagnostics).  
2) **Сканирование .bsl файлов** — происходит синхронно при запросе (для очень больших проектов может быть ощутимо).

---

## Milestones

### P1: Workspace stats через LSP 🟢
**Цель:** убрать хардкод и показывать реальное количество BSL файлов + диагностик.

**Сделано:**
- LSP команда `bsl.getWorkspaceStats`.
- Счетчик диагностик по `publish_diagnostics`.

**Проверка:**
- `backend/src/bin/lsp_server/server/command_handlers.rs` (handle_get_workspace_stats)
- `backend/src/bin/lsp_server/types.rs` (WorkspaceStatsResponse)

---

### P2: Реальные типы в Type Repository 🟢
**Цель:** дерево типов показывает платформу + конфигурацию.

**Сделано:**
- `bsl.getAllTypes` собирается из `TypeRepository.get_all_types()`.
- RawTypeData → TypeDto маппинг (методы/свойства/описания/табличные части).

**Проверка:**
- `backend/src/bin/lsp_server/commands/get_all_types.rs`
- `vscode-extension/src/providers/typeTreeBuilder.ts`

---

### P3: Overview читает реальные данные 🟡
**Цель:** Overview не содержит хардкода.

**Сделано:**
- `vscode-extension/src/providers/overviewProvider.ts` читает:
  - `bsl.getWorkspaceStats` → BSL files + Issues found
  - `bsl.getTypeRepositoryStats.lastUpdateTime` → Last Analysis
  - `bsl.getTypeRepositoryStats` → TypeRepository summary
- `$/progress` синхронизирован с Overview через `setIndexingProgress`.

**Проверка:**
- `vscode-extension/src/lsp/client/progress-handler.ts`
- `vscode-extension/src/lsp/progress.ts`

---

## Следующие шаги
1) При необходимости — добавить серверную агрегацию diagnostics по всему workspace (не только открытые файлы).
2) Вынести сканирование файлов в фоновую задачу, если будет заметная задержка при открытии Overview.
