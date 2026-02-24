# Scripts Directory

Вспомогательные скрипты для разработки и тестирования BSL Gradual Types проекта.

## 🚫 Политика артефактов VSCode extension

Генерируемые файлы `vscode-extension/out/**` и `vscode-extension/*.vsix` не должны попадать в git.

Проверка:
```bash
./scripts/check-vscode-artifacts-policy.sh
```

Локальный pre-commit hook (опционально):
```bash
# Не перезаписывай существующий hook без проверки:
test -e .git/hooks/pre-commit || ln -s ../../scripts/git-hooks/pre-commit .git/hooks/pre-commit
```

## 🔒 Политика Cargo.lock

`Cargo.lock` должен быть отслеживаемым файлом в git (для воспроизводимых сборок).

Проверка:
```bash
./scripts/check-cargo-lockfile-policy.sh
```

## 📜 Доступные скрипты

### `add-defender-exclusions.ps1` - Настройка Windows Defender (НОВОЕ!)

**Назначение:** Автоматическое добавление проекта в исключения Windows Defender для ускорения HBK Recovery и компиляции.

**Использование:**
```powershell
# В PowerShell с правами администратора:
.\scripts\add-defender-exclusions.ps1
```

**Что делает:**
1. Добавляет весь проект в исключения Defender
2. Добавляет `examples\syntax_helper` (HBK Recovery - 52K файлов!)
3. Добавляет `target\` (Rust компиляция)
4. Добавляет LSP процессы в исключения
5. (Опционально) Отключает сканирование ZIP архивов

**Эффект:**
- HBK Recovery: **180s → 122s** (выигрыш ~60 секунд!)
- Компиляция: меньше overhead от Defender
- CPU: MsMpEng.exe <5% вместо 15-25%

**Требования:** Windows 10/11, права администратора

---

### `clear_cache.sh` - Очистка Windows File System Cache

**Назначение:** Очистить Windows File System Cache (Standby List) для тестирования прогресса парсинга.

**Когда использовать:**
- Нужно увидеть прогресс парсинга типов платформы 1С в реальном времени
- После первого парсинга всё кэшируется и повторный парсинг занимает ~1 секунду

**Использование:**
```bash
./scripts/clear_cache.sh
```

**Требования:**
- Windows 10/11
- Утилита `tools/EmptyStandbyList.exe` (скачивается отдельно)
- Права администратора (появится UAC prompt)

**Что делает:**
1. Проверяет наличие `EmptyStandbyList.exe`
2. Запускает утилиту от имени администратора
3. Очищает Standby List (файловый кэш Windows)

---

### `test_progress.sh` - Комплексное тестирование прогресса парсинга

**Назначение:** Автоматизировать подготовку к тестированию прогресса парсинга.

**Использование:**
```bash
./scripts/test_progress.sh
```

**Что делает:**
1. Вызывает `clear_cache.sh` для очистки кэша
2. Проверяет наличие собранного LSP сервера
3. Предлагает скопировать LSP сервер в `vscode-extension/bin/`
4. Даёт инструкции по запуску VSCode Extension

**Требования:**
- Те же, что и для `clear_cache.sh`
- Собранный LSP сервер (`cargo build --release --bin bsl-lsp-server`)

---

## 🎯 Типичный workflow

### Тестирование прогресса парсинга (первый раз):

```bash
# 1. Скачай утилиту EmptyStandbyList.exe (один раз)
cd tools
curl -L https://wj32.org/wp/download/releases/empty-standby-list/EmptyStandbyList.exe -o EmptyStandbyList.exe

# 2. Собери LSP сервер
cd ..
cargo build --release --bin bsl-lsp-server

# 3. Запусти комплексное тестирование
./scripts/test_progress.sh
```

### Тестирование прогресса парсинга (повторно):

```bash
# Просто очисти кэш и запусти VSCode Extension
./scripts/clear_cache.sh

