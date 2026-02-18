## ADDED Requirements

### Requirement: didChange-path diagnostics ограничен дешёвыми инкрементальными шагами (MUST)
Система MUST обрабатывать `textDocument/didChange` через fast-path, который не запускает полный тяжёлый semantic пересчёт на каждый символ.

Для fast-path на `didChange`:
- MUST выполняться только дешёвые и инкрементальные шаги оркестрации (применение версии/состояния, минимальные локальные проверки);
- MUST NOT выполняться тяжёлые стадии полного diagnostics-пайплайна;
- MUST сохраняться совместимость с существующим strict-latest publish контрактом (см. требование про diagnostics publish latest-version).

#### Scenario: Частое редактирование не запускает полный heavy diagnostics на каждый символ
- **GIVEN** пользователь быстро вводит текст и генерирует серию `didChange` (`V`, `V+1`, `V+2`, ...)
- **WHEN** LSP обрабатывает входящие события
- **THEN** на каждом событии выполняется только fast-path
- **AND** тяжёлые проверки не запускаются синхронно на каждый `didChange`

### Requirement: Тяжёлые diagnostics стадии выполняются deferred с debounce и background class (MUST)
Система MUST выполнять полный тяжёлый diagnostics путь в deferred-профиле:
- запуск через debounce для coalescing серий `didChange`;
- выполнение в background CPU class;
- новая версия документа MUST supersede устаревший deferred запуск.

Система MUST проверять актуальность версии/поколения перед каждой дорогой стадией и прекращать устаревшую задачу до publish.

#### Scenario: Более новая версия supersede устаревший deferred запуск
- **GIVEN** запущен deferred diagnostics для версии `V`
- **AND** до завершения приходит `didChange` для версии `V+1`
- **WHEN** deferred задача для `V` доходит до следующей тяжёлой стадии
- **THEN** задача для `V` завершается как устаревшая (`superseded`) без публикации
- **AND** система продолжает обработку только актуального запуска

### Requirement: Diagnostics publish проверяет revision token с generation (MUST)
Публикация diagnostics MUST выполняться только при совпадении актуального revision token:
- `file_version`;
- `deps_id`;
- `settings_id`;
- `diagnostics_generation` (или эквивалентного monotonic токена запуска).

Результат для устаревшего token MUST NOT публиковаться и MUST NOT перезаписывать более новый publish.

#### Scenario: Устаревший запуск не может перезаписать актуальные diagnostics
- **GIVEN** heavy diagnostics запуск для поколения `G` и версии `V`
- **AND** затем пришла новая версия, создавшая поколение `G+1`
- **WHEN** запуск `G` завершается позже
- **THEN** publish для `G` отклоняется
- **AND** опубликованным остаётся только результат для актуального поколения

### Requirement: Дорогие проверки запускаются только по didSave и/или idle trigger (MUST)
Система MUST отделять expensive проверки от fast `didChange` пути.

Expensive-проверки MUST запускаться:
- по `textDocument/didSave`, если событие доступно;
- либо по `idle` trigger после отсутствия новых `didChange` в течение конфигурируемого окна.

Эти проверки MUST NOT быть обязательной частью каждого `didChange` запуска.

#### Scenario: Heavy-проверки выполняются после паузы или сохранения
- **GIVEN** пользователь печатает без сохранения
- **WHEN** идут последовательные `didChange`
- **THEN** heavy-проверки не выполняются на каждый символ
- **AND** heavy-проверки запускаются только после `didSave` или достижения `idle` окна

### Requirement: Observability фиксирует diagnostics trigger/profile/supersede причины (MUST)
Канонический observability контракт MUST фиксировать diagnostics pipeline по low-cardinality измерениям:
- `trigger` (`did_change|did_open|did_save|idle`);
- `profile` (`fast|debounced_full|idle_heavy`);
- `reason` (`published|superseded_version|superseded_generation|cancelled` минимум).

Dual-write MUST оставаться детерминированным из канонического event model: drilldown как primary, legacy как projection.

#### Scenario: Метрики показывают, что устаревшие перезапуски отфильтрованы до publish
- **GIVEN** серия `didChange` порождает superseded запуски
- **WHEN** запрашивается observability snapshot
- **THEN** в метриках видны события с `trigger=did_change` и `profile=debounced_full`
- **AND** superseded причины отражены отдельными low-cardinality значениями `reason`
- **AND** отсутствует publish stale-результата
