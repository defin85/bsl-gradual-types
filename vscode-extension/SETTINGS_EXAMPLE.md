# Пример настроек BSL Analyzer v1.11.0

## Новая структура с вложенными объектами

Теперь настройки BSL Analyzer организованы в логические группы с настоящей вложенностью в VS Code.

### Пример settings.json с новой структурой:

```json
{
    "bslAnalyzer": {
        "enabled": true,
        "general": {
            "enableRealTimeAnalysis": true,
            "maxFileSize": 2097152
        },
        "server": {
            "mode": "stdio",
            "tcpPort": 8080,
            "trace": "off"
        },
        "binaries": {
            "useBundled": true,
            "path": ""
        },
        "index": {
            "configurationPath": "C:\\MyProject\\conf",
            "platformVersion": "8.3.25",
            "platformDocsArchive": "C:\\1C\\rebuilt.shcntx_ru.zip",
            "autoIndexBuild": true
        },
        "analysis": {
            "rulesConfig": "",
            "enableMetrics": true
        }
    }
}
```

## Структура настроек:

### 1. Основная настройка
- `bslAnalyzer.enabled` - включить/выключить анализатор

### 2. Группа `general` - Основные настройки
- `bslAnalyzer.general.enableRealTimeAnalysis` - анализ в реальном времени
- `bslAnalyzer.general.maxFileSize` - максимальный размер файла (в байтах)

### 3. Группа `server` - Настройки LSP сервера
- `bslAnalyzer.server.mode` - режим работы ("stdio" или "tcp")
- `bslAnalyzer.server.tcpPort` - TCP порт (1-65535)
- `bslAnalyzer.server.trace` - уровень трассировки ("off", "messages", "verbose")

### 4. Группа `binaries` - Управление бинарными файлами
- `bslAnalyzer.binaries.useBundled` - использовать встроенные бинарники
- `bslAnalyzer.binaries.path` - путь к внешним бинарникам

### 5. Группа `index` - Настройки индексации
- `bslAnalyzer.index.configurationPath` - путь к конфигурации 1С
- `bslAnalyzer.index.platformVersion` - версия платформы (формат: X.X.X)
- `bslAnalyzer.index.platformDocsArchive` - путь к архиву документации
- `bslAnalyzer.index.autoIndexBuild` - автоматическая сборка индекса

### 6. Группа `analysis` - Настройки анализа
- `bslAnalyzer.analysis.rulesConfig` - путь к файлу правил
- `bslAnalyzer.analysis.enableMetrics` - включить метрики качества

## Как изменить настройки:

### Через UI:
1. Откройте настройки VS Code (`Ctrl+,`)
2. Найдите "BSL Analyzer"
3. Настройки будут сгруппированы по категориям

### Через settings.json:
1. Откройте Command Palette (`Ctrl+Shift+P`)
2. Введите "Preferences: Open Settings (JSON)"
3. Добавьте объект `bslAnalyzer` с нужными настройками

## Миграция со старых настроек

При первом запуске расширение автоматически предложит мигрировать старые настройки на новую структуру.