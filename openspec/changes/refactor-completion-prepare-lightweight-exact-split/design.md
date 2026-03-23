## Контекст
Current-revision fast lane уже сделал `CompletionHeadArtifact` first-class current-revision artifact, но completion default path остается структурно перегруженным:
- adapters по-прежнему входят через generic `prepare_stateful_operation`;
- canonical sequencing для него остается `wait_for_version -> snapshot_with_deps -> deps guard`;
- для `head_hit` member-access completion это слишком тяжелая граница, потому что first response логически уже отделен от exact truth;
- попытка держать долгоживущий shared `AnalysisV2` snapshot как published read boundary противоречит текущей ownership-модели runtime и не должна быть базой решения.

Следовательно, проблема уже не в отсутствии head artifact как такового, а в том, что request path не имеет собственного lightweight prepare boundary.

## Цели
- Выделить отдельный lightweight current-revision prepare path для first completion response.
- Сохранить exact stateful prepare как canonical heavy boundary для full semantic truth.
- Не допустить утечки writer-owned runtime state в новый boundary.
- Сделать lightweight/exact route различимым через observability и representative gate.

## Не-цели
- Не публиковать detached immutable head snapshot в рамках этого change.
- Не переводить все semantic операции на новый lightweight path.
- Не заменять exact truth на head truth для non-completion запросов.
- Не вводить stale, degraded или keyword fallback.

## Решение

### 1. Completion получает отдельный prepare contract
Completion first-response path MUST иметь отдельный контракт, логически отличный от generic `prepare_stateful_operation`.

Этот контракт должен уметь вернуть три класса состояний:
- `head-ready`: current revision уже имеет достаточно truth для first completion response;
- `exact-ready`: exact path уже готов и может быть использован напрямую;
- `not-ready`: neither head nor exact current-revision path не готовы в пределах bounded policy.

Названия DTO и enum могут отличаться, но граница должна быть именно такой: feature-specific и request-scoped.

### 2. Lightweight path не должен нести shared runtime snapshot
Lightweight boundary может содержать только узкие immutable данные, необходимые для first response:
- `file_id`, `file_version`;
- bounded consistency ids (`deps_id`, `settings_id`) или их эквивалент;
- доступ к current-revision head truth;
- feature-specific owner hints/candidate skeletons или другой минимальный payload.

Lightweight boundary MUST NOT:
- публиковать `AnalysisV2` как долгоживущий shared snapshot;
- давать наружу writer-owned mutable host state;
- становиться скрытым second exact path.

### 3. Exact path остается существующим heavy contract
`prepare_stateful_operation` и `PreparedOperationSnapshot` остаются canonical heavy boundary:
- для `hover`, `definition`, `signatureHelp`, `type-at-position`;
- для completion exact route и head-to-exact upgrade;
- для проверок, которым нужна полная semantic truth.

Это позволяет сделать change узким: меняется only completion first-response boundary, а не вся интерактивная семантика.

### 4. Default completion route должен стать head-first
Для member-access completion default path должен быть:
1. bounded lightweight current-revision prepare;
2. если `head-ready`, вернуть first response без mandatory heavy exact prepare;
3. если `exact-ready`, использовать exact route;
4. если neither ready, завершить bounded fail-closed без stale substitute.

Иначе head artifact формально существует, но runtime path остается effectively exact-first.

### 5. Acceptance должен доказывать route, а не только outcome
Representative gate должен проверять не только `ok_non_empty`, но и то, что:
- measured samples имеют явное route attribution (`head_hit` / `exact_hit`);
- current-revision first response не зависит от mandatory heavy generic prepare в тех samples, где `head_hit` уже возможен;
- exact upgrade остается отдельной стадией и не маскирует first-response path.

Synthetic tests нужны для:
- `head-ready` path под background saturation;
- fail-closed path при отсутствии current-revision head/exact readiness;
- сохранения exact-only semantics для `hover`, `definition`, `signatureHelp`, `type-at-position`.

## Рассмотренные альтернативы

### Оставить generic prepare и пытаться ускорить только exact path
Отклонено. Это не исправляет неверную boundary shape: first response остается заложником heavy exact prepare.

### Кэшировать shared `AnalysisV2` как published read model
Отклонено. Текущий `AnalysisV2` не является detached immutable snapshot и не должен становиться public feature boundary.

### Сразу идти в detached immutable head snapshot
Отклонено как ближайший шаг. Это более дорогая следующая архитектурная эволюция и не является prereq для split-prepare.

## Риски и trade-offs

### Риск: completion logic станет branch-heavy
Смягчение:
- split должен происходить на boundary level, а не как россыпь ad-hoc `if` в LSP adapter;
- generic prepare и completion prepare должны оставаться отдельными API.

### Риск: exact path и lightweight path разойдутся по semantic contract
Смягчение:
- lightweight truth должен быть строго ограничен first-response задачами;
- exact truth остается источником полной semantic истины;
- representative gate и parity-style tests должны фиксировать допустимую разницу между first response и exact upgrade.

### Риск: change снова попытаются реализовать через shared runtime snapshot
Смягчение:
- change явно запрещает long-lived shared `AnalysisV2` как новую boundary abstraction.

## Открытые разрывы после review
На момент последнего implementation review change еще не готов к архивированию и требует доработки по двум обязательным направлениям.

### 1. Lightweight boundary остается шире, чем разрешает change
Текущая реализация уже request-scoped и не публикует long-lived shared cache, но публичный lightweight contract все еще выдает широкий `AnalysisV2` carrier вместо узкого feature-specific DTO/read-model payload.

До закрытия change нужно:
- сузить публичный lightweight boundary до минимального payload для first response;
- не использовать `AnalysisV2` как внешний carrier для completion first-response API;
- сохранить `PreparedOperationSnapshot` единственным heavy exact boundary для общего semantic path.

### 2. Shipped gate path покрывает только churn real-module profile
Representative evidence уже существует для `revision-churn`, но обязательный `same-revision warm` real-module profile пока не wired в blocking default gate path.

До закрытия change нужно:
- запускать отдельный `same-revision warm` real-module gate рядом с `revision-churn`;
- держать оба профиля в `scripts/validate-v2-completion-gates.sh` и в CI;
- считать change незавершенным, пока shipped gate не проверяет оба обязательных live profiles.

### 3. Review outcome должен быть зафиксирован как checked-in evidence
Сам факт review нельзя считать доказанным только по устной/чатовой истории. Для closure change нужен checked-in artifact или эквивалентный traceable record, который фиксирует вывод review и подтверждает отсутствие detached immutable snapshot как prereq.

## Acceptance-направление
- Member-access completion на current revision имеет отдельный lightweight prepare boundary.
- `head_hit` по default path больше не требует mandatory full `snapshot_with_deps`.
- `hover`, `definition`, `signatureHelp`, `type-at-position` остаются exact-only через существующий heavy prepare.
- Representative gate различает lightweight first response и exact upgrade отдельными route signals.
