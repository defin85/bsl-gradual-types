## ADDED Requirements

### Requirement: Панель Symbol Browser доступна в доке Zed
Панель MUST регистрироваться при старте Zed через `add_panel_when_ready`, иметь иконку в статус-баре и поддерживать действия `Toggle`/`ToggleFocus`.

#### Scenario: Пользователь открывает панель
- **WHEN** пользователь выполняет `symbol browser: toggle`
- **THEN** панель появляется в правом доке с заголовком "Symbol Browser"

#### Scenario: Пользователь скрывает панель
- **WHEN** пользователь повторно выполняет `symbol browser: toggle`
- **THEN** панель скрывается

### Requirement: Панель запрашивает символы через LSP workspace/symbol
Панель MUST при инициализации отправлять запрос `workspace/symbol` с пустым query через `project.symbols("", cx)` и отображать полученные символы.

#### Scenario: Загрузка символов для открытого проекта
- **WHEN** панель открыта и активен языковой сервер с поддержкой `workspace/symbol`
- **THEN** панель показывает список символов, сгруппированных по `SymbolKind`

#### Scenario: Проект без workspace/symbol поддержки
- **WHEN** языковой сервер не поддерживает `workspace/symbol`
- **THEN** панель показывает сообщение "No symbols available"

#### Scenario: Ошибка workspace/symbol
- **WHEN** запрос `workspace/symbol` завершается ошибкой
- **THEN** панель показывает сообщение "Symbols unavailable"
- **AND** причина ошибки логируется

### Requirement: Символы группируются по SymbolKind
Панель MUST группировать символы по категориям (Functions, Classes, Structs, Interfaces, Enums, Constants, Variables, Modules, Namespaces, Properties, Constructors, Type Parameters, Other).

#### Scenario: Отображение сгруппированных символов
- **WHEN** получен список символов разных видов
- **THEN** символы отображаются в секциях с заголовками вида "Functions (42)", "Classes (15)"

### Requirement: Настройки панели доступны через settings.json
Панель MUST поддерживать настройки `button` (показ иконки), `default_width` (ширина), `dock` (сторона). Настройки доступны через `"symbol_browser"` ключ в settings.json.

#### Scenario: Изменение ширины панели
- **WHEN** пользователь устанавливает `"symbol_browser": {"default_width": 400}` в settings.json
- **THEN** панель открывается шириной 400px
