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

# Unified Workflow

We operate in a cycle: **OpenSpec (What) → Beads (How) → Code (Implementation)**.

## 1. Intent Formation

The user initiates with:
`/openspec-proposal "Add 2FA authentication"`

OpenSpec creates a change folder (`openspec/changes/<change-id>/`) containing:

- `proposal.md`: Business value and scope.
- `tasks.md`: High-level task list.
- `design.md`: Technical design (optional).
- `specs/.../spec.md`: Requirements and acceptance criteria.

**Agent Goal**: Edit these files until they represent a signable contract.

**DO NOT proceed to step 2 until you are explicitly told the keyword "Go!" in English.**

## 2. Task Transformation

Once the change is approved, execute the agent command:
`/openspec-to-beads <change-id>`

The agent must:

1.  Read the change files.
2.  Create a Beads Epic for the feature. Include a short description summarizing the intent and referencing the change folder (e.g., "See openspec/changes/<change-id>/").
3.  Create Beads Tasks for each item in `tasks.md`. Include a brief description for each task to provide context (why this issue exists and what needs to be done).
4.  Set dependencies (e.g., Infra blocks Backend blocks Frontend).

Result: A **live task graph in `.beads/`**, not just text.

## 3. Execution

Work loop:

- `bd ready`: Check actionable tasks
- `bd show <task-id>`: Get task context
- Implement code
- `bd close <task-id>`: Complete task
- `bd vc status`: Check Beads VC state (Dolt)
- `bd vc commit -m "..."`: Commit pending Beads changes when needed

**Rule**: Only work on tasks listed in `bd ready`.

## 4. Fixation

When all tasks are complete, execute the agent commands:

- `/openspec-apply <change-id>`: Verify code meets specs.
- Then, when ready,
- `/openspec-archive <change-id>`: Archive the change.

---

## Agent Mental Checklist

1.  **Start**: Is there an active OpenSpec change?
    - No? → Create one (`/openspec-proposal`).
    - Yes? → Read `proposal.md` and `tasks.md`.
2.  **Plan**: Are tasks tracked in Beads?
    - No? → Generate graph (`/openspec-to-beads`).
    - Yes? → Work from `bd ready`.
3.  **Align**: Keep OpenSpec (Intent) ↔ Beads (Plan) ↔ Code (Reality) in sync.

---

## OpenSpec Delivery Contract (Mandatory)

- Before coding for an OpenSpec change, build an execution matrix from `spec.md` requirements/scenarios to target files and tests.
- Every MUST/Requirement/Scenario must have automated evidence (`test`) or an explicitly approved exception from the user.
- Statuses `partially implemented` or `not implemented` for mandatory requirements block task completion and hand-off.
- For API surface changes, update all relevant layers together: backend view/urls, `contracts/orchestrator/src/**`, aggregated `contracts/orchestrator/openapi.yaml`, and frontend generated client/types when applicable.
- Async requirements must include a real async boundary (queue/worker/workflow). Synchronous execution in request path does not satisfy async requirements.
- Integration/source requirements must use real runtime integration paths. Metadata/mock paths are allowed only for tests or explicitly approved temporary modes.
- If any mandatory requirement cannot be delivered now, stop and escalate with concrete blockers and options; do not silently ship a partial implementation.
- Final delivery report must include `Requirement -> Code -> Test` evidence with concrete file paths.

---

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   - `git pull --rebase`
   - `bd vc status`
   - if there are pending Beads changes: `bd vc commit -m "..."`
   - `git push`
   - `git status` - MUST show "up to date with origin"
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Issue Tracking

This project uses **bd (beads)** for issue tracking.
Run `bd prime` for workflow context.

**Quick reference:**

- `bd ready` - Find unblocked work
- `bd create "Title" --type task --priority 2 --description "..."` - Create ad-hoc issue
- `bd close <task-id>` - Complete work
- `bd vc status` - Check Dolt VC status
- `bd vc commit -m "..."` - Commit pending Beads changes (if any)

For full workflow details: `bd prime`

### Beads Dolt Server Mode (текущий репозиторий)

Актуальный режим в этом репозитории: `dolt_mode: "server"` (`.beads/metadata.json`) + shared `beads-dolt.service`.

Ключевые правила:

- `bd sync` — deprecated/no-op, не использовать как шаг синхронизации.
- Базовая проверка окружения: `./debug/start-dolt.sh` и `bd doctor --server`.
- Проверка сервиса: `systemctl --user status beads-dolt.service --no-pager`.
- Для фиксации изменений использовать `bd vc status` / `bd vc commit`.
- В этом репозитории **не использовать Dolt remote/store** и не выполнять `bd dolt pull/push`.

## Semantic Search Playbook

Use this checklist for semantic code search in this repository.

### Search Order

1. Use `mcp__claude-context__search_code` first (semantic search).
2. Use `grep` second (exact/pattern search in known areas).
3. Use glob/filename pattern search last.

### Default Query Preset

- Always set `extensionFilter: [".rs"]` for first-pass code discovery.
- Start with `limit: 5-8`.
- Use intent-focused queries (behavior + domain terms), not only symbol names.
- If results are noisy, add concrete context keywords in the query:
  `TypeResolver`, `SemanticValidationVisitor`, `AnalysisHostV2`, `lsp_server`.

### Starter Query Templates

Use these as ready-to-run prompts for `search_code`:

1. `где реализована проверка совместимости типов аргументов вызова`
2. `построение diagnostics для неизвестного метода или свойства`
3. `понижение severity unknown member access до warning`
4. `сужение типа после проверки ТипЗнч в условии`
5. `entry point LSP diagnostics analysis pipeline`
6. `как формируется TypeResolution для member access`
7. `парсинг bsl модулей и построение индекса модулей конфигурации`
8. `где используется TypeMetadataLookup для проверки методов и свойств`

### Noise Control / Reindexing

If semantic search returns too many docs/test artifacts, reindex with `force=true`
and ignore these paths:

- `docs/**`
- `openspec/**`
- `vscode-extension/.vscode-test/**`
- `**/target/**`
- `**/node_modules/**`
- `**/dist/**`
- `**/build/**`
- `**/coverage/**`
- `**/tests/**`
- `examples/**`
