## Context
Текущая sidebar архитектура расширения разделена на два activity bar контейнера (`bslAnalyzer`, `bslAnalyzerCache`) и несколько независимых виджетов, которые получают данные по разным путям и с разным временем обновления.

Наблюдаемые симптомы:
- противоречие между `Overview: Issues Found` и содержимым `Diagnostics`;
- разные значения количества типов между `Overview`, `Type Repository` и `Quick Actions`;
- статический контент в Quick Actions вместо live-метрик;
- попадание internal UI tokens в пользовательский текст.

## Goals / Non-Goals
- Goals:
  - Один вход в sidebar (`BSL Analyzer`) вместо двух раздельных activity panels.
  - Единый контракт данных для всех sidebar виджетов.
  - Детерминированная консистентность счётчиков и статусов.
  - Отсутствие сырых internal токенов и хардкодов в user-facing UI.
- Non-Goals:
  - Не менять LSP-протокол.
  - Не делать полный UI-редизайн.
  - Не переписывать типовую систему/диагностику вне области sidebar contracts.

## Decisions

### 1) Unified Activity Bar Container
Расширение MUST иметь один activity bar container (`bslAnalyzer`).
Cache dashboard переносится как обычный view в тот же контейнер.

`bslAnalyzerCache` помечается как устаревший в migration phase и удаляется после совместимого перехода.

### 2) Единый Snapshot Contract для sidebar
Вводится единый агрегированный snapshot состояния sidebar (рабочее имя: `SidebarSnapshot`), включающий:
- `workspace_stats` (файлы, diagnostics),
- `type_repository_stats` (total/platform/config + timestamp/revision),
- `cache_stats` (включая status of optional metrics),
- `ui_health` (availability/freshness flags).

Все виджеты читают данные из одного snapshot/revision, а не из разрозненных запросов "кто когда успел".

### 3) Counter Consistency Policy
Для счётчиков и summary-строк:
- `Overview` и `Type Repository` используют один источник `type_repository_stats`;
- `Quick Actions` показывает live-count из этого же источника;
- `Diagnostics` summary согласован с источником workspace diagnostics.

Если данные устарели/недоступны, UI MUST показывать явный статус `stale`/`n/a` с причиной, а не противоречивые числа.

### 4) UI Rendering Policy
Пользовательский UI MUST NOT показывать сырые internal-токены формата `$(...)`.
Иконки и статусы отображаются через корректный VS Code UI API (`ThemeIcon`/iconPath) или уже отрендеренный текст.

### 5) Migration / Compatibility
- Команды refresh сохраняются (`bslAnalyzer.refreshOverview`, `bslAnalyzer.refreshCacheDashboard`, ...), но работают внутри единого container.
- Вкладка cache не теряется функционально; меняется только место и контракт данных.

## Risks / Trade-offs
- Риск: временное дублирование логики при migration period.
  - Mitigation: ограничить миграцию одним release window.
- Риск: нагрузка от частых unified refresh.
  - Mitigation: shared TTL/coalescing и debounce обновлений.
- Риск: регрессии тестов из-за смены view IDs/container wiring.
  - Mitigation: обновить e2e/smoke тесты на новую структуру sidebar.
