# Change: Полный набор CI проверок для Rust (GitHub Actions)

## Why
Сейчас GitHub Actions выполняет только проверки политики репозитория (workflow `Repo policy`), а `cargo fmt`/`cargo clippy`/`cargo test` прогоняются локально. Это:
- повышает риск регрессий (особенно при внешних контрибьютах);
- усложняет ревью (невозможно опираться на единый набор автоматических гейтов);
- создаёт разрыв между ожиданиями и фактической автоматизацией.

## What Changes
- Добавить отдельный GitHub Actions workflow (например, `.github/workflows/ci.yml`) для Rust quality gates, который запускается на `pull_request` и `push` в `master`.
- В рамках workflow выполнить:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked`
- При необходимости обновить документацию (README/CONTRIBUTING), чтобы она отражала новый состав проверок в CI и не противоречила фактам.

## Impact
- Affected specs: `dev-workflow`
- Affected code:
  - `.github/workflows/*` (добавление нового workflow)
  - возможно, README/CONTRIBUTING (уточнение формулировок)
- Риски:
  - увеличение времени CI на PR (компенсируется предсказуемостью и ранним отловом проблем)
  - необходимость держать команды CI в синхронизации с локальными рекомендациями из `CONTRIBUTING.md`

## Non-Goals
- Запуск perf/интеграционных сценариев, требующих внешних ресурсов (Syntax Helper, большие фикстуры) в CI.
- Платформенная матрица (Windows/macOS) и сборка WASM/VSCode extension — отдельные изменения, если понадобятся.
