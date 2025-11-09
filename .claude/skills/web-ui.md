# Web UI Launcher

Запускает BSL Gradual Types веб-сервер с фронтендом для просмотра типов в браузере.

## Что делает

1. Собирает frontend (Rust WASM приложение через Trunk)
2. Копирует статику в `backend/static/`
3. Запускает web server с platform types и фронтендом
4. Открывает браузер на http://127.0.0.1:8080

## Использование

```bash
# Запуск из корня проекта
/web-ui

# Или вручную:
cd frontend && trunk build --release && cd ..
cargo run --release -p bsl-backend --bin bsl-web-server -- \
  --port 8080 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper/rebuilt.shcntx_ru \
  --static-files-path backend/static
```

## Компоненты

**Frontend (WASM):**
- Путь: `frontend/`
- Билд: `trunk build --release`
- Выход: `backend/static/` (index.html + *.wasm + *.js)

**Backend (Web Server):**
- Путь: `backend/src/main.rs`
- Бинарник: `bsl-web-server`
- Порт: 8080 (по умолчанию)

## Endpoints

- `GET /` — фронтенд (React UI)
- `GET /api/health` — health check
- `GET /api/types` — все типы (JSON)
- `GET /api/search?q=<query>` — поиск типов

## Параметры

- `--port` — порт веб-сервера (по умолчанию 8080)
- `--enable-cors` — CORS для API (true/false)
- `--syntax-helper-path` — путь к platform types
- `--static-files-path` — путь к фронтенду (по умолчанию backend/static)

## Troubleshooting

**404 на корневом пути:**
- Проверь, что frontend собран: `ls backend/static/index.html`
- Пересобери frontend: `cd frontend && trunk build --release`

**WASM не загружается:**
- Проверь MIME types в browser console
- Убедись что файлы `*.wasm` и `*.js` есть в `backend/static/`

**Типы не загружаются:**
- Проверь `--syntax-helper-path` указывает на правильную директорию
- Должно быть: `examples/syntax_helper/rebuilt.shcntx_ru`
