# WASM Rebuild Procedure

## Проблема: Изменения в структурах не применяются после пересборки

### Симптомы
- Браузер показывает ошибки десериализации типа `Error: missing field '<field_name>'`
- WASM загружается с новым хешом, но ошибка остаётся
- Консоль показывает правильную версию Build, но старую структуру данных

### Причина
**Не кэширование браузера!** Проблема в отсутствии атрибутов Serde для обратной совместимости.

## ✅ Правильное решение

### 1. Добавить Serde атрибуты для новых полей

**Всегда используйте `#[serde(default)]` для новых полей:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub id: String,
    pub name: String,

    // ✅ ПРАВИЛЬНО: новое поле с default
    #[serde(default)]
    pub methods: Vec<String>,

    // ✅ ПРАВИЛЬНО: Option автоматически default = None
    #[serde(rename = "methodsCount")]
    pub methods_count: Option<usize>,
}
```

**Никогда не делайте так:**
```rust
// ❌ НЕПРАВИЛЬНО: Serde требует обязательное наличие поля
pub methods: Vec<String>,
```

### 2. Пересобрать WASM

```bash
cd frontend
trunk build --release
```

Trunk автоматически:
- Генерирует новый content hash для файлов
- Обновляет index.html со ссылками на новые файлы
- Копирует всё в `target/site/`

### 3. Перезагрузить страницу в браузере

Новый WASM с другим хешом загрузится автоматически.

## 🔧 Отладка

### Проверить, какая версия загружена

Добавьте версионную константу в `frontend/src/lib.rs`:

```rust
/// Build version to force WASM cache invalidation
pub const BUILD_VERSION: &str = "1.0.2-methods-support";

#[wasm_bindgen(start)]
pub fn main() {
    // Log build version for verification
    web_sys::console::log_1(&format!("🚀 BSL Frontend Build: {}", BUILD_VERSION).into());
    // ...
}
```

В консоли браузера вы увидите:
```
🚀 BSL Frontend Build: 1.0.2-methods-support
```

### Проверить API ответ

```bash
curl "http://localhost:3002/api/types?limit=1" | jq '.types[0] | keys'
```

Должны увидеть все поля, включая `methods`.

### Проверить структуру в исходниках

```bash
grep -n "pub methods" frontend/src/api/types.rs
```

Должно быть:
```
115:    #[serde(default)]
116:    pub methods: Vec<String>,
```

## 🚫 Что НЕ работает

### ❌ Очистка кэша браузера
Не нужна! Trunk использует content-based hashing.

### ❌ Полная очистка `cargo clean`
Тратит время на пересборку всех зависимостей без необходимости.

### ❌ Изменение версии для изменения хеша
Хеш меняется автоматически при изменении кода.

## 📚 Рекомендации

1. **Всегда добавляйте `#[serde(default)]`** для новых полей в существующих структурах
2. **Используйте `Option<T>`** для полей, которые могут отсутствовать
3. **Тестируйте десериализацию** при изменении API контрактов
4. **Проверяйте консоль браузера** для ошибок десериализации

## 🎯 Ключевой принцип

> **Проблема не в кэше - проблема в обратной совместимости Serde!**
>
> Если API возвращает новые поля, которых нет в структуре WASM - это нормально (поля игнорируются).
>
> Если структура WASM ожидает обязательные поля, которых нет в API - ошибка!

---

**Дата создания:** 2025-10-02
**Последнее обновление:** 2025-10-02
