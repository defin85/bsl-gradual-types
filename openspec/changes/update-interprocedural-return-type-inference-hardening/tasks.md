## 1. Контракты данных и совместимость
- [ ] 1.1 Зафиксировать контракт `ReturnDomain/ReturnSummary` и унифицировать семантику “known union + weak/dynamic” между persistent и overlay.
- [ ] 1.2 Расширить `MethodSignature` (или эквивалент) меткой weak/dynamic для inferred return‑типов (serde‑совместимо через `default`).
- [ ] 1.3 Обновить merge‑логику `SignatureIndex` для новой метки (монотонное объединение: OR).
- [ ] 1.4 При необходимости bump версии/фингерпринта кэшей конфиг‑парсинга/индексации (чтобы старые записи не смешивались).

## 2. Persistent‑контур: доведение weak/dynamic до call-site типизации
- [ ] 2.1 Изменить результат межмодульного return inference так, чтобы он возвращал не только строковый union, но и флаг weak/dynamic.
- [ ] 2.2 При построении `SignatureIndex` записывать `return_type` (строка union) + метку weak/dynamic в сигнатуру экспортного метода/функции.
- [ ] 2.3 В v2 inference при резолве `SignatureIndex.find_method/find_global_function` понижать `Certainty` до `InferredWeak`, если сигнатура помечена weak/dynamic (без потери union).

## 3. Overlay для open files: покрытие индексируемых module types
- [ ] 3.1 Расширить `module_owner_key_from_file_path` на `ObjectModule` и `RecordSetModule`.
- [ ] 3.2 Канонизировать ключи owner_type:
  - ObjectModule: `СправочникОбъект.<Имя>` / `ДокументОбъект.<Имя>` / …
  - RecordSetModule: `РегистрНакопленияНаборЗаписей.<Имя>` / `РегистрСведенийНаборЗаписей.<Имя>` / …
- [ ] 3.3 Обеспечить, что изменения в B (open file) обновляют типы вызовов в A (open file), когда B — object/recordset module.

## 4. Overlay фикс‑пойнт: “open files ∪ SignatureIndex”
- [ ] 4.1 При вычислении return‑domains в overlay использовать `SignatureIndex` как внешний источник return‑summary для вызовов в неоткрытые модули (без I/O).
- [ ] 4.2 Пробросить weak/dynamic из `SignatureIndex` в overlay‑домены.

## 5. AST fallback: не выключать interprocedural inference
- [ ] 5.1 В AST fallback формировать `return_facts` хотя бы консервативно (Unknown + has_dynamic), чтобы межпроцедурный вывод работал деградированно, но не “в ноль”.
- [ ] 5.2 Добавить регрессионный тест на fallback‑путь (ожидаем `return_is_weak=true`, а не пустоту).

## 6. Тестирование
- [ ] 6.1 Юнит‑тесты: owner mapping для ObjectModule/RecordSetModule → корректный owner_type key.
- [ ] 6.2 Интеграционный тест: open file A вызывает экспорт B из object module; несохранённая правка в B меняет тип в A без переиндексации.
- [ ] 6.3 Тест: динамика в return inference не уничтожает known union и понижает `Certainty` до `InferredWeak` при типизации вызова.

## 7. Документация
- [ ] 7.1 Обновить `docs/architecture/type-inference-flow.md`: канонические owner_type ключи для индексируемых module types и правила weak/dynamic для persistent‑контура.

