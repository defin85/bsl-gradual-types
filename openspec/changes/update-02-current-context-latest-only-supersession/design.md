## Контекст

Текущий `contextProvider` в extension:

- debounce-ит cursor moves на `200ms`;
- вызывает `vscode.commands.executeCommand('bsl.getCurrentContext', ...)`;
- не несёт explicit latest-only generation contract;
- не защищён от stale response apply кроме last-finished-wins поведения.

На backend `bsl.getCurrentContext` уже исполняется через bounded background blocking parse, но каждый
запрос всё ещё может independently дойти до expensive parse/context derivation даже если пользователь
уже отправил более новый cursor position.

## Цели

- Сделать latest cursor position единственным user-visible current-context result.
- Не позволять stale responses перетирать более свежий tooltip/status-bar state.
- Ограничить obsolete current-context work через bounded client generations и server supersession/coalescing.

## Не-цели

- Не менять completion/hover/signatureHelp contracts.
- Не переписывать transport на raw custom request с cancellation token в этом шаге.
- Не менять UI layout status bar или observability panel.

## Решения

### 1. Latest-only generation становится явным client contract

Extension должен завести bounded monotonically increasing generation per visible editor session и
передавать эту generation с каждым `bsl.getCurrentContext` request.

Debounce остаётся только admission-optimisation. Источником truth для stale handling становится
generation contract, а не timing.

### 2. Extension применяет только latest generation

Status-bar/current-context surface обновляется только ответом, который соответствует latest known
generation для данного editor session.

Любой older response:

- не применяется к UI;
- не переоткрывает stale tooltip;
- не считается user-visible success.

### 3. Server honors generation hints через supersession или coalescing

Backend не должен независимо протаскивать все obsolete current-context requests до expensive parse.

Если есть более новая generation того же editor session, older request должен:

- либо supersede-иться до parse/context derivation;
- либо коалесцироваться с эквивалентным newer work, если это сохраняет semantics;
- но в любом случае не накапливать unbounded obsolete auxiliary pressure.

### 4. Validation строится на cursor-burst mixed-load profile

Нужен realistic сценарий:

- rapid cursor/selection updates в одном large module;
- параллельный interactive completion;
- bounded number of meaningful current-context jobs;
- latest tooltip/status-bar state соответствует newest generation.

## Alternatives Considered

### Увеличить только debounce

Недостаточно. Timing-based throttle не решает stale response apply и obsolete server work.

### Сразу перейти на отдельный raw LSP request с cancellation token

Возможно позже, но это более широкий transport change. Для текущей проблемы достаточно explicit
latest-only generation contract поверх existing path.

### Ничего не делать, потому что auxiliary runtime уже изолирован

Изоляция снимает starvation с async runtime, но не убирает obsolete work accumulation и stale UI risk.

## Риски и trade-offs

### Риск: ошибки generation bookkeeping дадут stale tooltip

Нужны deterministic tests на newest-generation-wins и no-stale-apply.

### Риск: появится небольшой protocol growth

Дополнительные bounded generation hints стоят дешевле, чем повторный expensive parse для obsolete requests.

### Риск: split editors и несколько views одного файла усложнят identity

Нужно привязывать generation к editor session, а не только к `uri`.

## Migration / Rollout

1. Зафиксировать step-2 latest-only contract в OpenSpec.
2. Пронести generation hints end-to-end через extension и backend.
3. Добавить cursor-burst mixed-load validation и bounded obsolete-work assertions.
