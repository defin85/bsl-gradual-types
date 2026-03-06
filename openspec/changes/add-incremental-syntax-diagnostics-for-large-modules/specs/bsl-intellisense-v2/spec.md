## MODIFIED Requirements

### Requirement: Observability фиксирует diagnostics trigger/profile/supersede причины (MUST)
Канонический observability контракт MUST фиксировать diagnostics pipeline по low-cardinality измерениям:
- `trigger` (`did_change|did_open|did_save|idle`);
- `profile` (`fast|debounced_full|idle_heavy`);
- `reason` (`published|superseded_version|superseded_generation|cancelled` минимум).

Dual-write MUST оставаться детерминированным из канонического event model: drilldown как primary, legacy как projection.

Дополнительно для `syntax_diagnostics` stage:
- канонический observability contract MUST включать low-cardinality измерение `mode`, показывающее parse mode, использованный для текущей ревизии syntax diagnostics;
- допустимые значения `mode` MUST быть ограничены `incremental|reused|full|other`;
- mode-aware latency MUST позволять сравнить syntax diagnostics stage между parse mode без high-cardinality labels;
- legacy fixed-key метрика `intellisense_v2_syntax_diagnostics_query_ms` MUST сохраняться как aggregate compatibility projection и MUST NOT терять backward compatibility.

#### Scenario: Метрики показывают latency syntax diagnostics по parse mode
- **GIVEN** mixed нагрузка, где syntax diagnostics в одних ревизиях использует `incremental` или `reused`, а в других `full`
- **WHEN** запрашивается observability snapshot
- **THEN** канонический observability contract содержит mode-aware latency разрез для `syntax_diagnostics`
- **AND** значения `mode` ограничены `incremental|reused|full|other`
- **AND** aggregate legacy метрика `intellisense_v2_syntax_diagnostics_query_ms` остаётся доступной
