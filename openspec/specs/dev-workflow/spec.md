# Спецификация: dev-workflow

## Purpose
Требования к dev-циклу и репозиторию (политики артефактов, сборки, проверки), чтобы изменения исходников оставались чистыми и воспроизводимыми.
## Requirements
### Requirement: Генерированные артефакты VS Code extension не версионируются
Репозиторий MUST не хранить в git генерированные артефакты сборки VS Code extension (например, `vscode-extension/out/**`, `vscode-extension/*.vsix`).

#### Scenario: Чистый diff для исходников
- **GIVEN** разработчик меняет исходники расширения
- **WHEN** он делает commit
- **THEN** в diff не должны попадать генерированные файлы сборки, если они не являются исходниками

### Requirement: `Cargo.lock` отслеживается в git для воспроизводимости сборок
Репозиторий с бинарниками (сервер/CLI/LSP) MUST хранить `Cargo.lock` в git.

#### Scenario: Сборка повторяема на разных машинах
- **GIVEN** разработчик клонирует репозиторий на чистую машину
- **WHEN** он запускает `cargo build --workspace`
- **THEN** набор зависимостей должен определяться `Cargo.lock` (без неявного дрейфа версий)

### Requirement: Границы зависимостей workspace (library vs application)
Репозиторий MUST поддерживать слоистую архитектуру зависимостей (layered dependencies), где application/adapter крейты (например, web/LSP/MCP) зависят от библиотечных крейтов (domain/runtime), но не наоборот.

В частности, `bsl-agent` MUST NOT зависеть от `bsl-backend`. Общая логика startup/deps snapshot/кэша, необходимая и backend, и agent, MUST жить в отдельном библиотечном крейте (например, `bsl-runtime`).

#### Scenario: `bsl-agent` не тянет `bsl-backend` как зависимость
- **GIVEN** разработчик собирает workspace
- **WHEN** он проверяет дерево зависимостей для `bsl-agent`
- **THEN** `bsl-backend` отсутствует в зависимостях `bsl-agent` (прямых и транзитивных)

### Requirement: Декомпозиция `bsl-shared` (этап 1)
Система SHALL постепенно уменьшать связанность `bsl-shared`, выделяя базовые компоненты в отдельные library crates. На первом этапе:
- базовые доменные типы типовой системы SHOULD быть вынесены в `bsl-types`,
- интерфейсы/структуры репозитория типов и индексов SHOULD быть вынесены в `bsl-repository`.
- DTO для публичных контрактов (например, для HTTP/MCP parity) SHOULD быть вынесены в отдельный library crate (например, `bsl-api-dtos`), чтобы не смешивать контракты и доменную/инфраструктурную логику.

Миграция MUST быть поэтапной и сопровождаться тестами/quality gates, чтобы не ломать поведение анализа и внешние адаптеры.

#### Scenario: Workspace собирается и тесты проходят после выделения новых крейтов
- **GIVEN** выделены новые library crates и обновлены зависимости
- **WHEN** разработчик запускает `cargo test --workspace`
- **THEN** сборка проходит, а поведение (вывод и контракты) остаётся совместимым с текущими спецификациями

### Requirement: Документация и вспомогательные скрипты согласованы со структурой репозитория
Актуальные гайды и поддерживающие скрипты MUST не содержать ссылок на пути, отсутствующие в репозитории, если эти ссылки используются как «инструкция к действию» (команды, чек‑листы, «ожидаемые результаты»).

Под «актуальными гайдами» и документами, подпадающими под проверку, в рамках этого требования дополнительно понимаются:
- `backend/src/README.md` (как документ, который описывает структуру backend и используется как «инструкция к действию» для разработчиков)
- `architectural_report.md` (как источник актуальных ссылок на архитектурные артефакты/пути)

#### Scenario: Документация в корне и в backend не ссылается на несуществующий путь
- **GIVEN** в `backend/src/README.md` или `architectural_report.md` присутствуют ссылки на файлы исходного кода
- **WHEN** разработчик пытается открыть указанный файл по пути из документации
- **THEN** путь должен существовать в репозитории

### Requirement: Документация корректно описывает фактический состав CI
Документация MUST отражать фактическое состояние автоматизации: какие проверки выполняются в GitHub Actions (если есть) и какие проверки обязательны локально.

Документация MUST NOT создавать ожидание, что CI прогоняет `cargo fmt`/`cargo clippy`/`cargo test`, если этого нет. Если CI прогоняет этот набор проверок, документация MUST явно это указывать.

