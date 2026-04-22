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

### Requirement: Production Rust source files MUST быть не больше 1000 LOC
Репозиторий MUST поддерживать ограничение размера production Rust исходников: каждый production `.rs` файл MUST быть `<=1000` строк.

Под production scope понимаются `.rs` файлы за исключением:
- `third_party/**`;
- `**/target/**`;
- `**/node_modules/**`;
- тестовых и вспомогательных путей (`tests`, `benches`, `examples`, `fixtures`, `mocks`).

#### Scenario: Size gate блокирует файл больше 1000 строк
- **GIVEN** разработчик изменяет production Rust файл
- **WHEN** запускается size-gate проверка
- **THEN** проверка завершается fail, если файл имеет `>1000 LOC`
- **AND** merge блокируется до декомпозиции файла

### Requirement: Large-file target files MUST укладываться в LLM-friendly budget
Для файлов, входящих в inventory large-file refactor change, система MUST применять дополнительный budget, чтобы файл можно было анализировать LLM целиком:
- `LOC <= 800`;
- `bytes <= 80 KiB`;
- `tokens <= 12000` по `o200k_base` токенизации.

Проверка MUST выполняться специальным script-based gate и быть детерминированной.

#### Scenario: Target file превышает токен/байт budget и отклоняется
- **GIVEN** файл входит в target inventory large-file refactor
- **WHEN** выполняется LLM-budget gate
- **THEN** gate завершается fail, если нарушен любой из лимитов (`LOC`, `bytes`, `tokens`)
- **AND** merge блокируется до дальнейшей декомпозиции

### Requirement: Large-file refactor MUST быть behavior-preserving
Кампании декомпозиции крупных production Rust файлов MUST выполняться без изменения наблюдаемого поведения системы.

Это означает:
- публичные контракты (LSP/Web/MCP/CLI) MUST оставаться совместимыми;
- существующие acceptance/regression/perf проверки MUST оставаться неизменными, кроме отдельных согласованных change;
- изменения должны ограничиваться структурой модулей и ответственностей, а не бизнес-семантикой.

#### Scenario: Рефакторинг меняет внешний контракт и отклоняется
- **GIVEN** PR заявлен как large-file behavior-preserving refactor
- **WHEN** contract/regression проверки обнаруживают несовместимое изменение поведения
- **THEN** gate завершается fail
- **AND** merge блокируется до восстановления поведенческой паритетности

### Requirement: Для large-file refactor MUST существовать inventory и parity matrix
Для каждого change, направленного на декомпозицию крупных файлов, MUST быть зафиксированы:
- inventory целевых файлов с исходным размером;
- план batch-декомпозиции с зависимостями;
- parity validation matrix (минимум: compile/lint/tests и релевантные интеграционные проверки).

Progress change MUST измеряться уменьшением inventory до полного отсутствия target файлов `>1000 LOC` в production scope.

#### Scenario: Change без inventory/parity matrix не проходит review gate
- **GIVEN** создан change для декомпозиции крупных файлов
- **WHEN** выполняется pre-implementation review
- **THEN** gate завершается fail, если отсутствует inventory или parity validation matrix

### Requirement: Inline tests MUST быть вынесены из production Rust файлов
В рамках large-file refactor кампании inline тестовые модули в production `.rs` файлах (например `#[cfg(test)] mod tests`) MUST быть вынесены в отдельные test paths.

#### Scenario: Production файл содержит inline test module и отклоняется
- **GIVEN** PR заявлен как large-file behavior-preserving refactor
- **WHEN** policy gate обнаруживает inline test module в production `.rs`
- **THEN** gate завершается fail
- **AND** merge блокируется до переноса тестов в отдельные test paths

### Requirement: Change completion MUST NOT завышать readiness относительно MUST backlog
Система MUST иметь readiness gate, который запрещает считать OpenSpec change фактически complete, если по его MUST-требованиям остаётся открытый критический follow-up backlog.

Gate MUST сверять как минимум:
- статус checklist / validation в change;
- traceability matrix;
- review-gate verdict или эквивалентный audit artifact;
- связанный критический Beads backlog, созданный для закрытия тех же MUST-требований.

Если критический follow-up backlog существует, change MUST быть явно помечен как `partial`, `not ready` или эквивалентно незавершённый до закрытия этого backlog либо до утверждённого superseding delivery path.

