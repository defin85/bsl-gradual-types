## ADDED Requirements

### Requirement: Коммерческий trust/legal пакет обязателен для GA (MUST)
Для коммерческого GA система MUST иметь versioned buyer-facing документы:
- `EULA` (лицензионные условия),
- `PRIVACY` (обработка данных/телеметрии),
- `SUPPORT` (каналы и SLA/режим поддержки),
- `SECURITY` (vulnerability disclosure policy).

Документы MUST быть доступны из репозитория и MUST быть связаны из основного user-facing onboarding документа.

#### Scenario: Покупатель получает полный набор обязательных документов
- **GIVEN** опубликован кандидат в коммерческий релиз
- **WHEN** технический и юридический reviewer открывают репозиторий/маркетплейс-материалы
- **THEN** они находят `EULA`, `PRIVACY`, `SUPPORT`, `SECURITY` и ссылки на них из onboarding-документа

### Requirement: GA onboarding должен быть воспроизводимым и проверяемым (MUST)
Система MUST иметь короткий onboarding сценарий, который на чистом окружении приводит пользователя к рабочему состоянию без ручных ad-hoc шагов.

Onboarding MUST явно включать:
- prereqs,
- установку extension и запуск LSP,
- минимальную проверку работоспособности (completion/diagnostics на тестовом BSL-файле),
- ссылку на runtime overrides и licensing policy.

#### Scenario: Новый пользователь проходит onboarding без участия разработчика
- **GIVEN** чистая машина с установленными prereqs
- **WHEN** пользователь следует quickstart шагам
- **THEN** extension и LSP запускаются, а базовые IntelliSense функции работают на контрольном примере

### Requirement: Коммерческий релиз требует формального GA-checklist (MUST)
Каждый коммерческий релиз MUST проходить формальный checklist перед публикацией.

Checklist MUST содержать минимум:
- подтверждение прохождения quality gates,
- подтверждение консистентности docs/settings,
- подтверждение наличия release integrity артефактов,
- подтверждение согласованности с текущей licensing policy.

#### Scenario: Релиз без checklist не считается GA-ready
- **GIVEN** подготовлен релизный тег
- **WHEN** checklist не заполнен или не подписан ответственными ролями
- **THEN** релиз не публикуется как коммерческий GA
