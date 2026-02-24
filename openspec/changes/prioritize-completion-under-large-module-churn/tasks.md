## 1. Policy Contract
- [ ] 1.1 Зафиксировать в runtime policy формальное определение режима `large + churn` (критерии размера документа и активности правок).
- [ ] 1.2 Описать правила переключения профилей diagnostics для `large + churn` (fast на `didChange`, heavy на `idle/didSave`).

## 2. Runtime Scheduling
- [ ] 2.1 Реализовать интерактивный приоритет для completion/hover/signatureHelp в очереди runtime команд.
- [ ] 2.2 Добавить fairness-механику для background diagnostics, чтобы избежать starvation.

## 3. Observability
- [ ] 3.1 Добавить метрики входа/выхода из `large + churn` и причин отложенного heavy diagnostics.
- [ ] 3.2 Добавить drilldown-признаки, позволяющие отличать queue contention от query bottleneck после включения policy.

## 4. Validation
- [ ] 4.1 Добавить/обновить integration tests на переключение policy и приоритет интерактивного пути.
- [ ] 4.2 Прогнать scale-aware perf сценарий (`large/small`, `start/cold/warm`) и приложить JSON-отчет с pass/fail.
- [ ] 4.3 Выполнить `openspec validate prioritize-completion-under-large-module-churn --strict --no-interactive`.
