## 1. Discovery (inventory)
- [ ] 1.1 Сформировать полный список runtime `BSL_*` (только те, что читаются в рантайме)
- [ ] 1.2 Разметить каждый ключ: тип, дефолт, компонент(ы) использования, tier (`stable`/`dev-only`)

## 2. Shared runtime-config registry
- [ ] 2.1 Добавить общий модуль/крейтовый слой для реестра ключей и typed accessors
- [ ] 2.2 Реализовать merge: `defaults` < `env bootstrap` < `runtime overrides`
- [ ] 2.3 Добавить валидацию overrides: только известные `BSL_*`, неизвестные -> ошибка/diagnostic

## 3. LSP server: runtime updates without restart
- [ ] 3.1 Добавить settings payload для `BSL_*` overrides (stable + dev-only) в секцию `bsl`
- [ ] 3.2 На `workspace/didChangeConfiguration` применять overrides в runtime-config store
- [ ] 3.3 Заменить чтение `std::env::var("BSL_*")` в LSP пути на runtime-config accessors
- [ ] 3.4 Добавить тест(ы), что изменение overrides влияет на поведение без рестарта

## 4. VS Code extension: управляемость из Settings UI
- [ ] 4.1 Добавить settings для stable overrides и отдельные settings для dev-only overrides
- [ ] 4.2 Убедиться, что extension синхронизирует секции `bsl` и прокидывает overrides в LSP
- [ ] 4.3 Документировать примеры настройки overrides в README/CONFIG

## 5. bsl-agent: runtime update tool
- [ ] 5.1 Добавить MCP tool `workspace_update_settings` (по `session_id`) без перезапуска
- [ ] 5.2 Принять тот же JSON payload, что и LSP settings (stable + dev-only overrides)
- [ ] 5.3 Применять overrides к активной сессии и persist их
- [ ] 5.4 Добавить тест(ы) tool-call -> update -> эффект на поведение/метрики

## 6. Observability & metrics unification
- [ ] 6.1 Вынести/переиспользовать единую схему метрик (snapshot JSON) для LSP и bsl-agent
- [ ] 6.2 Сделать metrics/trace toggles частью runtime-config (stable/dev-only)

## 7. Quality gates
- [ ] 7.1 `cargo test --workspace`
- [ ] 7.2 `npm test` (vscode-extension) (smoke)

