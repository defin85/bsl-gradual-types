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
- `bd sync`: Sync state

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

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   - `git pull --rebase`
   - `bd sync`
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
- `bd sync` - Sync with git (run at session end)

For full workflow details: `bd prime`