Я сделал вывод return-типа в два шага: сначала вычисляю return_type у объявлений функций, потом использую его при резолве вызовов и присваиваний.

  1. Сбор объявлений функций/процедур (pass #1 по AST)

  - На первом проходе collect_global_symbols регистрирует в SymbolTable все FunctionDecl/ProcedureDecl верхнего уровня, чтобы работали forward reference (функция может быть ниже по тексту).
  - Но на этом этапе return_type = None (пока не анализировали тело).

  2. Пост‑проход по готовому IR: вывести return_type из Return узлов

  - После основного построения IR я добавил вызов:
      - infer_function_return_types_from_ir() — ищет SemanticNodeKind::FunctionDeclaration, берёт body (список индексов узлов тела), рекурсивно собирает все Return { value_type } внутри If/Loops/Try и т.п., и
        строит итоговый тип:
          - если return без значения → Неопределено
          - если нет return’ов → Неопределено
          - если встречается Unknown → итог Unknown (консервативно)
          - иначе делает union по типам возвращаемых значений.
      - Затем записывает результат:
          - в сам FunctionDeclaration.return_type
          - и в SymbolTable через set_function_return_type(...)
  - Код: semantic/src/converter.rs:147 (вызов пост‑пасс), semantic/src/converter.rs:164 (реализация).

  3. Резолв вызова ИмяФункции()

  - В resolve_global_function_return_type() я добавил ветку: если это пользовательская функция из SymbolTable и return_type уже вычислен — возвращаю его.
  - Код: semantic/src/type_inference.rs:741.

  4. “Догоняющее” проставление типов в IR и переменных

  - После того как SymbolTable знает return-типы функций, я делаю apply_inferred_function_return_types():
      - пробегаю FunctionCall { object_type: None } и если result_type был Unknown, подставляю вычисленный return-type (по имени функции).
      - затем пробегаю Assignment, и если value_node указывает на такой FunctionCall, обновляю Assignment.value_type и тип самой переменной в таблице символов.
  - Это нужно, чтобы КакаяТоСтрока = ФункцияКотораяВозвращаетСтроку(); получила тип даже если на момент конвертации выражения return_type ещё не был известен.
  - Код: semantic/src/converter.rs:196.

  Если хочешь, могу расширить логику merge return-types (например, учитывать Возврат; в функции как Неопределено, более мягко обращаться с Unknown, или делать более точный union с весами).

  1. update-vscode-artifacts-policy

  - Почему рано: затрагивает массовое удаление из git индекса vscode-extension/out/** и vscode-extension/*.vsix (они реально tracked сейчас), плюс правки в README.md/vscode-extension/INSTALLATION.md.
  - Это самый “шумный” diff; лучше сделать первым, чтобы остальные PR’ы не мерджили гигантские удаления.

  2. update-cargo-lockfile-policy

  - Почему рано: правит корневой .gitignore (сейчас Cargo.lock игнорируется: .gitignore:3) и документацию про структуру/воспроизводимость.
  - Чем раньше зафиксируем lockfile-политику, тем меньше шансов словить конфликт в .gitignore и доках.

  3. add-bsl-lsp-inlay-hints-and-code-actions

  - Почему здесь: большая функциональная фича, затрагивает backend/src/bin/lsp_server/** и тесты LSP.
  - Минимум пересечений с bsl-agent, но потенциально может подтянуть изменения в общих слоях (анализ/позиции/DTO), поэтому лучше сделать до “полировки” docs/scripts.

  4. add-mcp-bsl-agent-type-tools

  - Почему после LSP: в основном локально для bsl-agent/src/server/** и bsl-agent/src/session/**, опирается на существующие DTO (TypeDto/AnalysisResultDto).
  - Конфликтов с LSP-change почти не должно быть; пересечения возможны только если оба change полезут в shared DTO/типовую модель.

  5. update-docs-and-scripts-consistency

  - Почему после крупных code-change: правит конкретные места (docs/guides/roadmap-verification.md, scripts/compact_completed_milestones.py — сейчас там есть ссылки на несуществующий backend/src/system/
    tree_sitter_adapter.rs).
  - Делая после фич, уменьшаем шанс, что придётся “переправлять ссылки” повторно из‑за переездов/рефакторингов.

  6. update-ci-references

  - Почему ближе к концу: это широкий doc sweep (точно трогает README.md — CI badge сейчас фиктивный, и CONTRIBUTING.md тоже), легко конфликтует почти с любыми док-правками.
  - Лучше выполнить, когда README/INSTALLATION уже стабилизированы предыдущими changes.

  7. reduce-test-skip-surface

  - Почему последним: почти наверняка будет трогать scripts/run-intellisense-tests.sh, scripts/README.md, backend/tests/** (ignored/disabled).
  - Если сделать раньше, то фичи (п.3–4) добавят/поменяют тесты и придётся опять править “smoke/full” и документацию → лишняя волна конфликтов в scripts/**.

  Отдельно важное про OpenSpec-конфликты:

  - 5 changes (reduce-test-skip-surface, update-cargo-lockfile-policy, update-ci-references, update-docs-and-scripts-consistency, update-vscode-artifacts-policy) все добавляют дельты в один и тот же
    capability: dev-workflow (openspec/specs/dev-workflow/spec.md). Чтобы минимизировать конфликты при archiving, их лучше делать без параллельных веток, строго последовательно, каждый раз начиная с
    актуального master.