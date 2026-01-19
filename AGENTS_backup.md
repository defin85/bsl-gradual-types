<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# Инструкции для AI-ассистента (BSL Gradual Types)

Этот `AGENTS.md` — короткий индекс. Полные правила и рабочие процессы лежат в `.claude/rules/` и `.claude/skills/`.

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

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
