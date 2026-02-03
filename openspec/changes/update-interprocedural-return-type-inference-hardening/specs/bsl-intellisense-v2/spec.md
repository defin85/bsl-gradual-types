## ADDED Requirements

### Requirement: Return‑inference сохраняет “known union” и пробрасывает weak/dynamic через SignatureIndex (MUST)
Система MUST сохранять известные варианты возвращаемых типов (union) для пользовательских экспортов, доступных через `SignatureIndex`, и MUST пробрасывать признак неопределённости (weak/dynamic) до call-site типизации:

- если межпроцедурный вывод return‑типа смог вывести хотя бы один конкретный вариант, этот вариант MUST присутствовать в результате;
- если при выводе встретилась неопределённость (динамика/неразрешимый вызов/недостаток данных), результат MUST быть помечен как weak (например, `Certainty::InferredWeak`), но MUST NOT деградировать в “полный Unknown без вариантов”.

#### Scenario: Динамика не уничтожает известную часть union на call site через SignatureIndex
- **GIVEN** экспортная функция конфигурации возвращает `Строка` в одной ветке и динамический/неразрешимый результат в другой
- **AND** сигнатура этой функции доступна через `SignatureIndex`
- **WHEN** другой модуль типизирует вызов этой функции по `SignatureIndex`
- **THEN** результат содержит `Строка` как известный вариант
- **AND** результат помечен как weak/dynamic (например, `Certainty::InferredWeak`)

### Requirement: Overlay return‑summaries покрывает все открытые файлы индексируемых module types (MUST)
Система MUST поддерживать overlay return‑summaries для всех открытых файлов, которые относятся к модулям конфигурации, индексируемым в `SignatureIndex`:

- `CommonModule`
- `ManagerModule`
- `ObjectModule`
- `RecordSetModule`

Ключи owner_type в overlay MUST совпадать с теми, что используются в `SignatureIndex`:
- ObjectModule: `СправочникОбъект.<Имя>` / `ДокументОбъект.<Имя>` / …
- RecordSetModule: `РегистрНакопленияНаборЗаписей.<Имя>` / `РегистрСведенийНаборЗаписей.<Имя>` / …

#### Scenario: Несохранённая правка в ObjectModule влияет на типы вызовов в другом открытом файле
- **GIVEN** в IDE открыты два файла A и B
- **AND** B — ObjectModule (например, `Catalogs/Контрагенты/Ext/ObjectModule.bsl`) с экспортной функцией `F()`
- **AND** A вызывает `F()` на значении типа `СправочникОбъект.Контрагенты`
- **WHEN** пользователь меняет тело `F()` в B так, что её возвращаемый тип меняется
- **THEN** type-at-position/hover/completion в A используют обновлённый возвращаемый тип (из overlay), без переиндексации конфигурации

### Requirement: Fallback‑парсинг не выключает межпроцедурный return inference (MUST)
Если парсинг конфигурационного модуля деградирует в fallback‑режим, система MUST обеспечивать консервативные данные, достаточные для interprocedural return inference:

- return inference MUST продолжать работать (хотя бы деградированно);
- результат MUST сохранять known union (если есть) и помечать неопределённость как weak/dynamic;
- система MUST NOT терять return inference полностью из‑за отсутствия `return_facts`.

#### Scenario: AST fallback сохраняет weak/dynamic вместо “пустого” результата
- **GIVEN** модуль конфигурации попадает в AST fallback при парсинге
- **WHEN** индексация строит `SignatureIndex` и типизирует вызовы по нему
- **THEN** вызовы получают консервативный результат с weak/dynamic (если точный тип не удалось вывести), а не “пустой return_type”

