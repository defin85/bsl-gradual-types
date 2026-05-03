;comments
(line_comment) @comment

;methods
(procedure_definition name: (identifier) @function)
(function_definition name: (identifier) @function)
(method_call name: (identifier) @function.method)

;keywords
[
  (IF_KEYWORD) (THEN_KEYWORD) (ELSE_KEYWORD) (ELSIF_KEYWORD) (ENDIF_KEYWORD)
  (FOR_KEYWORD) (EACH_KEYWORD) (IN_KEYWORD) (TO_KEYWORD)
  (WHILE_KEYWORD) (DO_KEYWORD) (ENDDO_KEYWORD)
  (GOTO_KEYWORD) (RETURN_KEYWORD) (BREAK_KEYWORD) (CONTINUE_KEYWORD)
  (PROCEDURE_KEYWORD) (FUNCTION_KEYWORD) (ENDPROCEDURE_KEYWORD) (ENDFUNCTION_KEYWORD)
  (VAR_KEYWORD) (EXPORT_KEYWORD) (VAL_KEYWORD)
  (TRUE_KEYWORD) (FALSE_KEYWORD) (UNDEFINED_KEYWORD) (NULL_KEYWORD)
  (TRY_KEYWORD) (EXCEPT_KEYWORD) (RAISE_KEYWORD) (ENDTRY_KEYWORD)
  (ASYNC_KEYWORD) (AWAIT_KEYWORD)
  (ADDHANDLER_KEYWORD) (REMOVEHANDLER_KEYWORD)
  (PREPROC_REGION_KEYWORD) (PREPROC_ENDREGION_KEYWORD)
  (PREPROC_IF_KEYWORD) (PREPROC_ELSE_KEYWORD) (PREPROC_ELSIF_KEYWORD) (PREPROC_ENDIF_KEYWORD)
] @keyword

;operators
(operator) @operator
(annotation) @attribute
(NEW_KEYWORD) @constructor

;strings
[(string) (string_content)] @string

;numbers and dates
(number) @number
(date) @string.special

;brackets
["(" ")" "[" "]"] @punctuation.bracket

;delimiters
[";" "." ","] @punctuation.delimiter
