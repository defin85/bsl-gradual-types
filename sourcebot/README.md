# Sourcebot для BSL Gradual Types

Локальное развертывание Sourcebot для семантического поиска и анализа BSL проекта.

## Запуск

### Windows PowerShell (рекомендуется):
```powershell
cd C:\1CProject\bsl-gradual-types\sourcebot

docker run `
    -p 3000:3000 `
    -d `
    --pull=always `
    --rm `
    -v "${PWD}:/data" `
    -v "C:\1CProject\bsl-gradual-types:/repo" `
    -e CONFIG_PATH=/data/config.json `
    --name sourcebot `
    ghcr.io/sourcebot-dev/sourcebot:latest
```

### Command Prompt:
```cmd
cd C:\1CProject\bsl-gradual-types\sourcebot

docker run -p 3000:3000 --pull=always --rm -v "%CD%:/data" -v "C:\1CProject\bsl-gradual-types:/repo" -e CONFIG_PATH=/data/config.json --name sourcebot ghcr.io/sourcebot-dev/sourcebot:latest
```

## Использование

1. **Веб-интерфейс**: http://localhost:3000
2. **Семантический поиск**: "SystemCoordinator", "TypeResolver", "AnalysisEngine"
3. **Анализ архитектуры**: "simplified architecture", "domain layer"
4. **MCP интеграция**: доступ через Claude Code

## Остановка

```bash
docker stop sourcebot
# или Ctrl+C
```

## Конфигурация

Файл `config.json` содержит настройки подключения к GitHub репозиторию:

```json
{
    "$schema": "https://raw.githubusercontent.com/sourcebot-dev/sourcebot/main/schemas/v3/index.json",
    "connections": {
        "bsl-project": {
            "type": "github",
            "repos": [
                "defin85/bsl-gradual-types"
            ]
        }
    }
}
```

**Примечание**: Sourcebot индексирует код напрямую с GitHub, что позволяет работать с актуальной версией репозитория.