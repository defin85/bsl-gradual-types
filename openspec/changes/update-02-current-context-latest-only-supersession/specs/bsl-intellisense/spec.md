## ADDED Requirements

### Requirement: VS Code current-context surface keeps `bsl.getCurrentContext` latest-only per editor session (MUST)

VS Code extension MUST вести `bsl.getCurrentContext` в latest-only режиме для каждого visible editor
session.

Если extension использует `bsl.getCurrentContext` для status-bar или другой cursor-driven current-context
surface, она MUST вести bounded monotonically increasing request generation для каждого visible editor
session и MUST применять к UI только ответ, соответствующий latest known generation этого session.

Этот contract MUST включать:

- bounded generation hints, отправляемые вместе с `bsl.getCurrentContext` request;
- stale-response drop на client apply path;
- newest-generation-wins semantics для status-bar/current-context surface.

Debounce MAY использоваться как admission optimization, но MUST NOT быть единственным механизмом stale
control.

#### Scenario: Быстрые перемещения курсора не дают stale current-context tooltip

- **GIVEN** пользователь быстро перемещает курсор несколько раз в одном editor session
- **AND** extension отправляет несколько `bsl.getCurrentContext` requests для разных cursor positions
- **WHEN** older response приходит после newer generation
- **THEN** extension не применяет older response к current-context UI
- **AND** status-bar/tooltip остаётся согласован с newest known generation
