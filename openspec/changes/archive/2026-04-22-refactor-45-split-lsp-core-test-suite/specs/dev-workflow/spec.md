## ADDED Requirements

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
