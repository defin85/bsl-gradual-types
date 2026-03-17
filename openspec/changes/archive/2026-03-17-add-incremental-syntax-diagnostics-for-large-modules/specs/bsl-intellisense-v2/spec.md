## MODIFIED Requirements

### Requirement: Observability фиксирует diagnostics trigger/profile/supersede причины (MUST)
Канонический observability контракт MUST фиксировать diagnostics pipeline по low-cardinality измерениям:
- `trigger` (`did_change|did_open|did_save|idle`);
- `profile` (`fast|debounced_full|idle_heavy`);
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
