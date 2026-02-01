# Дизайн: Rust CI full suite

## Подход
- Оставить существующий workflow `Repo policy` как набор быстрых репозиторных проверок (политики/ограничения).
- Добавить отдельный workflow `CI` (название условное) для Rust quality gates.

## Команды
- Format: `cargo fmt --all -- --check`
- Lint: `cargo clippy --workspace --all-targets --locked -- -D warnings`
- Tests: `cargo test --workspace --locked`

## Окружение / таргеты
- Первичная цель: host target на `ubuntu-latest`.
- Cross-compilation, Windows/macOS matrix и проверки VSCode extension — вне текущего change (при необходимости отдельными изменениями).
