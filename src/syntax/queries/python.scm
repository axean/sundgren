; Python.

(function_definition name: (identifier) @name) @item.function
(decorated_definition definition: (function_definition name: (identifier) @name)) @item.function
(class_definition name: (identifier) @name) @item.class
(decorated_definition definition: (class_definition name: (identifier) @name)) @item.class
(type_alias_statement left: (type (identifier) @name)) @item.type
(expression_statement (assignment left: (identifier) @name right: (lambda))) @binding.function
(expression_statement (assignment left: (identifier) @name)) @binding.variable
(import_statement name: (_) @name) @binding.import
(import_from_statement module_name: (_) @name) @binding.import
(module (expression_statement (call function: (_) @name)) @binding.call)
(lambda) @scope.function

(comment) @token.comment
; A string that is a whole statement documents.
(expression_statement (string) @token.comment)
(string) @token.string
[(integer) (float)] @token.number
