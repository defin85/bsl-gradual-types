#!/usr/bin/env python3
"""
Тестовый скрипт для проверки автодополнения в LSP сервере
"""

import json
import subprocess
import sys
import time

def send_request(proc, request):
    """Отправляет JSON-RPC запрос в LSP сервер"""
    content = json.dumps(request)
    message = f"Content-Length: {len(content)}\r\n\r\n{content}"
    proc.stdin.write(message.encode())
    proc.stdin.flush()
    
def read_response(proc):
    """Читает ответ от LSP сервера"""
    # Читаем заголовок
    header = b""
    while True:
        byte = proc.stdout.read(1)
        header += byte
        if header.endswith(b"\r\n\r\n"):
            break
    
    # Извлекаем длину контента
    header_str = header.decode()
    for line in header_str.split("\r\n"):
        if line.startswith("Content-Length:"):
            content_length = int(line.split(":")[1].strip())
            break
    
    # Читаем контент
    content = proc.stdout.read(content_length)
    return json.loads(content.decode())

def main():
    # Запускаем LSP сервер
    proc = subprocess.Popen(
        ["cargo", "run", "--bin", "lsp-server"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr
    )
    
    try:
        # Initialize запрос
        send_request(proc, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": None,
                "rootUri": None,
                "capabilities": {}
            }
        })
        
        response = read_response(proc)
        print("Initialize response:", json.dumps(response, indent=2, ensure_ascii=False))
        
        # Initialized notification
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        })
        
        # Открываем документ
        test_content = """Процедура Тест()
    МассивТест = Новый Массив;
    МассивТест.
КонецПроцедуры"""
        
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///test.bsl",
                    "languageId": "bsl",
                    "version": 1,
                    "text": test_content
                }
            }
        })
        
        time.sleep(0.5)  # Даём время на обработку
        
        # Тест 1: Автодополнение после "МассивТест."
        print("\n=== Тест 1: Автодополнение для Массив ===")
        send_request(proc, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": "file:///test.bsl"
                },
                "position": {
                    "line": 2,
                    "character": 15  # После "МассивТест."
                }
            }
        })
        
        response = read_response(proc)
        if "result" in response and response["result"]:
            print(f"Найдено {len(response['result'])} автодополнений:")
            for item in response["result"][:5]:  # Показываем первые 5
                print(f"  - {item['label']}: {item.get('detail', '')}")
        else:
            print("Автодополнений не найдено")
        
        # Тест 2: Автодополнение для глобальных функций
        print("\n=== Тест 2: Автодополнение глобальных функций ===")
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///test.bsl",
                    "version": 2
                },
                "contentChanges": [{
                    "text": """Процедура Тест()
    Сооб
КонецПроцедуры"""
                }]
            }
        })
        
        send_request(proc, {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": "file:///test.bsl"
                },
                "position": {
                    "line": 1,
                    "character": 8  # После "Сооб"
                }
            }
        })
        
        response = read_response(proc)
        if "result" in response and response["result"]:
            print(f"Найдено {len(response['result'])} автодополнений:")
            for item in response["result"][:5]:
                print(f"  - {item['label']}: {item.get('detail', '')}")
        else:
            print("Автодополнений не найдено")
        
        # Тест 3: Автодополнение для Справочники
        print("\n=== Тест 3: Автодополнение для Справочники ===")
        send_request(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///test.bsl",
                    "version": 3
                },
                "contentChanges": [{
                    "text": """Процедура Тест()
    Справочники.
КонецПроцедуры"""
                }]
            }
        })
        
        send_request(proc, {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": "file:///test.bsl"
                },
                "position": {
                    "line": 1,
                    "character": 16  # После "Справочники."
                }
            }
        })
        
        response = read_response(proc)
        if "result" in response and response["result"]:
            print(f"Найдено {len(response['result'])} автодополнений:")
            for item in response["result"][:5]:
                print(f"  - {item['label']}: {item.get('detail', '')}")
        else:
            print("Автодополнений не найдено")
        
        # Shutdown
        send_request(proc, {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "shutdown"
        })
        
        response = read_response(proc)
        print("\nShutdown response:", response)
        
    finally:
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    main()