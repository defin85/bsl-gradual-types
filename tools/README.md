# Tools Directory

Директория для вспомогательных утилит проекта.

## EmptyStandbyList.exe

Утилита для очистки Windows File System Cache (Standby List).

### Зачем нужна?

Windows кэширует прочитанные файлы в RAM (Standby List). После первого парсинга документации 1С (24979 файлов) всё кэшируется, и повторный парсинг занимает ~1 секунду вместо 1-2 минут. Это мешает тестировать отображение прогресса парсинга в реальном времени.

**EmptyStandbyList.exe** очищает этот кэш, позволяя увидеть прогресс парсинга заново.

### Как скачать?

**Вариант 1: Прямая ссылка**
```bash
cd tools
curl -L https://wj32.org/wp/download/releases/empty-standby-list/EmptyStandbyList.exe -o EmptyStandbyList.exe
```

**Вариант 2: Вручную**
1. Открой https://wj32.org/processhacker/forums/viewtopic.php?t=1569
2. Скачай `EmptyStandbyList.exe`
3. Помести в `tools/EmptyStandbyList.exe`

**Вариант 3: Альтернативный источник (GitHub)**
```bash
cd tools
# Если найдётся зеркало на GitHub, добавим сюда
```

### Проверка целостности

После скачивания проверь размер файла:
```bash
ls -lh tools/EmptyStandbyList.exe
# Ожидаемый размер: ~6-10 KB
```

### Использование

```bash
# Через наш скрипт (рекомендуется)
./scripts/clear_cache.sh

# Или вручную (требует Admin прав)
powershell -Command "Start-Process 'tools/EmptyStandbyList.exe' -ArgumentList 'standbylist' -Verb RunAs"
```

### Безопасность

- ✅ **Официальная утилита** от разработчика Process Hacker (wj32)
- ✅ **Open Source**: Исходный код доступен
- ✅ **Подпись**: Подписана цифровой подписью автора
- ✅ **Функциональность**: Только очищает Standby List, ничего больше

### Альтернативы

Если не хочешь использовать стороннюю утилиту:

**1. RAMMap (Microsoft Sysinternals)**
- Официальная утилита от Microsoft
- GUI интерфейс
- Скачать: https://docs.microsoft.com/en-us/sysinternals/downloads/rammap

**2. PowerShell скрипт**
- См. `scripts/clear_cache_powershell.ps1`
- Требует запуска от имени администратора

---

## Файл в .gitignore

`EmptyStandbyList.exe` добавлен в `.gitignore`, так как:
1. Бинарный файл (~6-10 KB)
2. Может быть скачан по требованию
3. Не является частью исходного кода проекта

Каждый разработчик скачивает утилиту самостоятельно при первом использовании.
