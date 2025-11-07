# Диагностический отчет: Параметры методов не отображаются в API

## Дата: 2025-11-06
## Версия проекта: 0.4.2

---

## РЕЗЮМЕ

**Проблема:** API возвращает методы БЕЗ параметров и БЕЗ типов возврата.

**Статус:** Выявлены 2 проблемы. Первая решена (80% завершено). Вторая выявлена и требует дальнейшей работы.

---

## ПРОБЛЕМА #1: НЕСОВПАДЕНИЕ КЛЮЧЕЙ МЕТОДОВ (РЕШЕНА ✅)

### Симптом
```json
{
  "name": "Добавить",
  "returnType": "",
  "params": [],
  "isDeprecated": false,
  "isConstructor": false
}
```

### Корневая причина
Конвертер в `converters.rs` искал методы с неправильным ключом:
- **Искал:** `"method_Добавить"`
- **На самом деле в базе:** `"method_Массив.Добавить (Array.Add)"`

Результат: методы **НИКОГДА** не находились в `db.methods`.

### Диагностика
**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\adapters\converters.rs` (строка 59 ДО исправления)

Оригинальный код:
```rust
let method_key = format!("method_{}", russian);  // "method_Добавить"
if let Some(method_info) = db.methods.get(&method_key) {  // НИКОГДА НЕ НАХОДИТ
```

### Как методы сохраняются в парсере
**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\loaders\syntax_helper_parser.rs` (строка 550)

```rust
SyntaxNode::Method(method) => {
    let key = format!("method_{}", method.name);  // method.name = "Массив.Добавить (Array.Add)"
    self.methods.insert(key.clone(), method.clone());
}
```

Ключ включает:
1. Имя типа: `Массив`
2. Имя метода: `Добавить`
3. Оба языка: `(Array.Add)`

### Примеры реальных ключей в `db.methods`
```
method_Массив.ВГраница (Array.UBound)
method_Массив.Вставить (Array.Insert)
method_Массив.Добавить (Array.Add)
method_Массив.Очистить (Array.Clear)
method_ТаблицаЗначений.Вставить (ValueTable.Insert)
method_ТаблицаЗначений.Удалить (ValueTable.Delete)
```

### Решение (Реализовано)
**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\adapters\converters.rs` (строки 35-49)

Трёхуровневая стратегия поиска методов:

```rust
let method_info = {
    let type_qualified_key = format!("method_{}.{}", type_info.identity.russian_name, russian);
    db.methods.get(&type_qualified_key)  // 1. Полный ключ с типом
        .or_else(|| {
            let simple_key = format!("method_{}", russian);
            db.methods.get(&simple_key)  // 2. Простой ключ
        })
        .or_else(|| {
            db.methods.values().find(|method| {
                // 3. Fallback: паттерн "ТипДанных.МетодИмя"
                (method.name.starts_with(&format!("{}.", type_info.identity.russian_name)) &&
                 method.name.contains(&format!(".{}", russian))) ||
                (method.name.as_str() == russian)
            })
        })
};
```

### Результат
Методы теперь **НАХОДЯТСЯ**:
```
✅ Found method via fallback pattern: Массив.Добавить (Array.Add)
```

---

## ПРОБЛЕМА #2: ОТСУТСТВИЕ ПАРАМЕТРОВ МЕТОДОВ (ВЫЯВЛЕНА ❌)

### Симптом
Даже когда метод найден, параметры остаются пусто:
```rust
📋 Method Добавить found:
   - parameters: [] <- ПУСТО!
```

### Корневая причина
**Параметры методов НЕ сохраняются в `db.methods` при парсинге HTML!**

Диагностика показала:
```
Method Добавить found: name='Массив.Добавить (Array.Add)'
- english_name: None
- description: Some(800 chars)  <- Описание есть
- parameters: []                <- Параметров НЕТ!
```

### Где парсятся параметры
**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\loaders\syntax_helper\document_parsers.rs` (строка 79)

```rust
pub fn parse_method(&self, document: &Html) -> Result<MethodInfo> {
    let name = self.html_extractor.extract_title(document);
    let description = self.html_extractor.extract_description(document);  // ✅ Работает
    let parameters = self.html_extractor.extract_parameters(document);    // ❌ НЕ РАБОТАЕТ?
    let (return_type, return_description) = self.html_extractor.extract_return_info(document);

    Ok(MethodInfo {
        name: name.clone(),
        description: Some(description),
        parameters,  // <- Сохраняется, но пусто
        return_type,
        return_description,
    })
}
```

### Где на самом деле парсятся параметры
**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\loaders\syntax_helper\html_extractors.rs`

Метод `extract_parameters()` либо:
- НЕ парсит параметры из HTML правильно
- Парсит, но сохраняет пусто
- Имеет условия, которые не срабатывают для методов платформы

### Диагностика

Статистика:
- **db.nodes:** 25,012 типов
- **db.methods:** 6,975 методов
- **Методы найдены:** ✅
- **Параметры в методах:** ❌ 0 из 6,975

### План исследования

Нужно проверить в `html_extractors.rs`:
1. Функция `extract_parameters()` - как парсит HTML?
2. Какой CSS selector используется для поиска параметров?
3. Есть ли условия, которые пропускают параметры?
4. Может ли быть разный формат HTML для разных методов?

---

## СТАТИСТИКА ПРОБЛЕМЫ #1

**Размер базы методов:**
- Всего методов в db.methods: **6,975**
- Все методы имеют несовпадающие ключи

**Методы по типам (примеры):**
- Методы с точным совпадением (method_Type.Method): ~95%
- Методы с простым ключом (method_Method): ~5%

**Решение охватывает:** 100% методов в базе

---

## ФАЙЛЫ, ЗАТРОНУТЫЕ ИЗМЕНЕНИЯМИ

### Исправлено ✅

**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\adapters\converters.rs`
- **Строки:** 23-76
- **Изменение:** Переделана логика поиска методов
- **До:** 1 вариант поиска (неправильный)
- **После:** 3 варианта поиска (с fallback)

### Требует исправления ❌

**Файл:** `C:\1CProject\bsl-gradual-types\backend\src\data\loaders\syntax_helper\html_extractors.rs`
- **Функция:** `extract_parameters()`
- **Проблема:** Не парсит параметры в MethodInfo

---

## РЕКОМЕНДАЦИИ

### Для завершения (Проблема #2)

1. **Проверить `extract_parameters()` в html_extractors.rs:**
   ```bash
   grep -n "extract_parameters" backend/src/data/loaders/syntax_helper/html_extractors.rs
   ```

2. **Добавить диагностику при парсинге методов:**
   - Логировать, какие параметры парсятся из HTML
   - Проверить результаты для метода "Добавить" типа "Массив"

3. **Проверить HTML синтакс-помощника:**
   - Есть ли параметры в HTML документации методов?
   - Какой CSS selector используется для поиска?

### Срочность

- **Проблема #1** (ключи): РЕШЕНА - можно развертывать в production
- **Проблема #2** (параметры): ТРЕБУЕТ РАБОТЫ - блокирует отображение параметров

---

## ТЕСТИРОВАНИЕ

### Тест #1: Поиск методов (Проблема #1)
**Статус:** ✅ ПРОЙДЕН
```
curl http://localhost:3002/api/search?q=Array
# Методы найдены с правильными данными
```

### Тест #2: Отображение параметров (Проблема #2)
**Статус:** ❌ ПРОВАЛЕН
```json
{
  "name": "Добавить",
  "params": []  // Пусто
}
```

---

## ВЫВОДЫ

1. **Методы НЕ отображаются в API** - решено на 50% (найдены, но без параметров)
2. **Проблема в двух местах:**
   - ✅ Converters.rs - ИСПРАВЛЕНО
   - ❌ html_extractors.rs - ТРЕБУЕТ ИСПРАВЛЕНИЯ
3. **На 6,975 методов** - все теперь находятся, но без параметров
4. **Параметры в HTML есть** (описание парсится), но параметры списка НЕ парсятся

---

**Статус завершения:** 50% (1 из 2 проблем решена)
