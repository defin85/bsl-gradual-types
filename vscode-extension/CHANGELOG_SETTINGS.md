# Изменения настроек BSL Analyzer v1.10.0

## Новая структура настроек с группировкой

Настройки были реорганизованы в логические группы для удобства использования:

### 1. Основные настройки (`bslAnalyzer.general.*`)
- `bslAnalyzer.general.enabled` - Включить BSL Analyzer
- `bslAnalyzer.general.enableRealTimeAnalysis` - Анализ в реальном времени
- `bslAnalyzer.general.maxFileSize` - Максимальный размер файла для анализа

### 2. Настройки LSP сервера (`bslAnalyzer.server.*`)
- `bslAnalyzer.server.serverPath` - Путь к LSP серверу
- `bslAnalyzer.server.serverMode` - Режим работы (stdio/tcp)
- `bslAnalyzer.server.tcpPort` - TCP порт
- `bslAnalyzer.server.trace` - Уровень трассировки

### 3. Управление бинарными файлами (`bslAnalyzer.binaries.*`)
- `bslAnalyzer.binaries.useBundledBinaries` - Использовать встроенные бинарники
- `bslAnalyzer.binaries.indexServerPath` - Путь к внешним бинарникам

### 4. Настройки индексации (`bslAnalyzer.index.*`)
- `bslAnalyzer.index.configurationPath` - Путь к конфигурации 1С
- `bslAnalyzer.index.platformVersion` - Версия платформы 1С
- `bslAnalyzer.index.platformDocsArchive` - Путь к архиву документации
- `bslAnalyzer.index.autoIndexBuild` - Автоматическая сборка индекса

### 5. Настройки анализа (`bslAnalyzer.analysis.*`)
- `bslAnalyzer.analysis.rulesConfig` - Конфигурация правил
- `bslAnalyzer.analysis.enableMetrics` - Метрики качества кода

## Обратная совместимость

Расширение автоматически мигрирует старые настройки на новый формат при первом запуске.

### Мапинг старых настроек на новые:

| Старая настройка | Новая настройка |
|-----------------|-----------------|
| `bslAnalyzer.enabled` | `bslAnalyzer.general.enabled` |
| `bslAnalyzer.enableRealTimeAnalysis` | `bslAnalyzer.general.enableRealTimeAnalysis` |
| `bslAnalyzer.maxFileSize` | `bslAnalyzer.general.maxFileSize` |
| `bslAnalyzer.serverPath` | `bslAnalyzer.server.serverPath` |
| `bslAnalyzer.serverMode` | `bslAnalyzer.server.serverMode` |
| `bslAnalyzer.tcpPort` | `bslAnalyzer.server.tcpPort` |
| `bslAnalyzer.trace.server` | `bslAnalyzer.server.trace` |
| `bslAnalyzer.useBundledBinaries` | `bslAnalyzer.binaries.useBundledBinaries` |
| `bslAnalyzer.indexServerPath` | `bslAnalyzer.binaries.indexServerPath` |
| `bslAnalyzer.configurationPath` | `bslAnalyzer.index.configurationPath` |
| `bslAnalyzer.platformVersion` | `bslAnalyzer.index.platformVersion` |
| `bslAnalyzer.platformDocsArchive` | `bslAnalyzer.index.platformDocsArchive` |
| `bslAnalyzer.autoIndexBuild` | `bslAnalyzer.index.autoIndexBuild` |
| `bslAnalyzer.rulesConfig` | `bslAnalyzer.analysis.rulesConfig` |
| `bslAnalyzer.enableMetrics` | `bslAnalyzer.analysis.enableMetrics` |

## Улучшения

- ✅ Логическая группировка настроек
- ✅ Улучшенные описания на русском языке
- ✅ Автоматическая миграция старых настроек
- ✅ Поддержка обратной совместимости
- ✅ Валидация формата версии платформы
- ✅ Подсказки для enum значений