# Инструкции для AI-ассистента (BSL Gradual Types)

Этот `GEMINI.md` — короткий индекс. Полные правила и рабочие процессы лежат в `.claude/rules/` и `.claude/skills/`.

## TL;DR (частые команды)

```bash
# Сборка / тесты
./scripts/build-all.sh --release
# или быстрее без тестов:
./scripts/build-all.sh --release --skip-tests
# точечно (только Rust тесты):
cargo test --workspace

# Web API для отладки резолвинга типов: сервер запускаешь ты, я тестирую через curl
# (я сам сервер не запускаю и не останавливаю)
# Тебе: ./scripts/start-web-api.sh --build
curl -s http://localhost:3002/api/health
```

## Обязательные правила

- Отвечай на русском: `.claude/rules/project-specifics.md`
- Следуй принципам проекта (Right-Sized Architecture, Semantic IR, честная проверка): `.claude/rules/general.md`
- Перед отчётом о выполнении Milestone/задачи — подтверждай фактами (grep/read/tests): `docs/guides/roadmap-verification.md`

## Тестирование LSP через Web API

- Не запускай/не останавливай LSP/Web серверы сам (и не делай `pkill`); тестируй через `curl`, когда сервер поднят пользователем: `.claude/rules/web-api-testing.md`
- Полная справка по endpoints: `docs/api/web-api-reference.md`

## Навигация и архитектура

- Быстрые ссылки на документацию/roadmap: `.claude/rules/navigation.md`
- Архитектурная диаграмма и термины: `.claude/rules/architecture.md`

## Отладка

- Глубокая отладка Rust через MCP Debug: `.claude/rules/mcp-debug.md`
