## 1. Contract: canonical payload + compatibility
- [ ] 1.1 Зафиксировать канонический payload (camelCase) в спеках (`bsl-runtime-config`, `mcp-bsl-agent`)
- [ ] 1.2 `bsl-agent`: принимать camelCase и legacy snake_case (serde alias), но возвращать camelCase в ответах
- [ ] 1.3 Обновить/добавить тесты `bsl-agent` на camelCase payload + совместимость со snake_case

## 2. Mutability: runtime vs startup-only
- [ ] 2.1 Добавить `mutability` в runtime registry (`KeySpec`) и в snapshot (machine-readable)
- [ ] 2.2 `ApplyOverridesReport`: добавить список ключей, требующих рестарта для эффекта (если override менялся, но ключ startup-only)
- [ ] 2.3 Тест(ы): snapshot содержит mutability; report содержит restart-needed keys для известных startup-only ключей

## 3. Observability tool for bsl-agent
- [ ] 3.1 Добавить MCP tool `workspace_get_observability_metrics(session_id)` (только для ready-сессии)
- [ ] 3.2 Зафиксировать ответ (DTO) и совместимость формата с LSP `bsl.getObservabilityMetrics`
- [ ] 3.3 Тест(ы): tool доступен, возвращает JSON и отклоняет не-ready сессию

## 4. Docs
- [ ] 4.1 Документировать примеры overrides (VS Code settings + MCP tool payload) и пояснить startup-only semantics

## 5. Quality gates
- [ ] 5.1 `cargo test --workspace`
- [ ] 5.2 `npm test` (vscode-extension)

