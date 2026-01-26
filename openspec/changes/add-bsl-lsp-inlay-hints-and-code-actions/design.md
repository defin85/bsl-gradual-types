# Design: LSP inlay hints & code actions (no stubs)

## Цель
Дать IDE-grade UX (type hints + code actions), не нарушая принцип “no stubs by default”:
если capability объявлена — фича должна давать осмысленный результат, иначе capability не объявляется.

## Гейтинг и конфигурация
### Уровни настроек
1) **Feature gates** (включение фичи как таковой):
   - `initializationOptions.enableTypeHints`
   - `initializationOptions.enableCodeActions`
   Эти флаги читаются на `initialize` и определяют, будет ли capability объявлена.

2) **Тонкие настройки** (шум/детализация):
   - `bsl.typeHints.*` (например, `minCertainty`, `showVariableTypes`, `showReturnTypes`, `showUnionDetails`)
   - `bsl.codeActions.*` (если понадобится: allow-list kinds, лимиты)
   Эти настройки приходят через `workspace/didChangeConfiguration` (секция `bsl`).

### Почему capability лучше “не объявлять”, чем “возвращать пусто”
- UX: IDE не будет показывать пользователю меню/интерфейс фичи, которая “ничего не делает”.
- Производительность: IDE не будет генерировать лишние запросы.
- Контракт: проще доказывать соответствие spec (“нет заглушек”).

## Inlay hints (MVP)
### Источник данных
Использовать snapshot анализатора (analysis_v2) и существующую инфраструктуру вычисления типов (как для hover).

### Scope
- Запрос `textDocument/inlayHint` получает `range` → вычисляем hints только для элементов внутри диапазона.
- Тип hints: `InlayHintKind::TYPE`, label формата `: <TypeName>`.

### Шум/порог
- `minCertainty`: не показывать hints для неуверенных типов.
- Дополнительные флаги категории hints: переменные/возвраты.

### Нефункциональные требования
- Детерминизм: одинаковый текст → одинаковый порядок hints.
- Лимиты: максимум hints на ответ.
- Отмена: уважать cancellation token (best-effort).

## Code actions (MVP)
### Принцип
MVP должен включать хотя бы одну “реальную” правку (WorkspaceEdit), а не команду-заглушку.

### Выбор MVP
Предпочтительно начать с действий, которые:
- можно вычислить детерминированно по AST/IR,
- не требуют эвристик по тексту diagnostics,
- ограничены одним документом.

Пример кандидата: `refactor.extract` (extract variable) на выделенном выражении в пределах одной процедуры.

### Нефункциональные требования
- Операции должны быть быстрыми и не блокировать сервер.
- Ограничения применимости MUST быть задокументированы (MVP-гранулярность).