#### Scenario: Open follow-up epic блокирует честный verdict `complete`
- **GIVEN** review change выявил недоставленные MUST-требования
- **AND** для них создан критический Beads epic/task graph
- **WHEN** команда пытается считать исходный change complete только по checklist и validation
- **THEN** readiness gate отклоняет verdict `complete`
- **AND** требует явного partial/not-ready статуса или approved superseding delivery path

### Requirement: Traceability и review artifacts MUST отражать реальные gaps без optimistic overclaim (MUST)
Traceability matrix, review-gate и связанные acceptance artifacts MUST отражать реальный статус MUST-требований без optimistic overclaim.

Если evidence показывает `partial` или `gap`, артефакты MUST NOT маркировать требование как `covered` или `pass` без дополнительного подтверждённого delivery evidence.

#### Scenario: Conflicting evidence не допускает optimistic `covered`
- **GIVEN** traceability или review artifact утверждает `covered/pass`
- **AND** другой approved evidence artifact показывает открытый gap по тому же MUST-требованию
- **WHEN** readiness gate сверяет evidence
- **THEN** optimistic verdict отклоняется
- **AND** artefact должен быть исправлен до handoff или archive

### Requirement: Репозиторий предоставляет curated agent-facing documentation surface для Codex (MUST)
Репозиторий MUST поддерживать отдельный канонический слой agent-facing документации под `docs/agent/`, предназначенный для нового Codex-агента.

Минимальный состав этого слоя MUST включать:
- `index.md` как единый стартовый индекс;
- `architecture-map.md` с картой workspace, entry points и ссылками на source-of-truth документы;
- `verification.md` с каноническим run/test/verify contract;
- `task-artifacts.md` с картой OpenSpec/Beads/CI/runtime артефактов и способом трассировки `Requirement -> Code -> Test`.

Этот слой MUST отвечать как минимум на вопросы:
- что это за проект;
- как устроен workspace;
- где основные entry points;
- как запускать, тестировать и верифицировать изменения.

#### Scenario: Новый Codex-агент находит стартовую карту из одного индекса
- **GIVEN** агент впервые входит в репозиторий без накопленного локального контекста
- **WHEN** он открывает `docs/agent/index.md`
- **THEN** он получает ссылки на каноническую карту архитектуры, verify runbook и карту task artifacts без необходимости начинать с исторических roadmap-документов

### Requirement: Инструкции `AGENTS.md` слоисты и пригодны для Codex-first onboarding (MUST)
Репозиторий MUST использовать layered instruction model для `AGENTS.md`.

Root `AGENTS.md` MUST:
- быть коротким dispatcher/index документом;
- описывать только глобальный workflow и корневые правила;
- ссылаться на канонический `docs/agent/index.md`;
- явно указывать, в каких директориях есть area-specific инструкции.

High-friction зоны с отдельным toolchain, entry points или verify path MUST иметь локальные `AGENTS.md`. Минимальный набор таких зон в рамках этого требования:
- `backend/`
- `bsl-agent/`
- `vscode-extension/`

`AGENTS.override.md` MUST использоваться только для intentional override родительских инструкций, а не как общий механизм добавления локальных заметок.

#### Scenario: Агент переходит в backend и получает локальные инструкции
- **GIVEN** новый Codex-агент начинает работу из директории `backend/`
- **WHEN** он определяет активную instruction chain
- **THEN** root `AGENTS.md` даёт короткий глобальный контекст
- **AND** `backend/AGENTS.md` даёт backend-specific entry points, verify commands и карту важных файлов без дублирования полного project playbook

### Requirement: Agent verification runbook является исполнимым и использует живые runtime surfaces (MUST)
Репозиторий MUST иметь канонический agent-facing verification runbook, который использует фактические текущие binary/package names и классифицирует проверки по стоимости.

Runbook MUST:
- документировать живые entry commands для `bsl-cli`, `bsl-web-server`, `bsl-lsp-server` и `bsl-agent`;
- разделять проверки минимум на `smoke` и `manual/heavy`;
- явно фиксировать prerequisites и expected outcomes;
- быть согласованным с актуальными CI/manual gates и readiness checks.

#### Scenario: Новый агент выполняет smoke path без археологии по README
- **GIVEN** чистый checkout репозитория и подготовленные минимальные prerequisites
- **WHEN** агент следует каноническому agent verification runbook
- **THEN** он находит один согласованный smoke path для запуска и проверки проекта
- **AND** этот путь использует текущие binary/package names, а не исторические или удалённые команды

