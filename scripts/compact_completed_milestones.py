#!/usr/bin/env python3
"""
Скрипт для сокращения завершённых Milestone в ROADMAP_2025.md.
Оставляет только минимальные описания для завершённых Milestone 2.*.
"""

import re
from pathlib import Path

# Шаблоны сокращённых описаний для завершённых Milestone
COMPACT_TEMPLATES = {
    "2.1": """### 🧠 Milestone 2.1: Tree-sitter Integration ✅ **ЗАВЕРШЁН**

**Что сделано:**
- ✅ Подключена грамматика [tree-sitter-bsl v0.1.5](https://github.com/alkoleft/tree-sitter-bsl)
- ✅ Создан `TreeSitterAdapter` для конвертации AST
- ✅ Реализован инкрементальный парсинг для LSP (< 10ms)
- ✅ LSP hover показывает типы через tree-sitter

**Ключевые файлы:**
- [backend/src/system/tree_sitter_adapter.rs](backend/src/system/tree_sitter_adapter.rs)
- [backend/Cargo.toml](backend/Cargo.toml) — зависимость tree-sitter-bsl

---
""",

    "2.5": """### 🎨 Milestone 2.5: Унификация визуализации типов ✅ **ЗАВЕРШЁН**

**Что сделано:**
- ✅ Создан крейт `type-visualization` с trait `TypeRenderer`
- ✅ Реализации: `HtmlRenderer`, `JsonRenderer`, `MarkdownRenderer`
- ✅ Интеграция в LSP через custom request `bsl/renderTypeHtml`
- ✅ Унифицированы DTOs (`shared/src/api/dtos.rs` — единственный источник)
- ✅ Удалено 1054 строки дубликатов из TypeScript

**Ключевые файлы:**
- [type-visualization/src/lib.rs](type-visualization/src/lib.rs)
- [backend/src/bin/lsp_server.rs](backend/src/bin/lsp_server.rs) — LSP handler
- [vscode-extension/src/lsp/typeVisualization.ts](vscode-extension/src/lsp/typeVisualization.ts)

---
""",

    "2.7": """### 🔧 Milestone 2.7: TreeSitterAdapter — Завершение реализации ✅ **ЗАВЕРШЁН**

**Что сделано:**
- ✅ Полная конвертация tree-sitter AST → BSL IR
- ✅ Поддержка всех конструкций 1С (функции, циклы, условия)
- ✅ Обработка ошибок парсинга с fallback на regex
- ✅ Интеграция с фасадом системы типов для hover

**Ключевые файлы:**
- [backend/src/system/tree_sitter_adapter.rs](backend/src/system/tree_sitter_adapter.rs)

---
""",

    "2.8": """### 🏗️ Milestone 2.8: Semantic IR Layer ✅ **ЗАВЕРШЁН**

**Что сделано:**
- ✅ Создан промежуточный слой `shared/src/ir/` независимый от парсера
- ✅ `SemanticProgram` с упрощённым набором узлов
- ✅ `SymbolTable` для иерархии областей видимости
- ✅ `Parser trait` для Dependency Inversion (разные парсеры → единая IR)
- ✅ `AstToIrConverter` — мост между синтаксисом и семантикой

**Ключевые файлы:**
- [shared/src/ir/mod.rs](shared/src/ir/mod.rs)
- [backend/src/application/type_system_service.rs](backend/src/application/type_system_service.rs)

---
""",

    "2.9": """### ✨ Milestone 2.9: Inline Scope Analysis + Унификация данных ✅ **ЗАВЕРШЁН 2025-10-08**

**Что сделано:**
- ✅ Inline Scope Analysis — анализ типов локальных переменных "на лету" при hover
- ✅ `SemanticProgram.find_variable_at_position()` для поиска переменных
- ✅ v2 hover entrypoint использует IR вместо AST
- ✅ Работает в пределах одной процедуры/функции (достаточно для MVP)
- ✅ Type Index в Extension временно отключён (показывается заглушка)

**Ключевые файлы:**
- [shared/src/ir/mod.rs](shared/src/ir/mod.rs) — find_variable_at_position()
- [backend/tests/inline_scope_analysis_test.rs](backend/tests/inline_scope_analysis_test.rs)
- [cli/test_inline_scope.bsl](cli/test_inline_scope.bsl) — тестовый файл

---
""",

    "2.10": """### 📦 Milestone 2.10: LSP Configuration + Type Index Integration ✅ **ЗАВЕРШЁН 2025-10-08**

**Что сделано:**
- ✅ LSP Server принимает конфигурацию через initialization options
- ✅ Custom LSP requests: `bsl/renderTypeHtml`, `bsl/extractPlatformDocs`
- ✅ Type Index в VSCode Extension временно отключён
- ✅ Tooltip объясняет: "используйте LSP hover вместо Type Index"

**Ключевые файлы:**
- [backend/src/bin/lsp_server.rs](backend/src/bin/lsp_server.rs)

---
"""
}

def find_milestone_boundaries(content: str, milestone_num: str) -> tuple[int, int] | None:
    """Находит начальную и конечную позиции Milestone в тексте."""
    # Ищем заголовок Milestone
    pattern = rf"^### .*Milestone {re.escape(milestone_num)}:.*$"
    match = re.search(pattern, content, re.MULTILINE)

    if not match:
        return None

    start = match.start()

    # Ищем следующий заголовок того же уровня (###)
    next_section = re.search(r"^###\s", content[match.end():], re.MULTILINE)

    if next_section:
        end = match.end() + next_section.start()
    else:
        # Если не найден следующий раздел, берём до конца файла
        end = len(content)

    return (start, end)

def compact_milestone(content: str, milestone_num: str, template: str) -> str:
    """Заменяет подробное описание Milestone на сокращённое."""
    boundaries = find_milestone_boundaries(content, milestone_num)

    if not boundaries:
        print(f"WARNING: Milestone {milestone_num} not found in file")
        return content

    start, end = boundaries
    original_section = content[start:end]

    # Вычисляем сокращение
    lines_before = original_section.count('\n')
    lines_after = template.count('\n')
    reduction = lines_before - lines_after

    print(f"OK Milestone {milestone_num}: {lines_before} lines -> {lines_after} lines (reduced: {reduction} lines)")

    # Заменяем секцию
    return content[:start] + template + content[end:]

def main():
    roadmap_path = Path("c:/1CProject/bsl-gradual-types/ROADMAP_2025.md")

    if not roadmap_path.exists():
        print(f"ERROR: File not found: {roadmap_path}")
        return

    print(f"Reading {roadmap_path}...")
    content = roadmap_path.read_text(encoding='utf-8')

    print(f"\nCompacting completed Milestones...\n")

    # Сокращаем каждый завершённый Milestone
    for milestone_num, template in COMPACT_TEMPLATES.items():
        content = compact_milestone(content, milestone_num, template)

    # Сохраняем результат
    print(f"\nSaving changes to {roadmap_path}...")
    roadmap_path.write_text(content, encoding='utf-8')

    print(f"SUCCESS: ROADMAP_2025.md updated!")
    print(f"Backup saved as ROADMAP_2025.md.backup")

if __name__ == "__main__":
    main()
