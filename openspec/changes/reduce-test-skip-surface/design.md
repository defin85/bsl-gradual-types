# Дизайн: тестовые уровни и управление `#[ignore]`

## Политика
- `#[ignore]` допустим только при наличии явной причины (external fixture / long runtime / flaky / depends on unfinished feature).
- Для каждого ignored теста должна быть документированная команда запуска и prerequisites.

## Рекомендуемая структура
- smoke: быстрый набор для ежедневной разработки (по аналогии с `scripts/run-intellisense-tests.sh smoke`).
- full/manual: расширенный набор, допускающий heavy фикстуры (Syntax Helper и т.п.).

