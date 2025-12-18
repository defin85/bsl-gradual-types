# Roadmap: Переработка тестов VSCode-расширения (unit/integration/e2e, фикстуры, LSP)

**Статус:** 🟡 В ПРОЦЕССЕ (есть первые фиксы test-mode и прокидывание settings, но suite нестабилен)  
**Приоритет:** HIGH  
**Цель:** сделать тесты расширения **детерминированными**, **быстрыми по умолчанию** и **предсказуемыми**: 90% логики проверяется unit-тестами без VSCode, интеграционные тесты работают без реального LSP, а e2e (с реальным LSP/парсингом) — отдельный тяжёлый прогон.

---

## Контекст (проверено по репозиторию)

### Как сейчас запускаются “интеграционные” тесты
- `npm test` запускает `node ./out/test/runTest.js` (через `@vscode/test-electron`): `vscode-extension/package.json`.
- Тестовый VS Code использует директорию `.vscode-test` рядом с расширением (user-data, logs, extensions): `vscode-extension/.vscode-test/`.
- Тестовая suite поднимается через Mocha и glob по `**/**.test.js`: `vscode-extension/src/test/suite/index.ts`.

### Что ломало/ломает suite
- В тестовом окружении расширение пыталось показать UI-диалог выбора конфигурации, что приводило к `Canceled` и обрывало активацию: `vscode-extension/src/utils/configurationFinder.ts:127`.
- LSP стартует без путей `platformDocsArchive/configurationPath`, поэтому **реальный** парсинг docs/config не запускается, а тесты, ожидающие LSP/команды, падают по таймаутам: `vscode-extension/src/extension.ts:137`, `vscode-extension/src/test/suite/index.ts`.

### Уже сделано (первичные стабилизации)
- В test mode отключён UI-диалог выбора конфигурации (graceful skip): `vscode-extension/src/utils/configurationFinder.ts:127`.
- Команда `parseConfiguration` больше не дублирует прогресс локальным `withProgress`; “источник истины” — server-initiated WorkDoneProgress (`$/progress`): `vscode-extension/src/commands/parseConfiguration.ts:28`, `vscode-extension/src/lsp/client/progress-handler.ts:31`.
- Прогресс индексации BSL-модулей теперь попадает в WorkDoneProgress команды `bsl.parseConfiguration`: `backend/src/bin/lsp_server/commands/configuration.rs:134`.
- Тест-раннер может записывать settings для тестового VS Code (в т.ч. опционально реальные пути через env): `vscode-extension/src/test/runTest.ts:1`, `vscode-extension/src/test/suite/index.ts:1`.

---

## Принципы (Right-Sized Architecture)

- **Единый источник истины:** прогресс LSP-серверных операций всегда идёт через WorkDoneProgress (`$/progress`), а не через локальные `withProgress` в командах расширения.
- **Детерминизм:** unit/integration не зависят от наличия реальных архивов docs/конфигурации.
- **Разделение уровней:** unit ≠ integration ≠ e2e. Не смешивать ожидания.
- **Минимум магии:** тестовый workspace и настройки должны быть явными и воспроизводимыми.

---

## Цели и не-цели

### Цели
- Unit-тесты: быстрые (секунды), без VS Code/Extension Host.
- Integration-тесты: Extension Host есть, но LSP “замокан” и не требует реальных путей.
- E2E: отдельный прогон, который может использовать реальные `examples/syntax_helper` и `examples/conf/conf_test` и валидирует progress/команды на настоящем LSP.
- Уменьшить flaky/timeout тесты, ввести единые таймауты и хелперы ожиданий.

### Не-цели (на первом этапе)
- Полный e2e прогон всех команд/панелей на реальном LSP (слишком дорого).
- Поддержка всех OS/окружений (WSL/CI/GUI) без фикстур и без явных зависимостей.

---

## Milestones (вертикальные срезы)

### T1: Разделить тесты по уровням (unit / integration / e2e) 🟡
**Цель:** прекратить запуск “всего подряд” одним glob’ом и разделить ответственность.

**Критерии успеха:**
- `npm test` запускает только unit + integration (mock LSP).
- `npm run test:e2e` запускает отдельный набор e2e.

**Проверка:**
- ✅ сейчас тестовый suite собирает тесты glob’ом: `vscode-extension/src/test/suite/index.ts:1`.
- ❌ нет раздельных entrypoints/скриптов.

---

### T2: Фикстурный workspace для тестового VS Code 🟡
**Цель:** тестовый VS Code всегда стартует в заранее известном workspace (а не “как получится”).

**Критерии успеха:**
- В репо есть `vscode-extension/test-fixtures/workspace/` (минимальный проект) + `.vscode/settings.json`.
- `runTests()` получает `launchArgs` с путём на фикстуру workspace.

**Проверка:**
- ❌ `runTests()` сейчас без `launchArgs`: `vscode-extension/src/test/runTest.ts:1`.

---

### T3: Mock LSP для integration-тестов 🟡
**Цель:** интеграционные тесты расширения не зависят от Rust LSP и данных docs/config.

**Подход:**
- Ввести DI-слой: фабрика LanguageClient / интерфейс клиента / адаптер.
- В test mode подменять реальный клиент мок-реализацией.

**Критерии успеха:**
- Тесты `searchTypes/queryType/buildIndex` могут выполняться без поднятого LSP.
- Убираем таймауты “ждём 10 секунд, пока LSP оживёт”.

**Проверка:**
- ❌ сейчас тесты косвенно завязаны на реальный `startLanguageClient()`: `vscode-extension/src/extension.ts:140`, `vscode-extension/src/lsp/client/lifecycle.ts:96`.

---

### T4: E2E smoke-тесты на реальном LSP 🟡
**Цель:** 1–3 теста, которые действительно гарантируют базовую работоспособность end-to-end.

**Набор smoke:**
1) LSP стартует и отдаёт WorkDoneProgress begin/end.
2) `bsl.parseConfiguration` отдаёт прогресс и завершает `End` после индексации BSL модулей.
3) (опционально) `bsl.searchTypes` возвращает непустой результат после загрузки.

**Критерии успеха:**
- Тесты запускаются отдельной командой (`npm run test:e2e`) и имеют большие таймауты (минуты).
- Пути к фикстурам задаются env (`BSL_TEST_*`) или явным settings.json.

---

### T5: Таймауты и ожидания 🟡
**Цель:** централизовать ожидания событий (активация, регистрация команд, прогресс begin/end).

**Критерии успеха:**
- Единый helper `waitForCommand()/waitForClientRunning()/waitForProgressEnd()`.
- Таймауты зависят от уровня теста (unit: <1s, integration: <5–10s, e2e: 2–5min).

---

### T6: Диагностика и логи тестового VS Code ✅/🟡
**Цель:** при падении тестов быстро понимать, почему (без ручного дебага GUI).

**Критерии успеха:**
- При падении тестов выводится путь на `.vscode-test/user-data/logs/<timestamp>/...` и подсказка какие файлы читать.
- В логах расширения пишется режим теста, настройки, старт LSP.

**Фактический прогресс:**
- ✅ тестовый VS Code пишет логи в `.vscode-test/user-data/logs`: `vscode-extension/.vscode-test/user-data/logs/`.

---

## План внедрения (минимальный)

1) T2: добавить фикстурный workspace + `launchArgs` в `runTest.ts`.
2) T1: split suite entrypoints и npm scripts.
3) T3: DI для LSP клиента и мок в test mode.
4) T5: вынести ожидания/таймауты в helpers.
5) T4: добавить отдельные e2e smoke-тесты (с `BSL_TEST_USE_REAL_FIXTURES=1`).

