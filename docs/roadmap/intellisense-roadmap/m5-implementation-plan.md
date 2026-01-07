# План реализации M5: Snippets и Signature Help

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** поддержать LSP snippets и полноценный SignatureHelp с корректным активным параметром.

---

## Область работ

- Snippet‑вставки для методов/функций (placeholders, optional params)
- `completionItem/resolve` для тяжелых деталей (documentation/detail/snippet)
- `textDocument/signatureHelp` с корректным activeParameter/activeSignature
- Учет client capabilities (snippetSupport и resolveSupport)

---

## Пошаговый план

### Шаг 1: Контракт и capabilities ✅
- Уточнить контракт Snippet/SignatureHelp на уровне API и LSP.
- Учитывать `completionItem.snippetSupport` в client capabilities.
- Подготовить формат insertText/snippet и правила экранирования.

**Выход:** зафиксированный контракт и условия включения snippets.

---

### Шаг 2: Генерация snippet для методов ✅
- Построить snippet из `MethodSignature` (плейсхолдеры, `$0`).
- Optional параметры включать плейсхолдерами с пустым значением и располагать после обязательных.
- Экранирование `${`, `}` и спец‑символов.

**Выход:** builder snippets в domain/service слое.

---

### Шаг 3: Интеграция snippets в completion pipeline ✅
- Для методов/функций с параметрами подставлять insertText/snippet.
- Делать snippets только при `snippetSupport=true`, иначе plain text.
- Возвращать snippets через `completionItem/resolve` (минимальный payload в completion).

**Выход:** корректное отображение snippets в LSP‑клиенте.

---

### Шаг 4: SignatureHelp pipeline ✅
- Контекст вызова: скобки, строки, вложенные вызовы, многострочные вызовы.
- Определение activeParameter по запятым и глубине скобок.
- Уточнение receiver type через TypeSystem (методы объектов).

**Выход:** стабильный SignatureHelp с корректным активным параметром.

---

### Шаг 5: Тесты и регрессии ✅
- Unit: генерация snippets и экранирование.
- Integration: SignatureHelp по реальным вызовам (включая вложенные).
- LSP: проверка resolve и корректных insertTextFormat.

**Выход:** набор тестов M5, фиксирующий поведение.

---

## Критерии завершения

- Snippets корректно вставляются для методов/функций и учитывают optional params.
- `completionItem/resolve` возвращает тяжелые данные без нагрузки на hot path.
- SignatureHelp стабильно работает на типовых сценариях и корректно ведет activeParameter.
- Тесты покрывают ключевые сценарии snippets + signatureHelp.

---

## Фактический статус (по коду)

- LSP SignatureHelp реализован: `backend/src/bin/lsp_server/handlers/signature_help.rs`.
- SignatureHelp включен в capabilities: `backend/src/bin/lsp_server/server/language_server.rs`.
- Есть интеграционные тесты SignatureHelp: `backend/tests/lsp_signature_help_test.rs`.
- `completionItem/resolve` добавляет detail/documentation: `backend/src/bin/lsp_server/handlers/completion.rs`.
- Snippet‑формат добавляется в resolve и учитывает `completionItem.snippetSupport`.
- Генерация snippets для методов/функций реализована (optional параметры как пустые placeholders).
- SignatureHelp игнорирует комментарии/экранированные кавычки и пытается определить receiver type через resolver.

---

## Чек-лист задач для завершения M5

- Добавить проверку `snippetSupport` и отключать snippets при отсутствии поддержки.
- Реализовать генератор snippets для методов/функций (с optional params).
- Перенести snippet‑вставку в resolve (минимальный payload в completion).
- Улучшить SignatureHelp: receiver type, вложенные вызовы, строки/комментарии.
- Добавить unit/integration тесты для snippets и resolve.

---

## Задачи (тикеты) по M5

### T1: Контракт snippets и capabilities ✅
**Цель:** определить правила включения snippets и формата вставки.  
**Где:** `backend/src/bin/lsp_server/server/language_server.rs`, `backend/src/bin/lsp_server/handlers/completion.rs`.  
**DoD:**
- учитывается `completionItem.snippetSupport`;
- задокументирован формат placeholders и экранирование;
- есть тест/проверка fallback на plain text.

### T2: Snippet builder для методов ✅
**Цель:** формировать insertText по `MethodSignature`.  
**Где:** domain/service слой (например, `shared/src/domain/signature_index.rs` или новый builder).  
**DoD:**
- placeholders с индексами и `$0`;
- optional параметры идут после обязательных и имеют пустые placeholders;
- экранирование спец‑символов покрыто unit‑тестами.

### T3: Интеграция snippets в completion/resolve ✅
**Цель:** минимальные completion items + тяжелые данные через resolve.  
**Где:** `backend/src/application/type_system/...`, `backend/src/bin/lsp_server/handlers/completion.rs`.  
**DoD:**
- snippets выдаются только при поддержке клиента;
- resolve добавляет insertText/insertTextFormat и docs;
- без увеличения latency в hot path.

### T4: Улучшение SignatureHelp для контекста вызова ✅
**Цель:** точный activeParameter в реальных сценариях.  
**Где:** `backend/src/bin/lsp_server/handlers/signature_help.rs`, `shared/src/domain/signature_index.rs`.  
**DoD:**
- вложенные вызовы и строки не ломают подсчет;
- receiver type учитывается (методы объектов);
- интеграционные тесты на сложные кейсы.

### T5: Тесты M5 ✅
**Цель:** закрепить поведение snippets + signatureHelp.  
**Где:** `backend/tests/...`, `shared/tests/...`.  
**DoD:**
- unit‑тесты snippet builder;
- integration‑тесты signatureHelp и completion resolve;
- регрессионные тесты на экранирование.
