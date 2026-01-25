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