## 1. Baseline & Protocol
- [x] 1.1 Зафиксировать versioned baseline-артефакт для профилей `large` и `small` в формате `start/cold/warm` с обязательными метриками completion-контура.
- [x] 1.2 Добавить единый runner/сценарий, который воспроизводимо собирает эти профили в одной сессии и сохраняет machine-readable отчёт.

## 2. Scale-Aware Quality Gate
- [x] 2.1 Реализовать gate-проверку ratio-targets для `large` профиля относительно baseline (`completion_duration_ms`, `wait_for_file_version_completion_ms`).
- [x] 2.2 Реализовать отдельный non-regression guard для `small` профиля, чтобы ускорение на `large` не ухудшало интерактивность на лёгких файлах.
- [x] 2.3 Добавить fail-fast отчёт по стадиям (`wait_for_file_version`, `snapshot`, `ir_query`) для локализации bottleneck при падении gate.

## 3. Large-Module Bottleneck Reduction
- [x] 3.1 Уменьшить вклад стадии `wait_for_file_version_completion` в warm-path `large` профиле до целевого ratio относительно baseline.
- [x] 3.2 Подтвердить, что итоговое улучшение `completion_duration_ms` на `large` достигает целевого ratio и сохраняет стабильность cancel/incomplete-rate.

## 4. Validation
- [x] 4.1 Прогнать `start/cold/warm` для `large` и `small`, приложить итоговые JSON-артефакты и pass/fail summary.
- [x] 4.2 Обновить/добавить regression tests, проверяющие scale-aware gate в CI.
- [x] 4.3 Выполнить `openspec validate add-large-module-completion-acceleration-gate --strict --no-interactive`.
