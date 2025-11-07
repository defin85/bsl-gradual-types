# Scripts Directory

Вспомогательные скрипты для разработки и тестирования BSL Gradual Types проекта.

## 📜 Доступные скрипты

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
