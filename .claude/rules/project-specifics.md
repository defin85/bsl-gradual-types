# Особенности проекта

## Окружение

Проект работает в WSL + Arch Linux. Windows доступен через /mnt/c/, /mnt/d/.

## GitBash на Windows

При работе через GitBash:
- Используй Unix-style команды (`ls`, `grep`, `find`)
- НЕ используй PowerShell syntax

## URL-encoding для кириллицы

```bash
# НЕ работает в GitBash
curl "http://localhost:3002/api/search?q=Массив"

# Работает
curl "http://localhost:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"
```

**Конвертация:**
```bash
python3 -c "import urllib.parse; print(urllib.parse.quote('Массив'))"
```

## 1С проекты НЕ тестируются

**ИСКЛЮЧЕНИЕ:** Проекты НА ПЛАТФОРМЕ 1С (встроенный язык) — НЕ запускать Tester

**Причина:** Нет testing framework для встроенного языка 1С

**Pipeline:** architect → coder → reviewer (без tester)

**НО:** Наш проект (BSL Gradual Types) написан на **Rust/TypeScript** → тестируется полностью!

## Ответы на русском

Всегда используй русский язык в ответах.
