#!/bin/bash
# Тестовый скрипт для проверки LSP сервера

LSP_SERVER="./bin/lsp_server.exe"

# LSP initialize request (правильный формат с Content-Length)
REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
CONTENT_LENGTH=${#REQUEST}

echo "🧪 Тестирование LSP сервера..."
echo "📍 Путь: $LSP_SERVER"
echo "📝 Отправка initialize request..."

# Отправляем LSP сообщение в правильном формате
(
  echo -ne "Content-Length: $CONTENT_LENGTH\r\n\r\n"
  echo -n "$REQUEST"
) | timeout 5s "$LSP_SERVER" 2>&1 | head -30

echo ""
echo "✅ Тест завершён"
