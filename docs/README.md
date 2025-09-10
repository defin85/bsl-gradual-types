# 📚 Документация BSL Gradual Type System

Документация проекта системы градуальной типизации для языка 1С:Предприятие BSL.

## 🚀 Быстрый старт

1. **[Обзор целевой архитектуры](reference/target_architecture/overview.md)** - понимание системы
2. **[Дорожная карта](reference/target_architecture/roadmap.md)** - текущий статус и планы
3. **[Центральная система](reference/target_architecture/central_type_system.md)** - ключевые интерфейсы и контракты
4. **[План миграции](reference/target_architecture/migration_plan.md)** - стратегия перехода без ломки

## 📂 Структура документации

### 📚 Основные разделы
- **[API](api/)** — документация API
- **[Guides](guides/)** — руководства и инструкции
- **[Reference](reference/)** — справочные материалы
- **[Tutorials](tutorials/)** — обучающие материалы
- **[Images](images/)** — графические файлы
- **[Assets](assets/)** — дополнительные ресурсы

### 🎯 [Целевая архитектура](reference/target_architecture/)
Прагматичная версия «революционной» архитектуры:
- **[README.md](reference/target_architecture/README.md)** — Введение и структура
- **[overview.md](reference/target_architecture/overview.md)** — Слои и компоненты
- **[central_type_system.md](reference/target_architecture/central_type_system.md)** — Центральная система
- **[roadmap.md](reference/target_architecture/roadmap.md)** — Дорожная карта
- **[migration_plan.md](reference/target_architecture/migration_plan.md)** — Переход без ломки

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

## 🔧 Режим выполнения

Проект работает в целевом режиме (target) на основе `SystemCoordinator`. Переключатель движка legacy/target удалён. Для настройки загрузки конфигурации используйте опцию `--config path/to/cf` в соответствующих бинарях (например, `bsl-web-server`, `type-check`, `bsl-analyzer`).


## 📖 Дополнительные ресурсы

- [Корневой README](../README.md) - общая информация о проекте
- [CLAUDE.md](../CLAUDE.md) - инструкции для AI-ассистента
- [Примеры кода](../examples/) - примеры использования
