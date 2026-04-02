## 1. Runtime-контракт
- [x] 1.1 Уточнить и реализовать contract, что superseded same-file completion на existing completion path освобождает active turn / interactive ownership не позже ближайшего cooperative checkpoint после потери latest-wins.
- [x] 1.2 Сделать `response_build` path кооперативно cancellable внутри `collect` / `rank` / `format` или через эквивалентную interruptible boundary, чтобы stale request не дожёвывал multi-second tail после supersession.
- [x] 1.3 Сохранить bounded cancel/superseded outcome для старого request и запретить late publish user-facing completion ответа после потери актуальности.

## 2. Валидация и артефакты
- [x] 2.1 Добавить live regression для overlapping same-file completion, который доказывает first-poll budget нового request и bounded cancellation старого request на default runtime path.
- [x] 2.2 Расширить representative real-module gate overlap profile для same-file supersession и обновить checked-in evidence.
- [x] 2.3 Обновить shipped scripts / runbook / CI только там, где это требуется для нового overlap gate и его default verification path.

## 3. Гигиена change
- [x] 3.1 Прогнать `openspec validate refactor-completion-superseded-active-turn-release --strict --no-interactive`.
- [x] 3.2 Провести архитектурный review change против `refactor-completion-prepare-lightweight-exact-split` и `refactor-document-symbol-interactive-isolation`, подтвердив, что fix остаётся на existing completion path и не превращается в admission workaround или общий executor redesign.

> Зависимости: `2.2` опирается на runtime contract из `1.1`-`1.3`, а `2.3` нельзя закрывать до появления финального overlap evidence. Change остаётся completion-scoped и не должен переоткрывать `documentSymbol` или detached-snapshot follow-up.
