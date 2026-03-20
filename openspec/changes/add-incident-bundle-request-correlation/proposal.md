# Change: request-centric observability incident bundle summary

## Почему
После добавления export bundle и `Completion Timeline v6` raw-вложения уже позволяют разбирать completion-инциденты без ручного копирования UI-панелей. Однако текущий `incident.json` остаётся слишком бедным для машинного анализа:
- не фиксирует `uri` capture window даже когда все traces относятся к одному документу;
- не даёт request-centric списка проблемных completion requests;
- не переносит ключевые bounded latency facts из authoritative timeline в machine-readable summary;
- не использует уже доступные client probe timestamps для явного client/server edge correlation там, где это можно сделать без выдумывания данных.

В результате AI/incident handoff всё ещё требует ручного чтения `raw/completion_timeline.json` и `raw/client_probes.json`.

## Что меняется
- Добавить в export bundle request-centric derived report поверх уже существующих raw attachments.
- Ввести в `incident.json` и `summary.md` capture scope:
  - `uri`, если capture window относится к одному документу;
  - `request_count` и bounded request list для authoritative completion traces.
- Добавить bounded per-request summary для server trace:
  - request identity;
  - ключевые server-edge latency facts;
  - dominant bottleneck verdicts;
  - bounded prepare/exact deadline facts, если они присутствуют.
- Добавить optional client/server correlation facts, когда probe и trace можно сопоставить детерминированно без guesswork.
- Явно маркировать случаи, когда correlation недоступен или неоднозначен, вместо `null`-подобной пустоты.

## Не входит в scope
- Новый server-side custom request.
- Новый timeline contract version.
- Реконструкция authoritative server trace из client probes.
- `metrics delta` между двумя snapshot'ами: bundle по-прежнему использует один cumulative metrics snapshot.
- Новый отдельный UI surface; используются существующие export flow и bundle files.

## Влияние
- Затронутые спеки:
  - `bsl-intellisense`
- Затронутый код:
  - `vscode-extension/src/providers/observabilityIncidentBundle.ts`
  - `vscode-extension/src/test/suite/observabilityIncidentBundle.test.ts`
  - `vscode-extension/src/test/suite/observabilityCommands.test.ts`
  - `scripts/run-intellisense-tests.sh`
  - `scripts/test-intellisense-readiness-assets.py`
  - `vscode-extension/manual-lsp-test.md`
