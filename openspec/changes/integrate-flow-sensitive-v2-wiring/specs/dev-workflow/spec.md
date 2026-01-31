## ADDED Requirements

### Requirement: Проверка ссылок на пути в документации выполняется в CI (MUST)
Репозиторий MUST запускать проверку ссылок на пути в документации “инструкция к действию” в CI (GitHub Actions), чтобы документационный дрейф ловился до мержа.

Проверка MUST использовать репозиторный скрипт `scripts/check-doc-paths.py` и список целей `scripts/doc-path-check-targets.txt`.

#### Scenario: CI падает на несуществующем пути в документации
- **GIVEN** в одном из документов из `scripts/doc-path-check-targets.txt` добавлена ссылка на путь, которого нет в репозитории
- **WHEN** запускается GitHub Actions workflow repo policy
- **THEN** job проверки doc-paths падает и блокирует merge

