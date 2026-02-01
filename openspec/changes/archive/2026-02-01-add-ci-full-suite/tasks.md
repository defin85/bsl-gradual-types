## 1. Implementation
- [x] 1.1 Добавить GitHub Actions workflow для Rust quality gates (fmt/clippy/test) на `pull_request` и `push` в `master`.
- [x] 1.2 Зафиксировать “чистые” режимы команд: `cargo fmt -- --check`, `cargo clippy/test --locked`, clippy с `-D warnings`.
- [x] 1.3 Убедиться, что scope ограничен host target (обычная сборка), без cross-compilation и без платформенной матрицы.
- [x] 1.4 Обновить README/CONTRIBUTING так, чтобы они описывали фактический состав CI (и не создавали ложных ожиданий).

## 2. Validation
- [x] 2.1 `openspec validate add-ci-full-suite --strict --no-interactive`.
- [x] 2.2 В PR-процессе остаётся единственный источник правды: рекомендации локальных проверок в `CONTRIBUTING.md` совпадают с тем, что реально запускается в CI.
