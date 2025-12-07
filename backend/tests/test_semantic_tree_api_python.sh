#!/usr/bin/env bash
# Python-based testing for /api/semantic-tree endpoint with Cyrillic support
#
# Prerequisites:
# 1. Web server must be running: cargo run -p bsl-backend --bin bsl-web-server
# 2. Server should be listening on http://localhost:3002
# 3. Python 3 with json module (standard library)
#
# Usage:
#   chmod +x backend/tests/test_semantic_tree_api_python.sh
#   ./backend/tests/test_semantic_tree_api_python.sh

set -e

BASE_URL="http://localhost:3002"
API_ENDPOINT="$BASE_URL/api/semantic-tree"

echo "=========================================="
echo "Testing /api/semantic-tree with Python"
echo "=========================================="
echo ""

# Test 1: Simple procedure (Cyrillic)
echo "Test 1: Simple procedure with Cyrillic"
echo "---------------------------------------"
python3 -c "
import json, codecs
import urllib.request

code = '''Процедура ТестоваяПроцедура()
    Сообщить(\"Привет\");
КонецПроцедуры'''

data = {
    'code': code,
    'file_path': 'test.bsl'
}

json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode('utf-8'))
    print(json.dumps(result, indent=2, ensure_ascii=False))
"
echo ""
echo ""

# Test 2: Function with return (check metrics only)
echo "Test 2: Function with return (metrics)"
echo "---------------------------------------"
python3 -c "
import json
import urllib.request

code = '''Функция ПолучитьЧисло()
    Возврат 42;
КонецФункции'''

data = {'code': code, 'file_path': 'test.bsl'}
json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode('utf-8'))
    print('Metrics:', json.dumps(result['metrics'], indent=2))
"
echo ""
echo ""

# Test 3: Multiple functions
echo "Test 3: Multiple procedures and functions"
echo "------------------------------------------"
python3 -c "
import json
import urllib.request

code = '''Процедура Первая()
    Сообщить(\"Первая\");
КонецПроцедуры

Функция Вторая()
    Возврат 1;
КонецФункции

Процедура Третья()
    Сообщить(\"Третья\");
КонецПроцедуры'''

data = {'code': code, 'file_path': 'test.bsl'}
json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode('utf-8'))
    print('File path:', result['file_path'])
    print('Root nodes:', len(result['root_nodes']))
    print('Procedures:', result['metrics']['procedure_count'])
    print('Functions:', result['metrics']['function_count'])
    print('Node count:', result['metrics']['node_count'])
"
echo ""
echo ""

# Test 4: Variables in procedure
echo "Test 4: Variables in procedure body"
echo "------------------------------------"
python3 -c "
import json
import urllib.request

code = '''Процедура Тест()
    Переменная1 = 10;
    Переменная2 = \"Строка\";
    Переменная3 = Новый Массив;
КонецПроцедуры'''

data = {'code': code, 'file_path': 'test.bsl'}
json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode('utf-8'))
    proc = result['root_nodes'][0]
    print('Procedure name:', proc.get('name'))
    print('Children count:', len(proc.get('children', [])))
    print('Total nodes:', result['metrics']['node_count'])
"
echo ""
echo ""

# Test 5: Procedure with parameters
echo "Test 5: Procedure with parameters"
echo "----------------------------------"
python3 -c "
import json
import urllib.request

code = '''Процедура ПроцедураСПараметрами(Параметр1, Параметр2, Параметр3)
    Сообщить(Параметр1);
КонецПроцедуры'''

data = {'code': code, 'file_path': 'test.bsl'}
json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode('utf-8'))
    proc = result['root_nodes'][0]
    print('Procedure name:', proc.get('name'))
    print('Parameter count:', proc.get('attributes', {}).get('parameter_count'))
"
echo ""
echo ""

# Test 6: Root nodes structure
echo "Test 6: Root nodes structure details"
echo "-------------------------------------"
python3 -c "
import json
import urllib.request

code = '''Процедура Тест()
    Переменная = 123;
КонецПроцедуры'''

data = {'code': code, 'file_path': 'test.bsl'}
json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

with urllib.request.urlopen(req) as response:
    result = json.loads(response.read().decode('utf-8'))
    for i, node in enumerate(result['root_nodes']):
        print(f'Node {i}:')
        print(f\"  Kind: {node.get('kind')}\")
        print(f\"  Name: {node.get('name')}\")
        print(f\"  Location: line={node.get('location', {}).get('line')}, col={node.get('location', {}).get('column')}\")
        print(f\"  Attributes: {node.get('attributes')}\")
        print(f\"  Children: {len(node.get('children', []))}\")
"
echo ""
echo ""

# Test 7: Error handling - invalid code
echo "Test 7: Error handling (syntax error)"
echo "--------------------------------------"
python3 -c "
import json
import urllib.request

code = '''Процедура БезЗакрытия()
    Сообщить(\"Ошибка\");
# Missing КонецПроцедуры'''

data = {'code': code, 'file_path': 'error.bsl'}
json_data = json.dumps(data, ensure_ascii=False).encode('utf-8')
req = urllib.request.Request(
    '$API_ENDPOINT',
    data=json_data,
    headers={'Content-Type': 'application/json; charset=utf-8'}
)

try:
    with urllib.request.urlopen(req) as response:
        result = json.loads(response.read().decode('utf-8'))
        print('Response received (may have partial tree):')
        print('Root nodes:', len(result.get('root_nodes', [])))
        print('Metrics:', result.get('metrics'))
except urllib.error.HTTPError as e:
    print(f'HTTP Error {e.code}: {e.reason}')
    print('Error response:', e.read().decode('utf-8'))
"
echo ""
echo ""

echo "=========================================="
echo "All Python tests completed!"
echo "=========================================="
