# Конфигурация BSL Gradual Types

Этот документ описывает систему конфигурации для веб-сервера и фронтенда BSL Gradual Types.

## Веб-сервер (Backend)

### Способы конфигурации

Веб-сервер поддерживает несколько способов конфигурации в порядке приоритета:

1. **Аргументы командной строки** (высший приоритет)
2. **Переменные окружения**
3. **Конфигурационный файл**
4. **Значения по умолчанию** (низший приоритет)

### Аргументы командной строки

```bash
# Запуск с пользовательскими параметрами
cargo run --bin bsl-web-server -- \
  --host 0.0.0.0 \
  --port 3000 \
  --static-dir ./dist \
  --config ./my-config.toml

# Показать справку
cargo run --bin bsl-web-server -- --help
```

### Переменные окружения

```bash
# Основные настройки сервера
export BSL_HOST="0.0.0.0"
export BSL_PORT="3000"
export BSL_STATIC_DIR="./dist"
export BSL_CONFIG_FILE="./config.toml"

# Настройки логирования
export BSL_LOG_LEVEL="debug"
export BSL_LOG_FORMAT="pretty"

# Настройки CORS
export BSL_CORS_ORIGINS="http://localhost:3000,http://127.0.0.1:3000"
export BSL_CORS_METHODS="GET,POST,PUT,DELETE,OPTIONS"
export BSL_CORS_HEADERS="Content-Type,Authorization,Accept"

# Настройки API
export BSL_API_PREFIX="/api"
export BSL_API_TIMEOUT="60"
export BSL_API_MAX_BODY_SIZE="2097152"  # 2MB
```

### Конфигурационный файл

По умолчанию используется файл `backend/config.toml`. Пример конфигурации:

```toml
[server]
host = "127.0.0.1"
port = 8080
static_dir = "../frontend/dist"

[cors]
allowed_origins = ["http://localhost:8080"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["Content-Type", "Authorization", "Accept"]

[logging]
level = "info"  # error, warn, info, debug, trace
format = "compact"  # compact, pretty, json

[api]
prefix = "/api"
timeout = 30
max_body_size = 1048576  # 1MB
```

### Значения по умолчанию

- **Host**: `127.0.0.1`
- **Port**: `8080`
- **Static Directory**: `../frontend/dist`
- **Log Level**: `info`
- **Log Format**: `compact`
- **API Prefix**: `/api`
- **API Timeout**: `30` секунд
- **Max Body Size**: `1048576` байт (1MB)

## Фронтенд

### Конфигурация API

Фронтенд автоматически определяет базовый URL для API на основе текущего домена:

- **Разработка**: `http://localhost:8080/api`
- **Продакшн**: `{current_origin}/api`

### Переменные окружения браузера

Можно переопределить базовый URL через localStorage:

```javascript
// В консоли браузера
localStorage.setItem('BSL_API_BASE_URL', 'http://custom-server:3000/api');

// Перезагрузить страницу для применения изменений
location.reload();
```

## Примеры использования

### Разработка

```bash
# Запуск с настройками для разработки
export BSL_LOG_LEVEL="debug"
export BSL_LOG_FORMAT="pretty"
cargo run --bin bsl-web-server
```

### Продакшн

```bash
# Запуск на всех интерфейсах
cargo run --bin bsl-web-server -- \
  --host 0.0.0.0 \
  --port 80 \
  --config /etc/bsl/config.toml
```

### Docker

```dockerfile
# Пример Dockerfile
FROM rust:1.70 as builder
# ... сборка ...

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/bsl-web-server /usr/local/bin/
COPY config.toml /etc/bsl/config.toml

# Конфигурация через переменные окружения
ENV BSL_HOST=0.0.0.0
ENV BSL_PORT=8080
ENV BSL_CONFIG_FILE=/etc/bsl/config.toml

EXPOSE 8080
CMD ["bsl-web-server"]
```

## Безопасность

### CORS

Обязательно настройте CORS для продакшн среды:

```toml
[cors]
allowed_origins = [
  "https://yourdomain.com",
  "https://www.yourdomain.com"
]
allowed_methods = ["GET", "POST"]
allowed_headers = ["Content-Type"]
```

### Логирование

В продакшн используйте уровень `warn` или `error`:

```toml
[logging]
level = "warn"
format = "json"  # Для структурированных логов
```

## Устранение неполадок

### Проверка конфигурации

```bash
# Запуск с выводом конфигурации
BSL_LOG_LEVEL=debug cargo run --bin bsl-web-server -- --help
```

### Частые проблемы

1. **Порт занят**: Измените порт через `--port` или `BSL_PORT`
2. **Статические файлы не найдены**: Проверьте путь в `--static-dir`
3. **CORS ошибки**: Добавьте ваш домен в `allowed_origins`
4. **API недоступен**: Проверьте, что сервер запущен и доступен

### Логи

Для отладки включите подробное логирование:

```bash
BSL_LOG_LEVEL=debug BSL_LOG_FORMAT=pretty cargo run --bin bsl-web-server
```