#### Scenario: README не вводит в заблуждение по статусу CI
- **GIVEN** в README есть упоминание CI/проверок
- **WHEN** разработчик следует инструкции
- **THEN** он должен получить корректные ожидания: что именно проверяет GitHub Actions и какие проверки нужно запускать локально (если такие остаются)

### Requirement: Тестовый набор разделён на smoke и manual/heavy с явными правилами
Проект MUST иметь документированное разделение тестов на:
- smoke (быстрые, без внешних фикстур, запускаются по умолчанию локально),
- manual/heavy (требуют внешних данных/подготовки или занимают значительное время).

#### Scenario: Разработчик запускает smoke тесты без подготовки окружения
- **GIVEN** чистый checkout репозитория
- **WHEN** разработчик запускает smoke suite
- **THEN** тесты должны пройти без необходимости скачивать/подготавливать внешние фикстуры

### Requirement: GitHub Actions выполняет базовые Rust quality gates
GitHub Actions MUST прогонять базовые проверки качества для Rust workspace на `pull_request` и `push` в `master`: форматирование (`cargo fmt`), линтинг (`cargo clippy`) и тесты (`cargo test`).

#### Scenario: PR блокируется при нарушении качества
- **GIVEN** PR меняет Rust-код в workspace
- **WHEN** запускается CI workflow
- **THEN** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` и `cargo test --workspace` должны проходить успешно

#### Scenario: CI не модифицирует lockfile
- **GIVEN** репозиторий с закоммиченным `Cargo.lock`
- **WHEN** CI запускает проверки
- **THEN** команды используют `--locked`, и сборка падает, если `Cargo.lock` не соответствует зависимостям

### Requirement: Проверка ссылок на пути в документации автоматизирована (MUST)
Система MUST иметь автоматическую проверку (локальную команду и/или CI), которая валидирует, что в документах «инструкция к действию» ссылки на пути существуют в репозитории.

#### Scenario: Документационный дрейф ловится до мержа
- **GIVEN** в документации изменены пути или переименованы файлы/модули
- **WHEN** разработчик запускает проверку документации (локально или в CI)
- **THEN** проверка падает, если документ ссылается на путь, которого нет в репозитории

### Requirement: Versioned внешние контракты хранятся в `contracts/**` (MUST)
Система MUST хранить публичные внешние контракты в versioned каталоге `contracts/**`.

Минимальная структура MUST включать:
- surface идентификатор в пути (`contracts/<surface>/...`);
- явную major версию (`contracts/<surface>/v1/...`);
- артефакты контракта в рамках версии (schema и/или эквивалентный формализованный формат + примеры).

#### Scenario: Контракт для внешней поверхности фиксируется как versioned артефакт
- **GIVEN** команда вводит/меняет внешний интерфейс (LSP/Web/MCP/observability labels)
- **WHEN** change подготавливается к merge
- **THEN** в `contracts/**` существует versioned contract артефакт для этой поверхности
- **AND** путь контракта содержит surface и номер major версии

### Requirement: Breaking изменения контракта требуют version bump и migration note (MUST)
Система MUST применять version policy к контрактам:
- breaking change MUST сопровождаться major version bump (`vN -> vN+1`);
- breaking change MUST содержать migration note в change/proposal или contract changelog.

#### Scenario: Breaking контрактный change не проходит без version bump
- **GIVEN** PR меняет contract shape/semantics обратно несовместимым образом
- **WHEN** выполняется контрактная проверка
- **THEN** проверка падает, если major версия не увеличена
- **AND** проверка падает, если отсутствует миграционная заметка

### Requirement: Versioned contracts проходят compatibility-diff проверку как manual gate (MUST)
Система MUST иметь compatibility-diff проверку для `contracts/**`, которая сравнивает baseline и candidate версии контрактов на semantic совместимость.

Проверка MUST:
- классифицировать изменения как `non_breaking` или `breaking` по формальной policy;
- выдавать machine-readable отчёт (`pass/fail`, `violations`, `compared_versions`);
- запускаться в manual режиме (`workflow_dispatch`/ручная команда) на текущем этапе rollout.

#### Scenario: Manual compatibility-diff gate формирует детерминированный отчёт
- **GIVEN** разработчик меняет контракт в `contracts/<surface>/vN/...`
- **WHEN** запускается manual compatibility-diff gate
- **THEN** система формирует детерминированный отчёт с классификацией изменений
- **AND** отчёт содержит `pass/fail` и список нарушений policy

### Requirement: Breaking compatibility diff требует major bump и migration note (MUST)
Если compatibility-diff классифицирует изменение как `breaking`, система MUST требовать major bump (`vN -> vN+1`).

Если major bump выполнен, система MUST требовать migration note в `contracts/<surface>/vN/changelog.md`.

#### Scenario: Breaking изменение без major bump отклоняется
- **GIVEN** baseline и candidate контракт имеют breaking diff
- **WHEN** major версия не увеличена
- **THEN** compatibility-diff gate завершается fail
- **AND** отчёт явно указывает причину: `breaking_without_major_bump`

#### Scenario: Major bump без migration note отклоняется
- **GIVEN** для contract surface выполнен major bump
- **WHEN** в `changelog.md` отсутствует migration note
- **THEN** compatibility-diff gate завершается fail
- **AND** отчёт явно указывает причину: `missing_migration_note`

### Requirement: Change criticality классифицируется детерминированно до запуска process-gates (MUST)
Система MUST выполнять детерминированную классификацию каждого change перед применением ADR/doc-first/perf gates.

Минимальный контракт классификации MUST включать:
- `change_criticality` из фиксированного enum: `routine`, `behavioral`, `architectural`, `perf_critical`;
- machine-readable артефакт (в change artifacts) с причиной классификации и rule-id;
- fail-closed режим при `unknown`/`missing` классификации.

Нормативное правило применения gate:
- ADR/doc-first/perf-resource gates MUST быть обязательными для `architectural` и `perf_critical`;
- для остальных классов применяются только соответствующие им обязательные workflow checks.

#### Scenario: Неопределённая criticality блокирует implementation
- **GIVEN** change не содержит валидный `change_criticality`
- **WHEN** запускается pre-implementation workflow gate
- **THEN** gate завершается fail с причиной `change_criticality_missing_or_unknown`
- **AND** implementation этап не считается разрешённым

### Requirement: Архитектурно-значимые и perf-critical изменения проходят ADR gate до реализации (MUST)
Для изменений, затрагивающих архитектурно-значимые решения (минимум: модель владения ресурсами, синхронизационные примитивы hot path, модель очередей/cancellation, IPC границы, cache topology), система MUST требовать утвержденный ADR до начала имплементации.

ADR MUST содержать:
- контекст и целевой ASR (latency/memory/contention/correctness);
- минимум две альтернативы и причины выбора;
- ожидаемые бюджеты и критерии успеха;
- rollback/supersede стратегию.

#### Scenario: Perf-critical change без принятого ADR блокируется
- **GIVEN** change затрагивает интерактивный completion hot path и меняет synchronization strategy
- **WHEN** запускается change-review gate
- **THEN** отсутствие принятого ADR приводит к fail
- **AND** implementation этап не считается разрешённым

### Requirement: Non-MVP perf changes выполняются по doc-first контракту (MUST)
Для non-MVP изменений с архитектурным и/или производительным эффектом система MUST требовать полный doc-first пакет до реализации:
- `proposal.md`
- `design.md`
- `tasks.md`
- spec deltas
- acceptance matrix с функциональными и perf проверками.

#### Scenario: Proposal без acceptance matrix не проходит в implementation
- **GIVEN** change помечен как non-MVP и perf-affecting
- **WHEN** выполняется pre-implementation проверка
- **THEN** gate завершается fail, если отсутствует acceptance matrix с критериями pass/fail

### Requirement: Backend/runtime behavioral changes выполняются через test-first цикл (MUST)
Изменения поведения backend/runtime MUST реализовываться через test-first цикл:
- сначала воспроизводимый failing test/contract baseline;
- затем реализация;
- затем минимальный refactor без изменения смысловых acceptance условий.

Система MUST рассматривать отсутствие test-first evidence как нарушение process gate.

Test-first evidence MUST быть machine-readable и включать минимум:
- ссылку на failing evidence до фикса (`failing_ref`);
- ссылку на passing evidence после фикса (`passing_ref`);
- связку `change_id` и `scope` проверяемого поведения;
- deterministic reason-code при провале.

#### Scenario: Реализация без воспроизводимого failing test отклоняется
- **GIVEN** PR меняет поведение runtime анализа
- **WHEN** проверяется trace change-to-test
- **THEN** gate завершается fail, если нет зафиксированного failing test/contract baseline до фикса

#### Scenario: Test-first evidence отсутствует в machine-readable формате
- **GIVEN** backend/runtime behavioral change помечен как test-first required
- **WHEN** workflow gate проверяет evidence artifact
- **THEN** gate завершается fail с причиной `test_first_evidence_missing_or_invalid`
- **AND** merge блокируется до предоставления валидного evidence

### Requirement: Protected acceptance assets immutable в implementation change (MUST)
Система MUST защищать protected acceptance assets (ключевые acceptance tests, versioned contracts, perf baselines) от ad-hoc изменений в рамках implementation change.

Если изменение protected assets действительно необходимо, оно MUST выполняться отдельным согласованным change с явной мотивацией и migration note.

#### Scenario: Подгонка тестов под реализацию блокируется
- **GIVEN** implementation change модифицирует protected acceptance tests без отдельного approved change
- **WHEN** запускается protected-assets gate
- **THEN** gate завершается fail с причиной `protected_acceptance_asset_modified`
- **AND** merge блокируется до согласованного test/contract update path

### Requirement: Perf-critical merge gate требует resource evidence, а не только latency (MUST)
Для perf-critical изменений система MUST требовать детерминированные before/after артефакты с минимумом метрик:
- latency (`p50/p95/p99` для целевого interactive пути);
- allocations (количество и/или bytes per operation);
- lock contention / lock wait.
- для latency одновременно MUST проверяться два условия: относительный порог к baseline и абсолютный ceiling (SLO/budget), утвержденный в ADR/spec.

Gate MUST падать при отсутствии обязательных артефактов или выходе за утверждённые бюджеты.

#### Scenario: Latency улучшилась, но allocation budget нарушен
- **GIVEN** change показывает лучшее latency в warm профиле
- **WHEN** perf merge gate анализирует resource evidence
- **THEN** gate завершается fail, если allocations выходят за budget
- **AND** change не принимается до корректировки реализации или явного обновления budget через ADR

#### Scenario: Ratio к baseline проходит, но абсолютный latency ceiling нарушен
- **GIVEN** ratio latency к baseline укладывается в относительный порог
- **AND** абсолютный `p95` или `p99` превышает утвержденный ceiling
- **WHEN** perf merge gate анализирует отчёт
- **THEN** gate завершается fail с причиной превышения абсолютного latency budget
- **AND** merge блокируется до оптимизации или явного обновления budget через ADR

### Requirement: Bootstrap policy для initial perf budgets детерминирован и fail-closed (MUST)
Система MUST иметь явную bootstrap policy для первичного включения blocking perf gate.

Нормативные требования:
- blocking perf gate MUST NOT включаться без зафиксированных budget thresholds в versioned baseline contract;
- bootstrap расчёт initial budgets MUST быть формально описан (sample size, aggregation rule, профили `small|large|churn`);
- итоговые budget значения MUST быть записаны в versioned contract artifact и утверждены через ADR;
- при отсутствии bootstrap artifacts/verdict gate MUST завершаться fail-closed.

#### Scenario: Blocking gate отклоняет change без зафиксированного initial budget
- **GIVEN** perf-critical change пытается включить blocking mode
- **WHEN** baseline contract не содержит утверждённых initial resource/latency budgets
- **THEN** gate завершается fail с причиной `initial_budget_not_fixed`
- **AND** blocking mode не активируется

### Requirement: Option B является единственной архитектурой perf-gate (MUST)
Система MUST реализовывать perf-gate только через dedicated perf-gate module и versioned schema contract.

Нормативные требования:
- evaluator логика MUST находиться в одном выделенном модуле и вызываться всеми consumers (CI/harness/runtime checks);
- пороги/правила verdict MUST NOT дублироваться inline в `lsp_server` core или в helper-скриптах;
- schema contract для perf-gate MUST быть versioned в `contracts/intellisense-perf-gate/vN/**` и включать минимум `input`, `baseline`, `report`;
- breaking schema change MUST сопровождаться major version bump и migration note.

#### Scenario: Inline/per-script verdict логика блокируется
- **GIVEN** PR добавляет новый порог perf-verdict только в CI скрипт, минуя dedicated evaluator module
- **WHEN** выполняется workflow policy gate
- **THEN** gate завершается fail с причиной `perf_gate_architecture_violation`
- **AND** merge блокируется до переноса логики в dedicated module и schema contract

#### Scenario: Breaking schema без version bump отклоняется
- **GIVEN** изменена структура `report` schema для perf-gate обратно несовместимым способом
- **WHEN** запускается compatibility-diff для `contracts/intellisense-perf-gate/vN/**`
- **THEN** проверка завершается fail без major bump и migration note

