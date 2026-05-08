## ADDED Requirements

### Requirement: Расширение Zed регистрирует язык BSL с Tree-sitter грамматикой
Расширение MUST содержать `extension.toml` с зарегистрированной Tree-sitter грамматикой BSL из репозитория `third_party/tree-sitter-bsl` и метаданными языка в `languages/bsl/config.toml`.

#### Scenario: Zed распознаёт `.bsl` файлы и применяет грамматику
- **WHEN** пользователь открывает файл с расширением `.bsl` в Zed
- **AND** расширение BSL установлено
- **THEN** Zed использует Tree-sitter грамматику BSL для парсинга
- **AND** применяет queries из `highlights.scm`, `brackets.scm`, `indents.scm`, `outline.scm`

### Requirement: Wasm-загрузчик запускает bsl-lsp-server как Language Server
Расширение MUST содержать Wasm-модуль (`extension.wasm`), реализующий трейт `zed::Extension` и метод `language_server_command()`, который материализует extension-owned `bsl-lsp-server` в Zed extension work dir и возвращает команду запуска с аргументами для работы в режиме STDIO.

#### Scenario: Zed запускает LSP при открытии BSL-файла
- **WHEN** пользователь открывает `.bsl` файл в Zed
- **AND** расширение BSL установлено
- **THEN** Zed вызывает `language_server_command()` из Wasm-модуля
- **AND** `bsl-lsp-server` запускается как дочерний процесс
- **AND** LSP-сервер инициализируется и возвращает `initialize` результат

### Requirement: LSP-сервер перестраивает индекс типов по didChange/didSave
Расширение MUST обеспечивать, что `bsl-lsp-server` получает LSP-уведомления `textDocument/didChange` и `textDocument/didSave` и перестраивает индекс типов при изменениях.

#### Scenario: Изменение кода в редакторе вызывает перестройку индекса
- **WHEN** пользователь редактирует `.bsl` файл в Zed
- **THEN** LSP-сервер получает `textDocument/didChange`
- **AND** `analysis-v2` инкрементально перестраивает `TypeIndex`
- **AND** диагностики типов обновляются и публикуются через `textDocument/publishDiagnostics`

#### Scenario: Сохранение файла вызывает перестройку индекса
- **WHEN** пользователь сохраняет `.bsl` файл (didSave)
- **THEN** LSP-сервер перестраивает индекс для этого файла
- **AND** диагностики публикуются в Zed

### Requirement: LSP предоставляет IntelliSense-функции через стандартные LSP-методы
Расширение MUST обеспечивать, что `bsl-lsp-server` отвечает на стандартные LSP-запросы: `textDocument/completion`, `textDocument/hover`, `textDocument/definition`, `textDocument/references`, `textDocument/documentSymbol`, `textDocument/inlayHint`.

#### Scenario: Автодополнение работает в BSL-файлах
- **WHEN** пользователь вводит код в `.bsl` файле
- **AND** LSP-сервер активен
- **THEN** `textDocument/completion` возвращает релевантные варианты с учётом типов

#### Scenario: Hover показывает информацию о типе
- **WHEN** пользователь наводит курсор на идентификатор в `.bsl` файле
- **AND** LSP-сервер активен
- **THEN** `textDocument/hover` возвращает информацию о типе (TypeId, certainty)

#### Scenario: Go to Definition работает для общих модулей
- **WHEN** пользователь выполняет Go to Definition на вызове процедуры общего модуля
- **THEN** `textDocument/definition` возвращает местоположение определения

### Requirement: Расширение бандлит bsl-lsp-server для linux-x86_64
Расширение MUST предоставлять воспроизводимый build-процесс, который материализует скомпилированный `bsl-lsp-server` (release) для платформы linux-x86_64 как generated artifact в директории расширения перед сборкой `extension.wasm`. Wasm-загрузчик MUST embed-ить этот бинарник, при вызове `language_server_command()` записывать его в Zed extension work dir, делать исполняемым и возвращать относительный путь `./bsl-lsp-server`. Checked-in source MAY ignore generated `bsl-lsp-server` и `extension.wasm`, если build/smoke path явно пересобирает их перед установкой.

#### Scenario: LSP-сервер запускается из bundled бинарника
- **WHEN** Zed вызывает `language_server_command()`
- **THEN** Wasm-загрузчик материализует `bsl-lsp-server` внутри Zed extension work dir
- **AND** возвращённый путь указывает на extension-owned `./bsl-lsp-server`
- **AND** бинарник является исполняемым на linux-x86_64

### Requirement: Расширение устанавливается как dev-extension без публикации в registry
Расширение MUST поддерживать локальную установку через `zed: install dev extension`. Публикация в официальный Zed extensions registry является необязательной и не требуется для первичной итерации.

#### Scenario: Dev-установка расширения в Zed
- **WHEN** разработчик выполняет `zed: install dev extension` и выбирает директорию `zed-extension/`
- **THEN** расширение устанавливается и становится активным
- **AND** подсветка синтаксиса BSL работает
- **AND** LSP-сервер запускается при открытии `.bsl` файлов

### Requirement: Tree-sitter queries покрывают базовый синтаксис BSL
Расширение MUST предоставлять корректные Tree-sitter queries для подсветки ключевых слов, строк, комментариев, чисел, операторов и идентификаторов BSL. Queries SHOULD покрывать специфичные для BSL конструкции: директивы препроцессора (`&НаКлиенте`, `&НаСервере`), ключевые слова (`Процедура`, `Функция`, `Если`, `Для`, `Пока`, `Возврат`), операторы присваивания и сравнения.

#### Scenario: Ключевые слова BSL подсвечиваются
- **WHEN** открыт `.bsl` файл, содержащий `Процедура`, `КонецПроцедуры`, `Если`, `Тогда`, `Иначе`
- **THEN** эти ключевые слова подсвечиваются как `@keyword`

#### Scenario: Строки и комментарии подсвечиваются
- **WHEN** открыт `.bsl` файл, содержащий строковые литералы в двойных кавычках и комментарии `//`
- **THEN** строки подсвечиваются как `@string`
- **AND** комментарии подсвечиваются как `@comment`
