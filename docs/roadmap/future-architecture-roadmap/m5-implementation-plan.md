# План реализации M5: Platform Types DB (prebuilt + пополнение)

**Статус:** 🔴 ПЛАН  
**Цель:** хранить платформенные типы по версиям на сервере и уметь пополнять базу, если нужной версии нет.

---

## Область работ

- Серверный storage платформенных типов (по `platformVersion` + `language`).
- `platform.status/ensure/upload`:
  - `status` → `available|missing|building`
  - `ensure` → запустить build (если есть сырьё или prebuilt)
  - `upload` → принять сырьё/пакет (policy/tenant-scoped)
- Политика SaaS v1: curated prebuilt + controlled tenant upload.
  - `upload` доступен только admin/tenant scoped (и/или enterprise),
  - сборка асинхронно, статус `building`,
  - хранение tenant-scoped (без глобального шаринга по умолчанию).

---

## Критерии завершения (DoD)

- Сервер умеет различать версии и возвращать корректные типы/методы.
- Пополнение возможно “с клиента” (при наличии артефактов) и защищено политикой.
- Сборка/хранение учитывает fingerprint и `schemaVersion` (для детерминизма и инвалидации).
