# 🚀 BSL Analyzer - Инструкция по установке

## 📦 Готовый пакет
**Файл:** `bsl-analyzer-1.0.0.vsix` (21 KB)  
**Версия:** 1.0.0  
**Дата:** 2025-08-04

## ⚡ Быстрая установка

### Шаг 1: Установка расширения
1. Скопируйте файл `bsl-analyzer-1.0.0.vsix` в удобное место
2. Откройте VSCode
3. Нажмите `Ctrl+Shift+P`
4. Введите: "Extensions: Install from VSIX..."
5. Выберите файл `bsl-analyzer-1.0.0.vsix`
6. Нажмите "Install"
7. Перезапустите VSCode

### Шаг 2: Настройка путей (ОБЯЗАТЕЛЬНО!)
Откройте настройки VSCode (`Ctrl+,`) и настройте:

```json
{
  "bslAnalyzer.indexServerPath": "C:\\1CProject\\bsl_type_safety_analyzer\\target\\debug",
  "bslAnalyzer.configurationPath": "C:\\path\\to\\your\\1c\\configuration",
  "bslAnalyzer.platformVersion": "8.3.25"
}
```

### Шаг 3: Первый запуск
1. Откройте любой .bsl файл (или создайте test.bsl)
2. Нажмите `Ctrl+Shift+P`
3. Введите: "BSL Index: Build Unified BSL Index"
4. Дождитесь завершения индексации

## ✅ Проверка установки

### Проверьте статус расширения:
- В статус баре должно появиться: "BSL Analyzer: Ready"
- В Command Palette (`Ctrl+Shift+P`) найдите команды "BSL Index", "BSL Verification"

### Тест функциональности:
```
Ctrl+Shift+P → "BSL Index: Search BSL Type"
Введите: "Массив"
→ Должен открыться Webview с информацией о типе
```

## 🔧 Требования

### Обязательные файлы в target/debug/:
- ✅ `lsp_server.exe`
- ✅ `build_unified_index.exe` 
- ✅ `query_type.exe`
- ✅ `check_type_compatibility.exe`
- ✅ `incremental_update.exe`

### Если файлы отсутствуют:
```bash
cd "C:\1CProject\bsl_type_safety_analyzer"
cargo build --release
# Затем укажите path: target/release вместо target/debug
```

## 🎯 Доступные команды

После установки доступно **14 команд**:

### BSL Index (6 команд)
- Search BSL Type
- Search Method in Type  
- Build Unified BSL Index
- Show Index Statistics
- Incremental Index Update
- Explore Type Methods & Properties

### BSL Verification (2 команды)
- Validate Method Call
- Check Type Compatibility

### BSL Analyzer (6 команд)
- Analyze Current File
- Analyze Workspace
- Generate Reports
- Show Code Quality Metrics
- Configure Rules
- Restart LSP Server

## 🐛 Решение проблем

### "Command not found"
→ Проверьте настройку `bslAnalyzer.indexServerPath`

### "Configuration not found"  
→ Проверьте настройку `bslAnalyzer.configurationPath`

### "LSP Server not starting"
→ Проверьте наличие `lsp_server.exe` в указанной директории

### Расширение не активируется
→ Проверьте Output → BSL Analyzer на ошибки

## 🎉 Готово!

После успешной установки вы получите:
- 🔍 Поиск по 24,055+ BSL типам
- ⚡ Real-time диагностику кода
- 📊 Проверку совместимости типов
- 🎯 Валидацию вызовов методов
- 📈 Метрики качества кода

---
**BSL Analyzer v1.0** - Professional BSL development tools for VSCode