# Затем:
# - Открой vscode-extension в VSCode
# - Нажми F5
# - Открой конфигурацию 1С
# - Наблюдай прогресс!
```

---

### `run-intellisense-perf.sh` - Perf suite для IntelliSense

**Назначение:** регрессионные замеры completion latency (P95/P99) на профилях `small` и `medium`.

**Использование:**
```bash
./scripts/run-intellisense-perf.sh
```

**Обновление baseline:**
```bash
UPDATE_BASELINE=1 ./scripts/run-intellisense-perf.sh
```

**Large профиль (ручной запуск):**
```bash
cargo run -p bsl-backend --bin intellisense_perf -- \
  --scenario backend/tests/perf/scenarios/intellisense_large.json \
  --baseline backend/tests/perf/baselines/intellisense_large.json \
  --update-baseline \
  --threshold-p95 1.10 \
  --threshold-p99 1.15 \
  --output backend/tests/perf/reports/intellisense_large.json
```

**Вывод:**
- Baselines: `backend/tests/perf/baselines/`
- Отчёты: `backend/tests/perf/reports/`

---

### `validate-v2-completion-gates.sh` - Acceptance gates для `improve-v2-completion-interactive-reliability`

**Назначение:** воспроизводимый fail-fast прогон acceptance gates для задач `3.6/3.7`:
- completion latency: `p95 <= 300ms`, `p99 <= 800ms`;
- first-trigger success rate: `>= 99%`;
- terminal-empty (`missing_ir`) rate: `<= 0.5%`;
- parity mismatch rate: `<= 1%`;
- strict-валидация change через OpenSpec.

**Использование:**
```bash
./scripts/validate-v2-completion-gates.sh
```

**Важно:** скрипт не зависит от `.github/workflows/*` и предназначен для локального запуска или внешнего CI (например, Jenkins/GitLab Runner).

**Артефакты:**
- `backend/tests/perf/reports/improve-v2-completion-interactive-reliability-gate.json`
- `backend/tests/perf/reports/improve-v2-completion-interactive-reliability-gate.md`
- `backend/tests/perf/reports/improve-v2-completion-interactive-reliability-openspec-validate.log`

---

### `check-contract-compatibility-diff.py` - Manual compatibility-diff gate для `contracts/**`

**Назначение:** сравнить baseline и candidate versioned contracts и определить
`non_breaking`/`breaking` изменения с policy enforcement:
- breaking без major bump → fail (`breaking_without_major_bump`);
- major bump без migration note → fail (`missing_migration_note`).

**Использование:**
```bash
python3 scripts/check-contract-compatibility-diff.py \
  --baseline-ref master \
  --candidate-root contracts \
  --report artifacts/contracts-compatibility-diff-report.json
```

**Regression fixtures/tests (2 breaking + 2 non-breaking):**
```bash
python3 scripts/test-contract-compatibility-diff.py
```

Фикстуры лежат в `scripts/fixtures/contracts-compatibility-diff/`.

---

### `run-intellisense-tests.sh` - Smoke/full тесты IntelliSense

**Назначение:** стабильный набор тестов M8 для локального полного прогона; подходит для CI при подключении.

#### Политика уровней тестов (smoke vs manual/heavy)

В репозитории используется двухуровневая модель:

- **smoke**:
  - MUST проходить на чистом checkout репозитория;
  - MUST не требовать внешних фикстур/скачиваний/подготовки окружения (в т.ч. больших корпоративных конфигураций);
  - SHOULD выполняться быстро (ориентир: минуты, а не десятки минут);
  - цель: ежедневная проверка, что IntelliSense v2 не регрессирует по базовым сценариям.
- **manual/heavy**:
  - MAY требовать тяжёлые или внешние данные (например, Syntax Helper, большие конфигурации, специальные «битые» фикстуры);
  - MAY требовать запущенный сервер/особый сценарий запуска;
  - цель: расширенная проверка и расследование регрессий.

Инвентаризация `#[ignore]` и `tests/disabled/*` с причинами: `scripts/test-skip-inventory.md`.

**Использование:**
```bash
# Быстрый smoke (использует репозиторные фикстуры, не требует внешних данных)
./scripts/run-intellisense-tests.sh smoke

# Полный прогон (smoke + fixture конфигурации)
./scripts/run-intellisense-tests.sh full
```

**Состав:**
- `smoke`: unit‑тесты completion + golden + LSP интеграция.
- `full`: smoke + интеграционные тесты, которые загружают fixture конфигурации (например, `examples/conf/conf_test`).

---

## 🔧 Разрешение проблем

### Ошибка: "EmptyStandbyList.exe не найдена"

**Решение:**
```bash
cd tools
curl -L https://wj32.org/wp/download/releases/empty-standby-list/EmptyStandbyList.exe -o EmptyStandbyList.exe
```

Или скачай вручную: https://wj32.org/processhacker/forums/viewtopic.php?t=1569

См. `tools/DOWNLOAD_EmptyStandbyList.txt` для подробных инструкций.

### Ошибка: "Не удалось очистить кэш"

**Возможные причины:**
1. Отменил UAC prompt (нажал "Нет")
   - **Решение:** Запусти снова и нажми "Да" в UAC prompt
2. Антивирус заблокировал утилиту
   - **Решение:** Добавь `tools/EmptyStandbyList.exe` в исключения антивируса
3. Недостаточно прав администратора
   - **Решение:** Запусти GitBash от имени администратора

### Не вижу прогресс парсинга в VSCode

**Проверь:**
1. Кэш действительно очищен? (запусти `clear_cache.sh` снова)
2. LSP сервер обновлён? (`cp target/release/bsl-lsp-server.exe vscode-extension/bin/`)
3. VSCode Extension перезапущен? (закрой debug окно VSCode и запусти F5 снова)

---

## 📚 Дополнительная информация

- **Windows File System Cache:** Windows кэширует прочитанные файлы в RAM (Standby List). После парсинга 24979 файлов документации 1С всё кэшируется в памяти, и повторный парсинг занимает ~1 секунду вместо 1-2 минут.

- **EmptyStandbyList.exe:** Официальная утилита от разработчика Process Hacker (wj32). Очищает Standby List без перезагрузки системы.

- **Альтернативы:** RAMMap от Microsoft Sysinternals (GUI утилита) или PowerShell скрипты.

---

## ⚡ Быстрый прогон тестов: cargo-nextest

`scripts/build-all.sh` на этапе тестов использует `cargo nextest` (если `cargo-nextest` установлен) — это обычно быстрее и даёт более удобный вывод.

Установка:
```bash
cargo install cargo-nextest --locked
```

Управление:
- режим тестов: `./scripts/build-all.sh --tests quick|smoke|full`
  - `quick` (по умолчанию): debug + subset lib-тестов
  - `smoke`: `./scripts/run-intellisense-tests.sh smoke`
  - `full`: `cargo nextest run --release --workspace` (или `cargo test --release --workspace` если nextest выключен)
- принудительно включить nextest: `./scripts/build-all.sh --nextest`
- выключить nextest: `./scripts/build-all.sh --no-nextest`

---

## 🛡️ Безопасность

Все скрипты:
- ✅ Используют официальные утилиты
- ✅ Требуют явного подтверждения UAC
- ✅ Не модифицируют системные файлы
- ✅ Только очищают кэш RAM (обратимая операция)

EmptyStandbyList.exe:
- ✅ Open Source (исходный код доступен)
- ✅ Подписана цифровой подписью
- ✅ Широко используется разработчиками

---

**См. также:**
- `tools/README.md` - Подробнее об утилитах
- `tools/DOWNLOAD_EmptyStandbyList.txt` - Инструкции по скачиванию
