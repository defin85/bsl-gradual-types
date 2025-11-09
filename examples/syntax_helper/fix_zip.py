#!/usr/bin/env python3
"""
Восстанавливает ZIP архив из .hbk файла 1С
Удаляет мусорный заголовок и создаёт валидный ZIP
"""

# Читаем оригинальный файл
with open('shcntx_ru.hbk', 'rb') as f:
    data = f.read()

# Ищем первую ZIP signature (PK\x03\x04 - local file header)
zip_signature = b'PK\x03\x04'
start_pos = data.find(zip_signature)

if start_pos == -1:
    print("ZIP signature not found!")
    exit(1)

print(f"Found ZIP signature at offset: {start_pos}")
print(f"Removing {start_pos} bytes of junk header")

# Создаём восстановленный ZIP начиная с первой signature
with open('rebuilt.shcntx_ru.zip', 'wb') as f:
    f.write(data[start_pos:])

zip_size = len(data) - start_pos
print(f"Created rebuilt.shcntx_ru.zip ({zip_size:,} bytes)")
print(f"Original file size: {len(data):,} bytes")
print(f"Recovery successful!")
