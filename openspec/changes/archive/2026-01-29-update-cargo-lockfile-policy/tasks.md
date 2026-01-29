## 1. Реализация
- [x] 1.1 Убрать `Cargo.lock` из `.gitignore` и при необходимости обновить связанные игноры/документацию.
- [x] 1.2 Добавить/обновить раздел в `docs/project_structure.md` и/или `CONTRIBUTING.md` про политику `Cargo.lock`.
- [x] 1.3 Убедиться, что `Cargo.lock` добавлен в git и не конфликтует с существующими workflow/скриптами.

## 2. Валидация
- [x] 2.1 `git check-ignore -v Cargo.lock` не возвращает правил.
- [x] 2.2 `cargo build --workspace` проходит без обновления зависимостей (ожидаемо при валидном lockfile).
- [x] 2.3 `openspec validate update-cargo-lockfile-policy --strict --no-interactive`.
