## ADDED Requirements

### Requirement: VS Code sidebar использует один activity bar контейнер для BSL Analyzer (MUST)
Система MUST регистрировать sidebar расширения в одном activity bar container для BSL Analyzer.

`Overview`, `Diagnostics`, `Type Repository`, `Quick Actions` и `Cache Dashboard` MUST быть доступны внутри этого единого container.

#### Scenario: Пользователь видит один вход в sidebar расширения
- **GIVEN** расширение активировано в VS Code
- **WHEN** пользователь открывает Activity Bar
- **THEN** отображается один container BSL Analyzer
- **AND** внутри него доступны разделы overview/diagnostics/type repository/quick actions/cache dashboard

### Requirement: Счётчики типов консистентны между sidebar виджетами (MUST)
Система MUST формировать счётчики `TypeRepository` (`total`, `platform`, `configuration`) из единого snapshot/revision источника.

`Overview`, `Type Repository` и `Quick Actions` MUST отображать согласованные значения для одного и того же snapshot состояния.

#### Scenario: Platform count совпадает в Overview, Type Repository и Quick Actions
- **GIVEN** sidebar обновлён на одном snapshot type repository
- **WHEN** пользователь сравнивает значения в `Overview`, `Type Repository` и `Quick Actions`
- **THEN** platform/config/total counts не противоречат друг другу

### Requirement: Summary diagnostics в sidebar согласован с фактическим списком diagnostics (MUST)
Система MUST обеспечивать, что summary (`Issues Found`) и содержимое раздела `Diagnostics` рассчитываются из согласованного источника данных в рамках одного snapshot.

Система MUST NOT показывать одновременно "No issues found" и ненулевой summary issues для одного и того же состояния.

#### Scenario: Summary и diagnostics tree не противоречат
- **GIVEN** workspace snapshot содержит N диагностик
- **WHEN** пользователь смотрит `Overview` и `Diagnostics`
- **THEN** summary отражает те же diagnostics, что и дерево по severity
- **AND** не возникает конфликтующих статусов "issues > 0" и "No issues found"

### Requirement: Quick Actions использует live-метрики вместо статических значений (MUST)
Система MUST получать отображаемые счётчики в Quick Actions из live-данных LSP/TypeRepository и MUST NOT использовать хардкодные числовые значения.

#### Scenario: Счётчик типов в Quick Actions обновляется после изменения индекса
- **GIVEN** количество platform types изменилось после переиндексации
- **WHEN** пользователь открывает/обновляет Quick Actions
- **THEN** отображается актуальное значение из live-метрик, а не фиксированный хардкод

### Requirement: User-facing sidebar UI не показывает сырые internal tokens (MUST)
Система MUST отображать статусы и иконки в sidebar через корректные UI-примитивы VS Code и MUST NOT показывать неотрендеренные токены формата `$(...)` в пользовательских строках.

#### Scenario: Статус сервера отображается без сырых токенов
- **GIVEN** статус LSP сервера равен Running
- **WHEN** пользователь открывает раздел `LSP Server Status`
- **THEN** статус отображается как корректный UI-текст/иконка
- **AND** строка не содержит сырых фрагментов вроде `$(check)`
