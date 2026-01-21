## 1. Проектирование и контракт
- [x] 1.1 Зафиксировать приоритет источников статики: `BSL_AGENT_HTTP_STATIC_DIR` (если задан) → embedded.
- [x] 1.2 Зафиксировать build-time требование: сборка `bsl-agent` должна падать, если `target/site` отсутствует, с понятным сообщением.
- [x] 1.3 Зафиксировать поведение fallback для SPA роутинга (SPA index fallback) в embedded режиме.

## 2. Реализация: embedded статика в `bsl-agent`
- [x] 2.1 Добавить build-step (например `build.rs`) для проверки наличия `target/site` и понятной ошибки при отсутствии.
- [x] 2.2 Добавить механизм embed директории `target/site` в бинарник (выбранный crate/подход).
- [x] 2.3 Реализовать раздачу embedded файлов через axum/tower-http, включая fallback на `index.html`.
- [x] 2.4 Сохранить поддержку `BSL_AGENT_HTTP_STATIC_DIR` и сделать её приоритетной над embedded.

## 3. Тестирование и валидация
- [x] 3.1 Интеграционный тест: без `BSL_AGENT_HTTP_STATIC_DIR` UI отдаёт embedded `index.html`.
- [x] 3.2 Интеграционный тест: при заданном `BSL_AGENT_HTTP_STATIC_DIR` UI отдаёт статику с диска (override embedded).
- [ ] 3.3 Проверка build-time: при отсутствии `target/site` сборка `bsl-agent` завершается ошибкой с ожидаемым сообщением (CI/скрипт).

## 4. Документация
- [x] 4.1 Обновить `bsl-agent/README.md`: UI работает без внешней статики; `BSL_AGENT_HTTP_STATIC_DIR` используется для override/разработки.
