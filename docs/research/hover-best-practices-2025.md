# Исследование: Hover Best Practices для BSL Gradual Types

**Дата:** 2025-11-08
**Автор:** Architect (via Claude Code)
**Статус:** Research Complete
**Целевая Milestone:** 3.X (Future Enhancement)

---

## Содержание

1. [Executive Summary](#executive-summary)
2. [Анализ существующих решений](#анализ-существующих-решений)
3. [Ключевые паттерны и практики](#ключевые-паттерны-и-практики)
4. [Адаптация под BSL](#адаптация-под-bsl)
5. [Архитектурные варианты](#архитектурные-варианты)
6. [Рекомендация](#рекомендация)
7. [План реализации](#план-реализации)
8. [Приложения](#приложения)

---

## Executive Summary

### 🎯 Цель исследования

Изучение лучших практик hover (подсказок при наведении) в современных IDE и Language Servers для улучшения пользовательского опыта в BSL Gradual Type System.

### 📊 Текущее состояние

**Что уже есть:**
- ✅ HoverFormatter с конфигурируемыми лимитами (`max_methods: 10`, `max_properties: 5`)
- ✅ Markdown форматирование
- ✅ Отображение типов, методов, свойств
- ✅ Certainty indicators (🟢 Known, 🟡 Inferred, ⚪ Unknown)
- ✅ Graceful degradation для неизвестных типов

**Что нужно улучшить:**
- ❌ Нет кастомизации через LSP settings
- ❌ Нет интерактивности (ссылки на документацию)
- ❌ Нет expandable sections
- ❌ Ограниченная поддержка accessibility
- ❌ Нет lazy loading для больших типов
- ❌ Нет code actions в hover

### 🏆 Ключевые находки

1. **Rust Analyzer** — золотой стандарт LSP hover
   - Интерактивные ссылки на документацию
   - Expandable sections для сигнатур
   - Настройки в `rust-analyzer.hover.*`

2. **TypeScript Language Server** — фокус на кастомизации
   - `typescript.inlayHints.*` settings
   - Разные уровни детализации

3. **Pylance** — rich formatting
   - Docstrings с Markdown
   - Type hints inline
   - Инкрементальные обновления

4. **JetBrains IDEA** — максимальная информативность
   - Quick documentation panel
   - Expandable sections
   - HTML rendering с CSS

---

## Анализ существующих решений

### 1.1 Rust Analyzer

**Источники:**
- GitHub: https://github.com/rust-lang/rust-analyzer
- Documentation: https://rust-analyzer.github.io/

#### Особенности

**✨ Что делает хорошо:**

1. **Контекстная информация**
   ```markdown
   fn my_function(x: i32) -> String

   ---

   This function converts an integer to a string.

   # Examples

   ```rust
   let result = my_function(42);
   assert_eq!(result, "42");
   ```

   # Panics

   Never panics.
   ```

2. **Интерактивные элементы**
   - Ссылки на типы (кликабельные)
   - "Go to definition" из hover
   - Expandable trait implementations

3. **Настройки (через LSP)**
   ```json
   {
     "rust-analyzer.hover.actions.enable": true,
     "rust-analyzer.hover.actions.implementations.enable": true,
     "rust-analyzer.hover.actions.references.enable": true,
     "rust-analyzer.hover.documentation.enable": true,
     "rust-analyzer.hover.links.enable": true
   }
   ```

4. **Performance**
   - Lazy evaluation для trait impls
   - Кеширование результатов
   - Асинхронная генерация hover content

#### Что можно позаимствовать

✅ **Для BSL:**
- Интерактивные ссылки на типы платформы
- Настройки `bsl.hover.showMethods`, `bsl.hover.showProperties`
- Expandable sections для методов с большим количеством параметров
- Ссылки на platform documentation (syntax_helper)

---

### 1.2 TypeScript Language Server

**Источники:**
- NPM: https://www.npmjs.com/package/typescript-language-server
- Docs: https://emacs-lsp.github.io/lsp-mode/page/lsp-typescript/

#### Особенности

**✨ Что делает хорошо:**

1. **Разные режимы детализации**
   ```typescript
   // Краткий режим
   function add(a: number, b: number): number

   // Полный режим (с JSDoc)
   /**
    * Adds two numbers together.
    * @param a - The first number
    * @param b - The second number
    * @returns The sum of a and b
    */
   function add(a: number, b: number): number
   ```

2. **Настройки**
   ```json
   {
     "typescript.inlayHints.parameterNames.enabled": "all",
     "typescript.inlayHints.parameterTypes.enabled": true,
     "typescript.inlayHints.variableTypes.enabled": true,
     "typescript.inlayHints.functionLikeReturnTypes.enabled": true
   }
   ```

3. **Smart truncation**
   - Длинные сигнатуры сворачиваются
   - "Show more" для дополнительных overloads

#### Что можно позаимствовать

✅ **Для BSL:**
- Разные режимы: `compact` / `full` / `detailed`
- Smart truncation для методов с 10+ параметрами
- Inlay hints для типов переменных (отдельная фича)

---

### 1.3 Python LSP (Pylance)

**Источники:**
- GitHub: https://github.com/microsoft/pylance-release
- Talk Python Podcast: Episode #523

#### Особенности

**✨ Что делает хорошо:**

1. **Rich docstrings**
   ```python
   def calculate_sum(numbers: list[int]) -> int:
       """
       Calculate the sum of a list of numbers.

       Args:
           numbers: A list of integers to sum

       Returns:
           The sum of all numbers

       Example:
           >>> calculate_sum([1, 2, 3])
           6
       """
   ```

   **Hover показывает:**
   ```
   (function) calculate_sum(numbers: list[int]) -> int

   Calculate the sum of a list of numbers.

   Args:
       numbers: A list of integers to sum

   Returns:
       The sum of all numbers

   Example:
       >>> calculate_sum([1, 2, 3])
       6
   ```

2. **Incremental updates**
   - Hover обновляется асинхронно при изменении кода
   - Не блокирует UI

#### Что можно позаимствовать

✅ **Для BSL:**
- Поддержка markdown в описаниях методов/свойств
- Асинхронная генерация hover (уже есть async/await в application фасаде)

---

### 1.4 IntelliJ IDEA (Java)

**Источники:**
- JetBrains Docs: https://www.jetbrains.com/help/idea/
- Plugins: https://plugins.jetbrains.com/plugin/23257-lsp4ij

#### Особенности

**✨ Что делает хорошо:**

1. **Quick Documentation Panel**
   - HTML rendering с CSS
   - Таблицы для перегрузок методов
   - Цветовое выделение по типам

2. **Expandable sections**
   ```
   ▼ Parameters (3)
     • param1: String
     • param2: int
     • param3: Optional<T>

   ▼ Returns
     • List<Result>

   ▼ Throws (2)
     • IOException - если файл не найден
     • IllegalArgumentException - если параметр null
   ```

3. **Deep integration**
   - Code actions прямо в hover
   - "Create test" button
   - "Go to source" link

#### Что НЕ применимо к LSP

❌ **Для BSL:**
- HTML rendering (LSP поддерживает только Markdown/PlainText)
- Кнопки в hover (LSP не поддерживает интерактивные элементы внутри hover)
- **Но:** можно использовать Code Actions рядом с hover

---

### 1.5 LSP Protocol Specification

**Источники:**
- Specification: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
- Microsoft Learn: https://learn.microsoft.com/en-us/dotnet/api/microsoft.visualstudio.languageserver.protocol.hover

#### Официальные возможности LSP hover

```typescript
interface Hover {
  /**
   * The hover's content (can be MarkedString, MarkedString[], or MarkupContent)
   */
  contents: MarkedString | MarkedString[] | MarkupContent;

  /**
   * An optional range inside the text document that is used to
   * visualize the hover, e.g. by changing the background color.
   */
  range?: Range;
}

interface MarkupContent {
  /**
   * 'plaintext' | 'markdown'
   */
  kind: MarkupKind;

  /**
   * The content itself
   */
  value: string;
}
```

#### Ограничения LSP

❌ **Что НЕ поддерживается:**
- HTML (только Markdown/PlainText)
- Кнопки/формы внутри hover
- Expandable sections (нужно решать через Markdown)
- JavaScript/interactivity

✅ **Что поддерживается:**
- Markdown с code fences
- Синтаксическая подсветка в code blocks
- Ссылки (через `[text](uri)`)
- Emoji (🟢 🟡 ⚪)

---

## Ключевые паттерны и практики

### 2.1 Кастомизация

#### Паттерн: User Settings

**Примеры из индустрии:**

```json
// Rust Analyzer
{
  "rust-analyzer.hover.actions.enable": true,
  "rust-analyzer.hover.documentation.enable": true,
  "rust-analyzer.hover.links.enable": true
}

// TypeScript
{
  "typescript.preferences.includeInlayParameterNameHints": "all",
  "typescript.preferences.includeInlayFunctionParameterTypeHints": true
}

// Python (Pylance)
{
  "python.analysis.typeCheckingMode": "basic",
  "python.analysis.inlayHints.variableTypes": true
}
```

**Рекомендация для BSL:**

```json
{
  "bsl.hover.detailLevel": "full",  // "compact" | "full" | "detailed"
  "bsl.hover.showMethods": true,
  "bsl.hover.showProperties": true,
  "bsl.hover.maxMethods": 10,
  "bsl.hover.maxProperties": 5,
  "bsl.hover.showCertainty": true,
  "bsl.hover.showFacets": true,
  "bsl.hover.showDocumentation": true,
  "bsl.hover.enableLinks": true  // Ссылки на platform docs
}
```

---

### 2.2 Стили и форматирование

#### Паттерн: Markdown с Code Fences

**Лучшие практики:**

1. **Используй code fences для сигнатур**
   ```markdown
   ```bsl
   Функция ПолучитьСумму(a: Число, b: Число): Число
   ```
   ```

2. **Структурируй информацию секциями**
   ```markdown
   **Переменная:** МассивДанных
   **Тип:** Массив<Строка>
   **Уверенность:** 🟢 Known (100%)

   ---

   **Методы** (показано 5 из 25):
   • **Добавить(Значение: Произвольный)** → void
   • **Найти(Значение: Произвольный)** → Число
   • **Количество()** → Число
   • **Очистить()** → void
   • **Удалить(Индекс: Число)** → void

   ... и ещё 20 методов
   ```

3. **Используй визуальные индикаторы**
   - 🟢 Known (100%)
   - 🟡 Inferred (85%)
   - ⚪ Unknown (0%)
   - ⚠️ Предупреждения
   - 💡 Подсказки

---

### 2.3 Детализация

#### Паттерн: Progressive Disclosure

**Уровни детализации:**

**Level 1: Compact (минимум)**
```
Переменная: МассивДанных
Тип: Массив<Строка>
```

**Level 2: Full (по умолчанию)**
```
Переменная: МассивДанных
Тип: Массив<Строка>
Уверенность: 🟢 Known (100%)

Методы (показано 5 из 25):
• Добавить(Значение: Произвольный) → void
• Найти(Значение: Произвольный) → Число
• Количество() → Число
... и ещё 22 метода
```

**Level 3: Detailed (максимум)**
```
Переменная: МассивДанных
Тип: Массив<Строка>
Уверенность: 🟢 Known (100%)
Источник: Static Analysis
Фасеты: Object

Методы (показано 10 из 25):
• Добавить(Значение?: Произвольный = Неопределено) → void
  Добавляет элемент в конец массива

• Найти(Значение: Произвольный) → Число
  Ищет элемент в массиве, возвращает индекс

• Количество() → Число
  Возвращает количество элементов

... и ещё 15 методов

Свойства (показано 3 из 8):
• ВГраница: Число (readonly)
• НГраница: Число (readonly)
• Количество: Число (readonly)
... и ещё 5 свойств

📖 Документация: [Массив - 1С Platform](uri://platform-docs/array)
```

---

### 2.4 Интерактивность

#### Паттерн: Links в Markdown

**LSP поддерживает:**

```markdown
**Тип:** [Массив](command:bsl.goToDefinition?type=Массив)

**Методы:**
• [Добавить](command:bsl.goToMethod?type=Массив&method=Добавить)
• [Найти](command:bsl.goToMethod?type=Массив&method=Найти)

**Документация:**
[Массив - Синтакс Помощник](file:///path/to/syntax_helper/array.html)
```

**Ограничения:**
- VS Code не поддерживает `command:` URI в hover по умолчанию
- Можно использовать `file://` и `http://` URI

**Обходное решение:**
- Code Lens для "Go to definition"
- Quick Fix для "Show all methods"

---

### 2.5 Performance

#### Паттерн: Lazy Loading

**Проблема:**
- Платформенные типы с 100+ методами
- Hover генерируется для каждого наведения

**Решение 1: Incremental Rendering**
```rust
// Сначала показываем базовую информацию
let quick_hover = format!(
    "**Переменная:** {}\n**Тип:** {}",
    name, type_name
);

// Затем асинхронно догружаем методы/свойства
tokio::spawn(async move {
    let methods = metadata_lookup.get_methods(&resolution).await;
    // Обновить hover (если ещё актуально)
});
```

**Решение 2: Caching**
```rust
// application фасад уже использует cache layer
let cache_key = format!("hover:{}:{}", file_path, position);
if let Some(cached) = cache.get(&cache_key) {
    return cached;
}
```

**Решение 3: Limits**
```rust
// Уже реализовано в HoverFormatConfig
HoverFormatConfig {
    max_methods: 10,      // Показать только первые 10
    max_properties: 5,    // Показать только первые 5
    ..Default::default()
}
```

---

### 2.6 Accessibility

#### Паттерн: Screen Reader Support

**WCAG 2.0 Guidelines:**

1. **Content on Hover or Focus (1.4.13)**
   - Hover должен оставаться видимым, пока курсор на элементе
   - Должен быть доступен с клавиатуры (не только мышь)
   - Не должен auto-dismiss

2. **Text Alternatives (1.1.1)**
   - Emoji должны иметь текстовый fallback
   - `🟢 Known (100%)` лучше чем просто `🟢`

3. **Color Contrast (1.4.3)**
   - Контраст между текстом и фоном >= 4.5:1
   - VS Code темы автоматически обеспечивают это

**Рекомендации для BSL:**

✅ **Уже соблюдается:**
- Emoji + текст: `🟢 Known (100%)`
- Plaintext fallback через `OutputFormat::PlainText`

❌ **Требуется:**
- ARIA labels для интерактивных элементов (если добавим ссылки)
- Тестирование с NVDA/JAWS screen readers

---

## Адаптация под BSL

### 3.1 Текущее состояние

#### ✅ Что работает хорошо

1. **HoverFormatter архитектура**
   ```rust
   pub struct HoverFormatter {
       config: HoverFormatConfig,
       metadata_lookup: TypeMetadataLookup,
   }
   ```
   - Чистая архитектура (Separation of Concerns)
   - Конфигурируемые лимиты
   - Переиспользование в LSP/Web/CLI

2. **Certainty indicators**
   ```rust
   🟢 Known (100%)
   🟡 Inferred (85%)
   ⚪ Unknown (0%)
   ```
   - Визуально понятно
   - Честность о неопределённости

3. **Graceful degradation**
   ```rust
   if matches!(resolution.certainty, Certainty::Unknown) {
       return HoverBuilder::new(&self.config)
           .add_header("Переменная", name)
           .add_type_info(resolution)
           .add_certainty(&resolution.certainty)
           .add_section("⚠️", "**Тип не распознан системой**")
           .build();
   }
   ```
   - Информативные fallback сообщения

#### ❌ Что нужно улучшить

1. **Нет кастомизации через LSP settings**
   - `max_methods`, `max_properties` захардкожены в коде
   - Нет возможности выбрать уровень детализации

2. **Нет интерактивности**
   - Нет ссылок на platform documentation
   - Нельзя кликнуть на тип чтобы перейти к определению

3. **Ограниченная поддержка фасетов**
   - Фасеты не показываются в hover
   - Нет объяснения, что такое Manager vs Object

4. **Нет обработки Generic типов в hover**
   - `Массив<Строка>` показывается, но без пояснений о параметрах

---

### 3.2 Предложения

#### 3.2.1 Фасеты типов

**Проблема:**
- 1С типы имеют фасеты: Manager, Object, Reference, Selection, List
- Пользователь видит "СправочникСсылка.Номенклатура" и не понимает, что это

**Решение:**

```markdown
**Переменная:** НоменклатураСсылка
**Тип:** СправочникСсылка.Номенклатура
**Фасет:** Reference (ссылка на элемент)

💡 **Доступные фасеты:**
• Manager - менеджер справочника
• Object - объект справочника
• Reference - ссылка (текущий)
• Selection - выборка элементов
```

**Реализация:**

```rust
fn add_facet_info(self, resolution: &TypeResolution) -> Self {
    if let Some(active_facet) = &resolution.active_facet {
        let facet_description = match active_facet {
            FacetKind::Manager => "менеджер объекта",
            FacetKind::Object => "объект с данными",
            FacetKind::Reference => "ссылка на элемент",
            FacetKind::Selection => "выборка элементов",
            FacetKind::List => "список значений",
        };

        self.add_section(
            "Фасет",
            &format!("{:?} ({})", active_facet, facet_description)
        )
    } else {
        self
    }
}
```

---

#### 3.2.2 Методы платформы с большим количеством параметров

**Проблема:**
- Некоторые методы платформы имеют 10+ параметров
- Hover становится слишком длинным и нечитаемым

**Текущий формат:**
```
• ВыполнитьЗапрос(Запрос: Строка, Параметр1: Произвольный, Параметр2: Произвольный, ...) → РезультатЗапроса
```

**Улучшенный формат (для методов с 4+ параметрами):**

```markdown
**ВыполнитьЗапрос** → РезультатЗапроса

Параметры:
  1. Запрос: Строка
  2. Параметр1?: Произвольный = Неопределено
  3. Параметр2?: Произвольный = Неопределено
  4. Параметр3?: Произвольный = Неопределено
  ... и ещё 6 параметров

💡 Подсказка: Наведите на вызов метода для полной сигнатуры
```

**Реализация:**

```rust
const MULTILINE_PARAM_THRESHOLD: usize = 4;

fn format_method(&self, method: &RawMethodData) -> String {
    if method.params.len() >= MULTILINE_PARAM_THRESHOLD {
        // Многострочный формат
        let mut lines = vec![
            format!("**{}** → {}", method.name, method.return_type)
        ];

        if !method.params.is_empty() {
            lines.push("".to_string());
            lines.push("Параметры:".to_string());

            for (i, param) in method.params.iter().take(self.config.max_params).enumerate() {
                let optional = if param.is_optional { "?" } else { "" };
                let default = param.default_value.as_ref()
                    .map(|v| format!(" = {}", v))
                    .unwrap_or_default();

                lines.push(format!(
                    "  {}. {}{}: {}{}",
                    i + 1, param.name, optional, param.param_type, default
                ));
            }

            if method.params.len() > self.config.max_params {
                lines.push(format!(
                    "  ... и ещё {} параметров",
                    method.params.len() - self.config.max_params
                ));
            }
        }

        lines.join("\n")
    } else {
        // Однострочный формат (как сейчас)
        format!("• **{}({})** → {}", method.name, params_str, return_str)
    }
}
```

---

#### 3.2.3 Generic типы

**Проблема:**
- `Массив<Строка>` показывается, но не объясняется что это
- Пользователь не понимает, что `<Строка>` — это тип элементов

**Решение:**

```markdown
**Переменная:** СписокИмен
**Тип:** Массив<Строка>

💡 **Generic тип:**
• Базовый тип: Массив
• Тип элементов: Строка

**Методы:**
• Добавить(Значение: Строка) → void
  (параметр автоматически приводится к Строка)
• Найти(Значение: Строка) → Число
  (поиск только среди Строка элементов)
```

**Реализация:**

```rust
fn add_generic_info(self, resolution: &TypeResolution) -> Self {
    if let ResolutionResult::Generic(generic) = &resolution.result {
        let params_str = generic.type_params
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        self.add_section("💡 Generic тип", &format!(
            "• Базовый тип: {}\n• Параметры: {}",
            generic.base_type, params_str
        ))
    } else {
        self
    }
}
```

---

#### 3.2.4 Интеграция с platform documentation

**Проблема:**
- Syntax Helper имеет документацию в HTML
- Но в hover нет ссылок на неё

**Решение:**

```markdown
**Переменная:** МассивДанных
**Тип:** Массив

📖 **Документация:**
• [Синтакс Помощник: Массив](file:///path/to/syntax_helper/array.html)
• [1С Docs: Array](https://docs.1c.ru/platform/8.3/array)

💡 Для подробной информации откройте Syntax Helper
```

**Реализация:**

```rust
fn add_documentation_links(mut self, resolution: &TypeResolution) -> Self {
    if let Some(type_name) = resolution.get_platform_type_name() {
        let syntax_helper_path = self.config.syntax_helper_path
            .join(format!("{}.html", type_name.to_lowercase()));

        if syntax_helper_path.exists() {
            let uri = format!("file://{}", syntax_helper_path.display());
            self.sections.push(format!(
                "📖 **Документация:** [Синтакс Помощник: {}]({})",
                type_name, uri
            ));
        }
    }

    self
}
```

**Важно:**
- VS Code поддерживает `file://` URI в hover
- Клик откроет HTML в браузере (или встроенном просмотрщике)

---

## Архитектурные варианты

### Вариант 1: Минимальный (2-3 дня)

**Scope:**
- ✅ Добавить LSP configuration settings
- ✅ Три уровня детализации: `compact` / `full` / `detailed`
- ✅ Multiline formatting для методов с 4+ параметрами
- ✅ Документация в README для настроек

**Изменения:**

1. **VSCode Extension settings** (`package.json`)
   ```json
   "configuration": {
     "properties": {
       "bsl.hover.detailLevel": {
         "type": "string",
         "enum": ["compact", "full", "detailed"],
         "default": "full",
         "description": "Уровень детализации hover подсказок"
       },
       "bsl.hover.maxMethods": {
         "type": "number",
         "default": 10,
         "description": "Максимальное количество методов в hover"
       },
       "bsl.hover.maxProperties": {
         "type": "number",
         "default": 5,
         "description": "Максимальное количество свойств в hover"
       }
     }
   }
   ```

2. **LSP Server** (`backend/src/bin/lsp_server.rs`)
   ```rust
   // Передавать settings из LSP client → application фасад
   let config = HoverFormatConfig {
       max_methods: settings.get("bsl.hover.maxMethods").unwrap_or(10),
       max_properties: settings.get("bsl.hover.maxProperties").unwrap_or(5),
       detail_level: settings.get("bsl.hover.detailLevel").unwrap_or("full"),
       ..Default::default()
   };
   ```

3. **HoverFormatter** (`backend/src/helpers/hover_formatter.rs`)
   ```rust
   pub enum DetailLevel {
       Compact,   // Только тип
       Full,      // Тип + методы (до max_methods)
       Detailed,  // Тип + методы + свойства + фасеты + документация
   }

   pub struct HoverFormatConfig {
       pub detail_level: DetailLevel,
       // ... остальное без изменений
   }
   ```

**Плюсы:**
- ✅ Быстрая реализация (2-3 дня)
- ✅ Обратная совместимость (default settings)
- ✅ Значительное улучшение UX

**Минусы:**
- ❌ Нет интерактивности (ссылки)
- ❌ Нет expandable sections (LSP ограничение)

---

### Вариант 2: Стандартный (5-7 дней)

**Scope:**
- ✅ Всё из Варианта 1
- ✅ Поддержка фасетов в hover
- ✅ Generic типы с пояснениями
- ✅ Ссылки на platform documentation
- ✅ Multiline formatting для всех методов с 4+ параметрами

**Дополнительные изменения:**

1. **Фасеты**
   ```rust
   impl HoverBuilder {
       fn add_facet_info(self, resolution: &TypeResolution) -> Self {
           // ... (см. раздел 3.2.1)
       }
   }
   ```

2. **Generic типы**
   ```rust
   impl HoverBuilder {
       fn add_generic_info(self, resolution: &TypeResolution) -> Self {
           // ... (см. раздел 3.2.3)
       }
   }
   ```

3. **Документация**
   ```rust
   pub struct HoverFormatConfig {
       pub syntax_helper_path: Option<PathBuf>,
       // ... остальное
   }

   impl HoverBuilder {
       fn add_documentation_links(self, resolution: &TypeResolution) -> Self {
           // ... (см. раздел 3.2.4)
       }
   }
   ```

**Плюсы:**
- ✅ Значительное улучшение информативности
- ✅ Интеграция с Syntax Helper
- ✅ Поддержка всех типов BSL (фасеты, generic)

**Минусы:**
- ⚠️ Требует настройки `syntax_helper_path`
- ⚠️ Ссылки `file://` работают только локально

---

### Вариант 3: Продвинутый (10-14 дней)

**Scope:**
- ✅ Всё из Варианта 2
- ✅ Code Actions в hover (через LSP Code Lens)
- ✅ Асинхронная генерация hover для больших типов
- ✅ Умное кеширование с invalidation
- ✅ A11y тестирование с screen readers
- ✅ Web documentation server (для remote work)

**Дополнительные фичи:**

1. **Code Lens для hover**
   ```rust
   // Показывать Code Lens над переменной с типом
   let type_info: Массив<Строка> (🟢 Known)
                  ↓
   [Show all methods] [Go to definition] [Documentation]
   ```

2. **Асинхронная генерация**
   ```rust
   async fn get_hover_info(&self, code: &str, line: usize, col: usize) -> Result<String> {
       // Quick response (базовая информация)
       let quick_hover = self.get_quick_hover(code, line, col)?;

       // Если нужна детальная информация - догружаем асинхронно
       if self.config.detail_level == DetailLevel::Detailed {
           tokio::spawn(async move {
               // Получить методы/свойства
               // Обновить hover (если ещё актуально)
           });
       }

       Ok(quick_hover)
   }
   ```

3. **Web Documentation Server**
   ```rust
   // bsl-web-server предоставляет HTTP endpoint для документации
   GET /api/docs/types/Массив

   Response:
   {
     "name": "Массив",
     "description": "Коллекция произвольных элементов",
     "methods": [...],
     "properties": [...]
   }
   ```

   **Hover ссылается:**
   ```markdown
   📖 **Документация:** [Массив](http://localhost:3002/docs/types/Массив)
   ```

**Плюсы:**
- ✅ Максимальная функциональность
- ✅ Работает удалённо (через Web Server)
- ✅ Accessibility compliant

**Минусы:**
- ❌ Долгая реализация (10-14 дней)
- ❌ Требует Web Server running
- ❌ Сложнее в maintenance

---

## Рекомендация

### 🎯 Выбранный вариант: **Вариант 2 (Стандартный)**

**Обоснование:**

1. **Баланс усилий и результата**
   - 5-7 дней работы
   - Значительное улучшение UX
   - Поддержка всех типов BSL (фасеты, generic)

2. **Реализуемость**
   - Не требует архитектурных изменений
   - Использует существующие компоненты
   - Обратная совместимость

3. **Приоритет фич**
   - ✅ Настройки через LSP (must-have)
   - ✅ Фасеты и Generic типы (критично для 1С)
   - ✅ Ссылки на документацию (nice-to-have)
   - ❌ Code Lens (можно отложить)
   - ❌ Web Server (overhead)

### 📈 Миграционный путь

**Phase 1: Milestone 3.1 (Вариант 1)**
- LSP settings
- Три уровня детализации
- Multiline formatting

**Phase 2: Milestone 3.2 (Вариант 2)**
- Фасеты
- Generic типы
- Документация links

**Phase 3: Milestone 3.3 (опционально, Вариант 3)**
- Code Lens
- Асинхронность
- Web Server integration

---

## План реализации

### Milestone 3.1: Hover Settings & Detail Levels (5 дней)

#### Task 1.1: VSCode Extension Settings (1 день)

**Файлы:**
- `vscode-extension/package.json`
- `vscode-extension/src/extension.ts`

**Изменения:**

1. Добавить configuration в `package.json`:
   ```json
   "configuration": {
     "title": "BSL Gradual Types",
     "properties": {
       "bsl.hover.detailLevel": {
         "type": "string",
         "enum": ["compact", "full", "detailed"],
         "default": "full",
         "enumDescriptions": [
           "Только тип переменной",
           "Тип + методы (до max)",
           "Тип + методы + свойства + документация"
         ],
         "description": "Уровень детализации hover подсказок"
       },
       "bsl.hover.maxMethods": {
         "type": "number",
         "default": 10,
         "minimum": 1,
         "maximum": 50,
         "description": "Максимальное количество методов в hover"
       },
       "bsl.hover.maxProperties": {
         "type": "number",
         "default": 5,
         "minimum": 1,
         "maximum": 20,
         "description": "Максимальное количество свойств в hover"
       },
       "bsl.hover.showCertainty": {
         "type": "boolean",
         "default": true,
         "description": "Показывать уверенность в типе (Known/Inferred/Unknown)"
       }
     }
   }
   ```

2. Передавать settings в LSP через `workspace/didChangeConfiguration`:
   ```typescript
   client.onReady().then(() => {
     // Отправить начальные настройки
     client.sendNotification('workspace/didChangeConfiguration', {
       settings: {
         bsl: vscode.workspace.getConfiguration('bsl')
       }
     });

     // Слушать изменения
     vscode.workspace.onDidChangeConfiguration(e => {
       if (e.affectsConfiguration('bsl.hover')) {
         client.sendNotification('workspace/didChangeConfiguration', {
           settings: {
             bsl: vscode.workspace.getConfiguration('bsl')
           }
         });
       }
     });
   });
   ```

**Результат:**
- ✅ Настройки доступны в VS Code Settings UI
- ✅ Изменения передаются в LSP Server

---

#### Task 1.2: LSP Server Configuration Handling (1 день)

**Файлы:**
- `backend/src/bin/lsp_server.rs`

**Изменения:**

1. Принять `workspace/didChangeConfiguration` notification:
   ```rust
   use serde::{Deserialize, Serialize};

   #[derive(Debug, Clone, Deserialize, Serialize)]
   struct BslHoverSettings {
       #[serde(rename = "detailLevel")]
       detail_level: String,  // "compact" | "full" | "detailed"

       #[serde(rename = "maxMethods")]
       max_methods: usize,

       #[serde(rename = "maxProperties")]
       max_properties: usize,

       #[serde(rename = "showCertainty")]
       show_certainty: bool,
   }

   impl Default for BslHoverSettings {
       fn default() -> Self {
           Self {
               detail_level: "full".to_string(),
               max_methods: 10,
               max_properties: 5,
               show_certainty: true,
           }
       }
   }
   ```

2. Обновить application фасад, чтобы принимать settings:
   ```rust
   // В LSP server state
   struct ServerState {
       analysis_host: Arc<AnalysisHostV2>,
       hover_settings: Arc<RwLock<BslHoverSettings>>,
   }

   // Handler для didChangeConfiguration
   async fn on_did_change_configuration(
       params: DidChangeConfigurationParams,
       state: Arc<ServerState>,
   ) {
       if let Some(bsl_settings) = params.settings.get("bsl") {
           if let Some(hover_settings) = bsl_settings.get("hover") {
               let settings: BslHoverSettings = serde_json::from_value(hover_settings).unwrap_or_default();
               *state.hover_settings.write().await = settings;
           }
       }
   }
   ```

**Результат:**
- ✅ LSP Server получает настройки от VS Code
- ✅ Настройки обновляются динамически

---

#### Task 1.3: HoverFormatter с DetailLevel (2 дня)

**Файлы:**
- `backend/src/helpers/hover_formatter.rs`

**Изменения:**

1. Добавить `DetailLevel` enum:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum DetailLevel {
       /// Только тип переменной
       Compact,

       /// Тип + методы (до max_methods)
       Full,

       /// Тип + методы + свойства + фасеты + документация
       Detailed,
   }
   ```

2. Обновить `HoverFormatConfig`:
   ```rust
   pub struct HoverFormatConfig {
       pub max_methods: usize,
       pub max_properties: usize,
       pub detail_level: DetailLevel,
       pub show_certainty: bool,
       pub output_format: OutputFormat,
       pub theme: Theme,
       pub locale: Locale,
   }

   impl Default for HoverFormatConfig {
       fn default() -> Self {
           Self {
               max_methods: 10,
               max_properties: 5,
               detail_level: DetailLevel::Full,
               show_certainty: true,
               output_format: OutputFormat::Markdown,
               theme: Theme::Dark,
               locale: Locale::Ru,
           }
       }
   }
   ```

3. Обновить `format_variable` с учётом `detail_level`:
   ```rust
   pub fn format_variable(&self, name: &str, resolution: &TypeResolution) -> String {
       match self.config.detail_level {
           DetailLevel::Compact => {
               HoverBuilder::new(&self.config)
                   .add_header("Переменная", name)
                   .add_type_info(resolution)
                   .build()
           }

           DetailLevel::Full => {
               HoverBuilder::new(&self.config)
                   .add_header("Переменная", name)
                   .add_type_info(resolution)
                   .add_certainty_if_enabled(&resolution.certainty)
                   .add_methods(resolution, &self.metadata_lookup)
                   .build()
           }

           DetailLevel::Detailed => {
               HoverBuilder::new(&self.config)
                   .add_header("Переменная", name)
                   .add_type_info(resolution)
                   .add_certainty_if_enabled(&resolution.certainty)
                   .add_methods(resolution, &self.metadata_lookup)
                   .add_properties(resolution, &self.metadata_lookup)
                   .build()
           }
       }
   }
   ```

4. Добавить `add_certainty_if_enabled`:
   ```rust
   impl HoverBuilder {
       fn add_certainty_if_enabled(self, certainty: &Certainty) -> Self {
           if self.config.show_certainty {
               self.add_certainty(certainty)
           } else {
               self
           }
       }
   }
   ```

**Результат:**
- ✅ Три уровня детализации работают
- ✅ Настройки применяются динамически

---

#### Task 1.4: Multiline Formatting для методов (1 день)

**Файлы:**
- `backend/src/helpers/hover_formatter.rs`

**Изменения:**

1. Добавить константу для threshold:
   ```rust
   const MULTILINE_PARAM_THRESHOLD: usize = 4;
   ```

2. Обновить `add_methods` с multiline formatting:
   ```rust
   fn add_methods(
       mut self,
       resolution: &TypeResolution,
       metadata_lookup: &TypeMetadataLookup,
   ) -> Self {
       let methods = metadata_lookup.get_methods(resolution);

       if !methods.is_empty() {
           let total_count = methods.len();
           let display_count = self.config.max_methods.min(total_count);

           let mut method_lines = vec![format!(
               "Методы (показано {} из {}):",
               display_count, total_count
           )];

           for method in methods.iter().take(display_count) {
               let formatted = self.format_method(method);
               method_lines.push(formatted);
           }

           if total_count > display_count {
               method_lines.push(format!(
                   "\n... и ещё {} методов",
                   total_count - display_count
               ));
           }

           self.sections.push(method_lines.join("\n"));
       }

       self
   }

   fn format_method(&self, method: &RawMethodData) -> String {
       if method.params.len() >= MULTILINE_PARAM_THRESHOLD {
           // Multiline format
           let mut lines = vec![
               format!("**{}** → {}", method.name, method.return_type)
           ];

           if !method.params.is_empty() {
               lines.push("  Параметры:".to_string());

               for (i, param) in method.params.iter().enumerate() {
                   let optional = if param.is_optional { "?" } else { "" };
                   let default = param.default_value.as_ref()
                       .map(|v| format!(" = {}", v))
                       .unwrap_or_default();

                   lines.push(format!(
                       "    {}. {}{}: {}{}",
                       i + 1, param.name, optional, param.param_type, default
                   ));
               }
           }

           lines.join("\n")
       } else {
           // Single line format (existing)
           let params_str = method.params.iter()
               .map(|p| {
                   let optional_marker = if p.is_optional { "?" } else { "" };
                   let default_suffix = p.default_value.as_ref()
                       .map(|v| format!(" = {}", v))
                       .unwrap_or_default();
                   format!("{}{}: {}{}", p.name, optional_marker, p.param_type, default_suffix)
               })
               .collect::<Vec<_>>()
               .join(", ");

           format!("• **{}({})** → {}", method.name, params_str, method.return_type)
       }
   }
   ```

**Результат:**
- ✅ Методы с 4+ параметрами форматируются multiline
- ✅ Читаемость улучшена

---

### Milestone 3.2: Facets, Generics, Documentation (7 дней)

#### Task 2.1: Фасеты в hover (2 дня)

**Файлы:**
- `backend/src/helpers/hover_formatter.rs`

**Изменения:**

1. Добавить `add_facet_info` в `HoverBuilder`:
   ```rust
   fn add_facet_info(mut self, resolution: &TypeResolution) -> Self {
       if let Some(active_facet) = &resolution.active_facet {
           let facet_description = match active_facet {
               FacetKind::Manager => "менеджер объекта",
               FacetKind::Object => "объект с данными",
               FacetKind::Reference => "ссылка на элемент",
               FacetKind::Selection => "выборка элементов",
               FacetKind::List => "список значений",
           };

           self.sections.push(format!(
               "**Фасет:** {:?} ({})",
               active_facet, facet_description
           ));

           // Показать доступные фасеты
           if !resolution.available_facets.is_empty() {
               let available = resolution.available_facets
                   .iter()
                   .map(|f| format!("{:?}", f))
                   .collect::<Vec<_>>()
                   .join(", ");

               self.sections.push(format!(
                   "💡 **Доступные фасеты:** {}",
                   available
               ));
           }
       }

       self
   }
   ```

2. Добавить в `DetailLevel::Detailed`:
   ```rust
   DetailLevel::Detailed => {
       HoverBuilder::new(&self.config)
           .add_header("Переменная", name)
           .add_type_info(resolution)
           .add_certainty_if_enabled(&resolution.certainty)
           .add_facet_info(resolution)  // ← НОВОЕ
           .add_methods(resolution, &self.metadata_lookup)
           .add_properties(resolution, &self.metadata_lookup)
           .build()
   }
   ```

**Результат:**
- ✅ Фасеты отображаются в hover
- ✅ Пояснения на русском языке

---

#### Task 2.2: Generic типы (2 дня)

**Файлы:**
- `backend/src/helpers/hover_formatter.rs`

**Изменения:**

1. Добавить `add_generic_info`:
   ```rust
   fn add_generic_info(mut self, resolution: &TypeResolution) -> Self {
       if let ResolutionResult::Generic(generic) = &resolution.result {
           let params_str = generic.type_params
               .iter()
               .map(|p| p.to_string())
               .collect::<Vec<_>>()
               .join(", ");

           self.sections.push(format!(
               "💡 **Generic тип:**\n• Базовый тип: {}\n• Параметры: {}",
               generic.base_type, params_str
           ));
       }

       self
   }
   ```

2. Добавить в `DetailLevel::Detailed`:
   ```rust
   .add_generic_info(resolution)  // После add_type_info
   ```

**Результат:**
- ✅ Generic типы объясняются понятно
- ✅ Пример: `Массив<Строка>` показывает базовый тип и параметры

---

#### Task 2.3: Ссылки на документацию (3 дня)

**Файлы:**
- `backend/src/helpers/hover_formatter.rs`
- `backend/src/bin/lsp_server.rs` (для передачи `syntax_helper_path`)

**Изменения:**

1. Добавить `syntax_helper_path` в конфиг:
   ```rust
   pub struct HoverFormatConfig {
       pub syntax_helper_path: Option<PathBuf>,
       // ... остальное
   }
   ```

2. Добавить `add_documentation_links`:
   ```rust
   fn add_documentation_links(mut self, resolution: &TypeResolution) -> Self {
       if let Some(type_name) = self.get_platform_type_name(resolution) {
           let mut links = Vec::new();

           // Syntax Helper (локальный HTML)
           if let Some(base_path) = &self.config.syntax_helper_path {
               let html_path = base_path.join(format!("{}.html", type_name.to_lowercase()));
               if html_path.exists() {
                   let uri = format!("file://{}", html_path.display());
                   links.push(format!(
                       "[Синтакс Помощник: {}]({})",
                       type_name, uri
                   ));
               }
           }

           // 1C Platform Docs (онлайн)
           links.push(format!(
               "[1С Platform Docs](https://docs.1c.ru/search?q={})",
               type_name
           ));

           if !links.is_empty() {
               self.sections.push(format!(
                   "📖 **Документация:**\n{}",
                   links.iter()
                       .map(|l| format!("• {}", l))
                       .collect::<Vec<_>>()
                       .join("\n")
               ));
           }
       }

       self
   }

   fn get_platform_type_name(&self, resolution: &TypeResolution) -> Option<String> {
       match &resolution.result {
           ResolutionResult::Concrete(ConcreteType::Platform(pt)) => {
               Some(pt.name.clone())
           }
           ResolutionResult::Generic(gt) => {
               Some(gt.base_type.clone())
           }
           _ => None,
       }
   }
   ```

3. Передать `syntax_helper_path` из LSP server:
   ```rust
   // В lsp_server.rs при создании v2 host runtime
   let syntax_helper_path = std::env::var("BSL_SYNTAX_HELPER_PATH")
       .ok()
       .map(PathBuf::from);

   let hover_config = HoverFormatConfig {
       syntax_helper_path,
       // ... остальное из settings
   };
   ```

**Результат:**
- ✅ Ссылки на Syntax Helper (если настроен путь)
- ✅ Ссылки на онлайн документацию 1С
- ✅ Клик открывает HTML в браузере

---

### Критерии завершения (Definition of Done)

**Milestone 3.1:**
- [ ] VS Code settings UI работает
- [ ] Три уровня детализации (`compact`, `full`, `detailed`)
- [ ] Multiline formatting для методов с 4+ параметрами
- [ ] Unit тесты для всех новых функций
- [ ] Integration тесты LSP settings → hover
- [ ] Документация в README.md

**Milestone 3.2:**
- [ ] Фасеты отображаются в hover
- [ ] Generic типы объясняются
- [ ] Ссылки на документацию работают
- [ ] Unit тесты для фасетов/generic
- [ ] Документация с примерами

---

### Зависимости

**Milestone 3.1:**
- ✅ Нет блокирующих зависимостей
- ⚠️ Требуется обновить VSCode Extension build (npm)

**Milestone 3.2:**
- ✅ Milestone 3.1 завершён
- ⚠️ Syntax Helper должен быть доступен локально (опционально)

---

### Оценка времени

**Milestone 3.1:** 5 рабочих дней
- Task 1.1: 1 день (VSCode settings)
- Task 1.2: 1 день (LSP server)
- Task 1.3: 2 дня (DetailLevel)
- Task 1.4: 1 день (Multiline)

**Milestone 3.2:** 7 рабочих дней
- Task 2.1: 2 дня (Фасеты)
- Task 2.2: 2 дня (Generic)
- Task 2.3: 3 дня (Документация)

**Итого:** 12 дней (~2.5 недели)

---

## Приложения

### A. Примеры hover из разных LSP

#### A.1 Rust Analyzer (struct)

```markdown
```rust
pub struct String { /* fields omitted */ }
```

A UTF-8–encoded, growable string.

The `String` type is the most common string type that has ownership over the contents of the string. It has a close relationship with its borrowed counterpart, the primitive str.

# Examples

You can create a `String` from a literal string with `String::from`:

```rust
let hello = String::from("Hello, world!");
```

Go to [String documentation](https://doc.rust-lang.org/std/string/struct.String.html)
```

#### A.2 TypeScript (function)

```markdown
```typescript
function add(a: number, b: number): number
```

**Parameters:**
- `a: number` - The first number
- `b: number` - The second number

**Returns:** `number` - The sum of a and b

**Example:**
```typescript
const result = add(1, 2); // 3
```
```

#### A.3 Python (Pylance)

```markdown
```python
(function) calculate_sum(numbers: list[int]) -> int
```

Calculate the sum of a list of numbers.

**Args:**
- `numbers`: A list of integers to sum

**Returns:**
- The sum of all numbers

**Example:**
```python
>>> calculate_sum([1, 2, 3])
6
```
```

---

### B. LSP Configuration Examples

#### B.1 Rust Analyzer settings.json

```json
{
  "rust-analyzer.hover.actions.enable": true,
  "rust-analyzer.hover.actions.implementations.enable": true,
  "rust-analyzer.hover.actions.references.enable": true,
  "rust-analyzer.hover.documentation.enable": true,
  "rust-analyzer.hover.links.enable": true,
  "rust-analyzer.inlayHints.enable": true,
  "rust-analyzer.inlayHints.parameterHints": true,
  "rust-analyzer.inlayHints.typeHints": true
}
```

#### B.2 TypeScript settings.json

```json
{
  "typescript.inlayHints.parameterNames.enabled": "all",
  "typescript.inlayHints.parameterTypes.enabled": true,
  "typescript.inlayHints.variableTypes.enabled": true,
  "typescript.inlayHints.functionLikeReturnTypes.enabled": true,
  "typescript.suggest.includeCompletionsForModuleExports": true
}
```

#### B.3 Предлагаемые BSL settings.json

```json
{
  "bsl.hover.detailLevel": "full",
  "bsl.hover.maxMethods": 10,
  "bsl.hover.maxProperties": 5,
  "bsl.hover.showCertainty": true,
  "bsl.hover.showFacets": true,
  "bsl.hover.enableLinks": true,
  "bsl.syntaxHelperPath": "C:/1C/Platform/8.3/syntax_helper"
}
```

---

### C. Performance Considerations

#### C.1 Caching Strategy

**Текущее состояние:**
- ✅ analysis cache для результатов анализа
- ✅ IR cache для Intermediate Representation
- ❌ Нет кеширования hover content

**Предложение:**

```rust
use lru::LruCache;

struct HoverCache {
    cache: LruCache<String, String>,
}

impl HoverCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity.try_into().unwrap()),
        }
    }

    fn get(&mut self, key: &str) -> Option<&String> {
        self.cache.get(key)
    }

    fn put(&mut self, key: String, value: String) {
        self.cache.put(key, value);
    }

    fn invalidate(&mut self, file_path: &str) {
        // Удалить все ключи для файла
        self.cache.iter()
            .filter(|(k, _)| k.starts_with(file_path))
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>()
            .iter()
            .for_each(|k| { self.cache.pop(k); });
    }
}
```

**Cache key:**
```rust
let cache_key = format!("{}:{}:{}", file_path, line, col);
```

**Invalidation:**
- При изменении файла (`textDocument/didChange`)
- При изменении настроек (`workspace/didChangeConfiguration`)

#### C.2 Lazy Loading

**Проблема:**
- Некоторые платформенные типы имеют 100+ методов
- Генерация hover может занимать 100-200ms

**Решение:**

```rust
async fn get_hover_info_async(
    &self,
    code: &str,
    line: usize,
    col: usize,
) -> Result<String> {
    // Quick response (синхронно, <50ms)
    let quick_info = self.get_quick_hover(code, line, col)?;

    // Если нужна детальная информация
    if self.config.detail_level == DetailLevel::Detailed {
        // Асинхронно догрузить методы/свойства
        let resolution = self.resolve_type_at_position(code, line, col)?;

        tokio::spawn(async move {
            let methods = self.metadata_lookup.get_methods(&resolution).await;
            // LSP не поддерживает обновление hover после отправки
            // Но можно прекешировать для следующего раза
        });
    }

    Ok(quick_info)
}
```

**Важно:**
- LSP не поддерживает streaming hover
- Но можно прекешировать для следующего наведения

---

### D. Accessibility Checklist

#### D.1 WCAG 2.0 Compliance

- [x] **1.4.13 Content on Hover or Focus**
  - Hover не auto-dismiss
  - Доступен с клавиатуры (LSP обеспечивает)

- [x] **1.1.1 Text Alternatives**
  - Emoji + текст: `🟢 Known (100%)`
  - PlainText fallback через `OutputFormat::PlainText`

- [x] **1.4.3 Color Contrast**
  - VS Code темы обеспечивают контраст >= 4.5:1

- [ ] **TODO: Screen Reader Testing**
  - NVDA (Windows)
  - JAWS (Windows)
  - VoiceOver (macOS)

#### D.2 Keyboard Navigation

- [x] Hover доступен через `Ctrl+K Ctrl+I` (VS Code)
- [x] Hover остаётся видимым при фокусе
- [ ] TODO: Проверить навигацию по ссылкам в hover

---

### E. Ссылки

#### Официальные спецификации

1. **LSP Specification**
   - https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
   - Hover: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover

2. **WCAG 2.0**
   - https://www.w3.org/WAI/WCAG20/quickref/
   - Content on Hover: https://www.w3.org/WAI/WCAG21/Understanding/content-on-hover-or-focus.html

#### Примеры реализаций

3. **Rust Analyzer**
   - GitHub: https://github.com/rust-lang/rust-analyzer
   - Hover implementation: https://github.com/rust-lang/rust-analyzer/tree/master/crates/ide/src/hover

4. **TypeScript Language Server**
   - NPM: https://www.npmjs.com/package/typescript-language-server
   - GitHub: https://github.com/typescript-language-server/typescript-language-server

5. **Pylance**
   - GitHub: https://github.com/microsoft/pylance-release
   - Podcast: https://talkpython.fm/episodes/show/523/pyrefly-fast-ide-friendly-typing-for-python

#### Статьи и руководства

6. **VS Code Extension Development**
   - Settings: https://code.visualstudio.com/docs/reference/default-settings
   - Configuration: https://code.visualstudio.com/api/references/contribution-points#contributes.configuration

7. **Markdown в LSP**
   - MDN: https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Howto/Markdown_in_MDN
   - GFM: https://github.github.com/gfm/

---

**Конец документа**

---

**Версия:** 1.0
**Дата последнего обновления:** 2025-11-08
**Автор:** Architect (via Claude Code)
**Статус:** Ready for Review
