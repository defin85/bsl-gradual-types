# Навигация по документации

## Roadmap и прогресс

| Документ | Описание |
|----------|----------|
| [ROADMAP_2025.md](ROADMAP_2025.md) | Актуальный план развития проекта |
| [ROADMAP_ARCHIVE_2025.md](ROADMAP_ARCHIVE_2025.md) | Архив завершённых Milestones (13 этапов) |
| [docs/guides/roadmap-verification.md](docs/guides/roadmap-verification.md) | Правила проверки выполнения |

## Руководства разработчика

| Документ | Описание |
|----------|----------|
| [docs/guides/development-workflow.md](docs/guides/development-workflow.md) | Команды cargo/npm/bash, сборка, тестирование |
| [docs/guides/tooling-guide.md](docs/guides/tooling-guide.md) | MCP инструменты, ast-grep, sourcebot |

## Архитектура

| Документ | Описание |
|----------|----------|
| [docs/architecture/type_system_architecture.md](docs/architecture/type_system_architecture.md) | Система типов + визуальная диаграмма (Mermaid) |
| [docs/architecture/milestones-history.md](docs/architecture/milestones-history.md) | История Milestone 2.8-2.18 |
| [docs/architecture/components-detailed.md](docs/architecture/components-detailed.md) | Детальные компоненты |

## API и интеграция

| Документ | Описание |
|----------|----------|
| [docs/api/web-api-reference.md](docs/api/web-api-reference.md) | Web API endpoints с примерами curl |

## Научная база

| Документ | Описание |
|----------|----------|
| [docs/reference/scientific-basis.md](docs/reference/scientific-basis.md) | Balyuk & Popova (2021) |

## Общая документация

| Документ | Описание |
|----------|----------|
| [docs/README.md](docs/README.md) | Главный навигатор всей документации |

## Структура проекта

```
bsl-gradual-types/
├── ROADMAP_2025.md              # Актуальный roadmap
├── ROADMAP_ARCHIVE_2025.md      # Архив Milestones
│
├── .claude/
│   ├── rules/                   # Правила для Claude (этот файл)
│   │   ├── general.md
│   │   ├── web-api-testing.md
│   │   ├── mcp-debug.md
│   │   ├── skills.md
│   │   ├── project-specifics.md
│   │   └── navigation.md
│   │
│   └── skills/                  # Автоматизированные навыки
│       ├── build.md
│       ├── test-runner.md
│       ├── api-tester.md
│       └── roadmap-checker.md
│
└── docs/
    ├── README.md                # Главный навигатор
    │
    ├── guides/                  # Практические руководства
    │   ├── development-workflow.md
    │   ├── roadmap-verification.md
    │   └── tooling-guide.md
    │
    ├── architecture/            # Архитектурные описания
    │   ├── type_system_architecture.md
    │   ├── milestones-history.md
    │   └── components-detailed.md
    │
    ├── api/                     # API документация
    │   └── web-api-reference.md
    │
    └── reference/               # Справочные материалы
        └── scientific-basis.md
```
