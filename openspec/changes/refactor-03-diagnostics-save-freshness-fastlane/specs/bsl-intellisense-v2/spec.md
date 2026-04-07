## MODIFIED Requirements

### Requirement: Дорогие проверки запускаются только по didSave и/или idle trigger (MUST)
Система MUST отделять expensive проверки от fast `didChange` пути.

Expensive-проверки MUST запускаться:
- по `textDocument/didSave`, если событие доступно;
- либо по `idle` trigger после отсутствия новых `didChange` в течение конфигурируемого окна.

Эти проверки MUST NOT быть обязательной частью каждого `didChange` запуска.

Если expensive diagnostics запускаются по `didSave`, система MAY делать bounded first-publish
fastlane до final heavy publish, но такой fastlane:
- MUST оставаться same-version truthful для сохранённой revision;
- MUST NOT публиковать older-version diagnostics;
- MUST NOT ждать unbounded `wait_for_file_version` только ради final heavy completeness.

#### Scenario: Heavy-проверки выполняются после паузы или сохранения
- **GIVEN** пользователь печатает без сохранения
- **WHEN** идут последовательные `didChange`
- **THEN** heavy-проверки не выполняются на каждый символ
- **AND** heavy-проверки запускаются только после `didSave` или достижения `idle` окна

#### Scenario: didSave first publish bounded, even if writer apply lags
- **GIVEN** пользователь сохранил документ на версии `V`
- **AND** analysis writer ещё не догнал `V` в applied revision state
- **WHEN** запускается diagnostics path для `didSave`
- **THEN** система делает bounded first publish для версии `V` без seconds-scale ожидания `wait_for_file_version`
- **AND** первый publish использует только same-version truthful artifacts
- **AND** final heavy publish для `V` может завершиться вторым проходом позже

### Requirement: Observability фиксирует diagnostics trigger/profile/supersede причины (MUST)
Канонический observability контракт MUST фиксировать diagnostics pipeline по low-cardinality измерениям:
- `trigger` (`did_change|did_open|did_save|idle`);
- `profile` (`fast|debounced_full|save_fastlane|idle_heavy`);
- `reason` (`published|superseded_version|superseded_generation|cancelled` минимум).

Dual-write MUST оставаться детерминированным из канонического event model: drilldown как primary, legacy как projection.

Дополнительно для `syntax_diagnostics` stage:
- канонический observability contract MUST включать low-cardinality измерение `mode`, показывающее parse mode, использованный для текущей ревизии syntax diagnostics;
- поле `mode` MUST интерпретироваться stage-aware:
  - для `syntax_diagnostics` `mode` означает parse mode;
  - для completion-related stages `mode` сохраняет completion-routing semantics существующего контракта;
- schema validation / typed registry MUST запрещать недопустимые сочетания stage/mode;
- допустимые значения `mode` MUST быть ограничены `incremental|reused|full|other`;
- для diagnostics path без version-bound `ParseSnapshot` (включая `non-LSP` origins, если shared parse snapshot отсутствует) система MUST публиковать `mode=other`;
- `full` MUST использоваться только когда shared parse snapshot / parse-report contract для текущей ревизии явно указывает на full parse path;
- mode-aware latency MUST позволять сравнить syntax diagnostics stage между parse mode без high-cardinality labels;
- legacy fixed-key метрика `intellisense_v2_syntax_diagnostics_query_ms` MUST сохраняться как aggregate compatibility projection и MUST NOT терять backward compatibility.

Для save-triggered first publish observability MUST отдельно позволять доказать:
- latency до первого publish после `didSave`;
- был ли использован `save_fastlane` или только final heavy path;
- не ушла ли задержка в `wait_for_file_version`/apply lag до first publish.

#### Scenario: Метрики показывают latency syntax diagnostics по parse mode
- **GIVEN** mixed нагрузка, где syntax diagnostics в одних ревизиях использует `incremental` или `reused`, а в других `full`
- **WHEN** запрашивается observability snapshot
- **THEN** канонический observability contract содержит mode-aware latency разрез для `syntax_diagnostics`
- **AND** значения `mode` ограничены `incremental|reused|full|other`
- **AND** aggregate legacy метрика `intellisense_v2_syntax_diagnostics_query_ms` остаётся доступной

#### Scenario: Non-LSP path без shared parse snapshot деградирует в `other`
- **GIVEN** diagnostics выполняется через origin/path, где для текущей ревизии нет version-bound `ParseSnapshot`
- **WHEN** публикуется observability snapshot для `syntax_diagnostics`
- **THEN** канонический observability contract использует `mode=other`
- **AND** система MUST NOT синтезировать `incremental`, `reused` или `full` из adapter-local предположений

#### Scenario: Save fastlane distinguishable from heavy follow-up
- **GIVEN** first diagnostics refresh после `didSave` выполняется через bounded fastlane
- **WHEN** анализируется observability snapshot или checked-in acceptance report
- **THEN** first publish помечается отдельным profile `save_fastlane`
- **AND** heavy follow-up остаётся различимым как `idle_heavy`
- **AND** evidence позволяет отличить fastlane успех от apply-lag wait regression
