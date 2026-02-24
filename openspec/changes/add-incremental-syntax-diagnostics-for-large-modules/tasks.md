## 1. Incremental Parsing Contract
- [ ] 1.1 Специфицировать API и lifecycle incremental parse (предыдущее дерево, применение edit, parse новой ревизии).
- [ ] 1.2 Зафиксировать критерии валидности edit-преобразования и условия fallback на full parse.

## 2. Integration in v2 Pipeline
- [ ] 2.1 Внедрить incremental parse путь в syntax diagnostics контур для последовательных ревизий одного файла.
- [ ] 2.2 Обеспечить детерминированный fallback на full parse при невозможности incremental обновления.

## 3. Observability
- [ ] 3.1 Добавить метрики incremental hit/miss/fallback и причины fallback.
- [ ] 3.2 Добавить stage-level сравнение latency incremental vs full parse для large профиля.

## 4. Validation
- [ ] 4.1 Добавить regression tests на эквивалентность diagnostics (incremental vs full parse).
- [ ] 4.2 Добавить perf regression тесты/сценарии для large/small профилей.
- [ ] 4.3 Выполнить `openspec validate add-incremental-syntax-diagnostics-for-large-modules --strict --no-interactive`.