### Requirement: Codex setup и recurring workflows оформлены как portable agent-facing артефакты (MUST)
Репозиторий MUST иметь канонический portable setup path для Codex и repo-local skills для повторяющихся agent workflow.

Portable setup path MUST:
- использовать sanitized examples без machine-specific абсолютных путей и без секретов;
- объяснять, как подключать поддерживаемые repo-local MCP/tooling integration;
- ссылаться на канонический agent-facing runbook, а не дублировать его.

Repo-local skills MUST существовать под `.agents/skills/` как минимум для следующих recurring workflows:
- workspace verification;
- `bsl-agent` MCP bootstrap/smoke;
- OpenSpec delivery matrix / `Requirement -> Code -> Test` evidence;
- audit drift в agent-facing документации.

#### Scenario: Агент поднимает Codex bootstrap без локально-зашитой конфигурации другого разработчика
- **GIVEN** новый Codex-агент работает в чистом локальном окружении
- **WHEN** он следует каноническому setup path и использует repo-local skills
- **THEN** ему не требуется зависеть от machine-specific checked-in конфигурации или секретов
- **AND** повторяющиеся workflow доступны как переиспользуемые skills

### Requirement: Drift в agent-facing документации и командах ловится машинно до merge (MUST)
Репозиторий MUST иметь machine-checkable validation для agent-facing documentation surface и документированных onboarding-команд.

Эта validation MUST как минимум ловить:
- ссылки на отсутствующие пути в agent-facing и первичных onboarding-документах;
- устаревшие package/bin names и broken documented commands;
- отсутствие канонических agent docs и ожидаемой instruction layering.

Validation MUST быть доступна как локальная команда и SHALL подключаться к CI/manual gate.

#### Scenario: Устаревшая команда в onboarding-доке не проходит validation
- **GIVEN** в `README.md`, `docs/README.md`, `docs/BUILD_GUIDE.md` или `docs/guides/development-workflow.md` появляется устаревшая команда или несуществующий binary name
- **WHEN** запускается agent-facing docs validation
- **THEN** проверка завершается fail до merge
- **AND** отчёт явно указывает, какой документ и какая команда больше не соответствуют фактическому workspace

### Requirement: Catastrophic detached Rust test suites MUST быть декомпозированы в directory modules
Repo-owned detached Rust test modules MUST NOT оставаться монолитными, если они превышают
`10_000 LOC`.

Для такого suite refactor MUST использовать directory-module layout (`tests/mod.rs` или
семантически эквивалентный вариант) с:

- themed child modules;
- shared support module для harness/helpers;
- scope только в repo-owned test paths.

Из policy scope исключаются:

- `third_party/**`;
- `**/target/**`;
- `**/node_modules/**`;
- generated/vendor paths вне repo-owned test sources.

#### Scenario: Detached test suite больше 10k LOC не остаётся в одном плоском файле
- **GIVEN** repo-owned detached Rust test module превышает `10_000 LOC`
- **WHEN** выполняется agreed refactor этого suite
- **THEN** change MUST разложить его в directory module с themed child modules и shared support
- **AND** catastrophic monolith MUST NOT оставаться одним плоским `tests.rs`

### Requirement: Detached test-suite decomposition MUST сохранять test selectors и validation surface
Behavior-preserving decomposition detached Rust test suite MUST сохранять существующие test
function names / selectors и текущую targeted validation surface, если отдельный approved change
явно не меняет acceptance assets.

Это означает:

- selector-based команды `cargo test ... <test_name>` MUST продолжать работать;
- split MUST NOT silently weaken acceptance coverage только ради новой файловой структуры;
- rename/remove selector требует отдельной явной мотивации и обновления acceptance artifacts.

#### Scenario: Split сохраняет invokable targeted selector
- **GIVEN** до refactor существует targeted команда `cargo test ... <existing_test_name>`
- **WHEN** detached test suite разложен по child modules
- **THEN** тот же selector остаётся invokable после split

#### Scenario: Неподтверждённое переименование acceptance selector отклоняется
- **GIVEN** decomposition change переименовывает или удаляет существующий targeted selector
- **WHEN** для этого нет отдельного approved change, обновляющего acceptance assets
- **THEN** parity / review gate завершается fail
- **AND** merge блокируется до восстановления selector parity или явного approved superseding path

