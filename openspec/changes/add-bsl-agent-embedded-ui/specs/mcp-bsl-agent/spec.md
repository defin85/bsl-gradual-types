## ADDED Requirements

### Requirement: Embedded SPA для HTTP UI `bsl-agent` (работает без внешней статики)
Система SHALL встраивать (embed) артефакт SPA (собранный `frontend → target/site`) внутрь бинарника `bsl-agent` и SHALL раздавать его через HTTP UI, чтобы UI мог работать без внешних файлов статики.

#### Scenario: UI работает без `BSL_AGENT_HTTP_STATIC_DIR`
- **GIVEN** `bsl-agent` собран со встроенным SPA и запущен с включённым HTTP UI
- **WHEN** клиент открывает `http://localhost:<port>/`
- **THEN** сервер отдаёт `index.html` и остальные ассеты из embedded набора

### Requirement: `BSL_AGENT_HTTP_STATIC_DIR` имеет приоритет над embedded
Если `BSL_AGENT_HTTP_STATIC_DIR` задан и указывает на существующую директорию, `bsl-agent` SHALL раздавать статику с диска из этой директории, даже если embedded статика присутствует.

#### Scenario: Внешняя статика перекрывает embedded
- **GIVEN** `bsl-agent` собран со встроенным SPA
- **AND** `BSL_AGENT_HTTP_STATIC_DIR` указывает на директорию со статикой
- **WHEN** клиент открывает `http://localhost:<port>/`
- **THEN** сервер отдаёт файлы статики с диска из `BSL_AGENT_HTTP_STATIC_DIR`

### Requirement: Build-time ошибка при отсутствии `target/site`
Система SHALL завершать сборку `bsl-agent` с понятной ошибкой, если артефакт SPA для embed отсутствует (например, не существует `target/site/index.html`).

#### Scenario: Сборка без SPA завершается ошибкой
- **GIVEN** `target/site` отсутствует
- **WHEN** выполняется сборка `bsl-agent`
- **THEN** сборка завершается ошибкой с сообщением о необходимости сначала собрать `frontend`

