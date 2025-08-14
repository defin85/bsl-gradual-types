# 📚 Документация BSL Gradual Type System

Документация проекта системы градуальной типизации для языка 1С:Предприятие BSL.

## 🚀 Быстрый старт

Если вы новичок в проекте, начните с:

1. **[Обзор архитектуры](architecture/overview.md)** - понимание системы
2. **[Дорожная карта](development/roadmap.md)** - текущий статус и планы
3. **[Ключевые решения](decisions/DESIGN_DECISIONS.md)** - почему так, а не иначе

## 📂 Структура документации

### 🏗️ [Архитектура](architecture/)
Описание архитектуры системы и её компонентов:
- **[overview.md](architecture/overview.md)** - Общий обзор архитектуры
- **[EVOLUTIONARY_TYPE_SYSTEM_ARCHITECTURE.md](architecture/EVOLUTIONARY_TYPE_SYSTEM_ARCHITECTURE.md)** - Эволюционный план развития

### 🎨 [Дизайн](design/)
Дизайн-решения и концепции:
- **[FACET_SYSTEM_DESIGN.md](design/FACET_SYSTEM_DESIGN.md)** - Фасетная система для множественных представлений типов
- **[ALTERNATIVE_TYPE_SYSTEM_APPROACHES.md](design/ALTERNATIVE_TYPE_SYSTEM_APPROACHES.md)** - Рассмотренные альтернативные подходы

### 💡 [Решения](decisions/)
Обоснование принятых архитектурных решений:
- **[DESIGN_DECISIONS.md](decisions/DESIGN_DECISIONS.md)** - Ключевые архитектурные решения
- **[UNIFIED_TYPE_SYSTEM_COMPILED_REVIEW.md](decisions/UNIFIED_TYPE_SYSTEM_COMPILED_REVIEW.md)** - Анализ проблем и решений

### 🔧 [Реализация](implementation/)
Детали реализации компонентов:
- **[parser-syntax-helper.md](implementation/parser-syntax-helper.md)** - Парсер синтакс-помощника платформы
- **[visualization.md](implementation/visualization.md)** - Система визуализации типов

### 👨‍💻 [Разработка](development/)
Информация для разработчиков:
- **[roadmap.md](development/roadmap.md)** - Дорожная карта и прогресс разработки
- **[migration.md](development/migration.md)** - План миграции из старого репозитория
- **[testing.md](development/testing.md)** - Организация тестирования

### 🗄️ [Архив](archive/)
Устаревшие документы предыдущих итераций разработки

## 📊 Текущий статус проекта

**Версия**: 0.4.0  
**Завершённые фазы**: Phase 1, 2, 3, 3.5, 3.6  
**Следующая фаза**: Phase 4 (Flow-sensitive анализ)

### Статистика системы типов:
- **4361** типов платформы
- **276** категорий с правильными названиями
- **6975** методов объектов
- **13357** свойств объектов
- **476** глобальных функций
- **712** системных перечислений

## 🎯 Ключевые концепции

### TypeResolution
Центральная абстракция - не "тип", а "разрешение типа" с уровнем уверенности:
- **Known** - тип известен на 100%
- **Inferred(0.0-1.0)** - тип выведен с определённой уверенностью
- **Unknown** - тип невозможно определить

### Фасетная система
Один тип может иметь разные представления (фасеты):
- **Manager** - менеджер для работы со справочником
- **Object** - изменяемый объект
- **Reference** - ссылка на элемент
- **Metadata** - описание структуры

### Градуальная типизация
Комбинация статического анализа и runtime контрактов:
- Статические типы где возможно
- Runtime проверки где необходимо
- Плавный переход от динамической к статической типизации

## 🛠️ Основные команды

```bash
# Визуализация системы типов
cargo run --example visualize_parser_v3

# Анализ BSL файла
cargo run --bin bsl-analyzer -- --file module.bsl

# Запуск LSP сервера
cargo run --bin lsp-server

# Тесты
cargo test
```

## 📖 Дополнительные ресурсы

- [Корневой README](../README.md) - общая информация о проекте
- [CLAUDE.md](../CLAUDE.md) - инструкции для AI-ассистента
- [Примеры кода](../examples/) - примеры использования