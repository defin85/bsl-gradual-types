## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Документация корректно описывает фактический состав CI
Документация MUST отражать фактическое состояние автоматизации: какие проверки выполняются в GitHub Actions (если есть) и какие проверки обязательны локально.

Документация MUST NOT создавать ожидание, что CI прогоняет `cargo fmt`/`cargo clippy`/`cargo test`, если этого нет. Если CI прогоняет этот набор проверок, документация MUST явно это указывать.

#### Scenario: README не вводит в заблуждение по статусу CI
- **GIVEN** в README есть упоминание CI/проверок
- **WHEN** разработчик следует инструкции
- **THEN** он должен получить корректные ожидания: что именно проверяет GitHub Actions и какие проверки нужно запускать локально (если такие остаются)
