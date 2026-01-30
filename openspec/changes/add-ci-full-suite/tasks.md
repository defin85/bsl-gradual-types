## 1. Implementation
- [ ] 1.1 Добавить GitHub Actions workflow для Rust quality gates (fmt/clippy/test) на `pull_request` и `push` в `master`.
- [ ] 1.2 Зафиксировать “чистые” режимы команд: `cargo fmt -- --check`, `cargo clippy/test --locked`, clippy с `-D warnings`.
- [ ] 1.3 Убедиться, что scope ограничен host target (обычная сборка), без cross-compilation и без платформенной матрицы.
- [ ] 1.4 Обновить README/CONTRIBUTING так, чтобы они описывали фактический состав CI (и не создавали ложных ожиданий).

## 2. Validation
- [ ] 2.1 `openspec validate add-ci-full-suite --strict --no-interactive`.
- [ ] 2.2 В PR-процессе остаётся единственный источник правды: рекомендации локальных проверок в `CONTRIBUTING.md` совпадают с тем, что реально запускается в CI.
