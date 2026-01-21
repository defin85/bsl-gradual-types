## ADDED Requirements

### Requirement: Runtime registry для discovery HTTP UI (multi-instance в одном `BSL_CACHE_DIR`)
Система SHALL поддерживать “runtime discovery registry” для HTTP UI `bsl-agent`, чтобы клиент мог узнать фактический адрес и порт UI (включая случай автопорта `127.0.0.1:0`) без парсинга логов.

Когда HTTP UI включён (например, через `BSL_AGENT_HTTP_ADDR`) и успешно привязан (bind), `bsl-agent` SHALL записывать registry запись с фактическим `ui_url` в директории состояния, производной от `BSL_CACHE_DIR` (state root), так чтобы несколько параллельных инстансов в одном `BSL_CACHE_DIR` не конфликтовали между собой.

#### Scenario: Инстанс с автопортом записывает фактический порт в registry
- **GIVEN** запущен `bsl-agent` с `BSL_AGENT_HTTP_ADDR=127.0.0.1:0` и HTTP UI успешно стартовал
- **WHEN** процесс завершил bind
- **THEN** в state root появляется registry запись, содержащая `ui_url` вида `http://localhost:<port>` с фактическим портом

### Requirement: CLI discovery через `bsl-agent ui ...`
Система SHALL предоставлять CLI сабкоманды в бинарнике `bsl-agent` для discovery HTTP UI:
- `bsl-agent ui list` (список кандидатов),
- `bsl-agent ui url` (получить URL одного инстанса).

`bsl-agent ui url` SHALL печатать plain `http://localhost:<port>` (без лишнего текста), чтобы вывод мог использоваться в скриптах.

#### Scenario: Единственный инстанс возвращает URL
- **GIVEN** в registry есть ровно один “живой” инстанс HTTP UI
- **WHEN** пользователь запускает `bsl-agent ui url`
- **THEN** команда печатает `http://localhost:<port>` и завершается успешно

### Requirement: Безопасное поведение при неоднозначности (ошибка при >1)
Если в registry найдено более одного “живого” инстанса и пользователь не задал селектор, `bsl-agent ui url` SHALL завершаться ошибкой (без выбора “по умолчанию”) и SHALL печатать список кандидатов для уточнения.

Система SHALL поддерживать селектор `--roots <path>`, который выбирает инстанс по точному совпадению строки root среди `roots[]`, полученных из `GET /api/mcp/sessions`.

#### Scenario: Несколько инстансов без селектора приводят к ошибке
- **GIVEN** в registry есть два “живых” инстанса HTTP UI
- **WHEN** пользователь запускает `bsl-agent ui url` без селекторов
- **THEN** команда завершается ошибкой и печатает список кандидатов (например, `instance_id/pid/ui_url`)

#### Scenario: Селектор `--roots` выбирает нужный инстанс по точному совпадению
- **GIVEN** в registry есть два “живых” инстанса HTTP UI и они обслуживают разные `roots[]`
- **WHEN** пользователь запускает `bsl-agent ui url --roots <root>`
- **THEN** команда выбирает инстанс, у которого в `/api/mcp/sessions` есть `roots[]`, содержащий строку `<root>` в точном совпадении

