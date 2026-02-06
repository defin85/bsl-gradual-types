## 1. Contract: canonical payload + compatibility
- [x] 1.1 Зафиксировать канонический payload (camelCase) в спеках (`bsl-runtime-config`, `mcp-bsl-agent`)
- [x] 1.2 `bsl-agent`: принимать camelCase и legacy snake_case (serde alias), но возвращать camelCase в ответах
- [x] 1.3 Обновить/добавить тесты `bsl-agent` на camelCase payload + совместимость со snake_case

## 2. Mutability: runtime vs startup-only
- [x] 2.1 Добавить `mutability` в runtime registry (`KeySpec`) и в snapshot (machine-readable)
- [x] 2.2 `ApplyOverridesReport`: добавить список ключей, требующих рестарта для эффекта (если override менялся, но ключ startup-only)
- [x] 2.3 Тест(ы): snapshot содержит mutability; report содержит restart-needed keys для известных startup-only ключей

## 3. Observability tool for bsl-agent
- [x] 3.1 Добавить MCP tool `workspace_get_observability_metrics(session_id)` (только для ready-сессии)
- [x] 3.2 Зафиксировать ответ (DTO) и совместимость формата с LSP `bsl.getObservabilityMetrics`
- [x] 3.3 Тест(ы): tool доступен, возвращает JSON и отклоняет не-ready сессию

## 4. Docs
- [x] 4.1 Документировать примеры overrides (VS Code settings + MCP tool payload) и пояснить startup-only semantics

## 5. Quality gates
- [x] 5.1 `cargo test --workspace`
- [x] 5.2 `npm test` (vscode-extension)
