; JavaScript and TypeScript, with javascript.scm or typescript.scm after it.

(function_declaration name: (_) @name) @item.function
(generator_function_declaration name: (_) @name) @item.function
(class_declaration name: (_) @name) @item.class
(class_body (method_definition name: (_) @name) @item.function)
(variable_declarator name: (identifier) @name value: [(arrow_function) (function_expression) (generator_function)]) @binding.function
(variable_declarator name: (identifier) @name value: (class)) @binding.class
(lexical_declaration kind: "const" (variable_declarator name: (identifier) @name) @binding.const)
(lexical_declaration (variable_declarator name: (identifier) @name) @binding.variable)
(variable_declaration (variable_declarator name: (identifier) @name) @binding.variable)
(import_statement source: (string) @name) @binding.import
(export_statement source: (string) @name) @binding.import
(program (expression_statement (call_expression function: (_) @name)) @binding.call)
[(arrow_function) (function_expression) (generator_function)] @scope.function

((comment) @lead (#match? @lead "^/\\*\\*"))
(decorator) @lead

(comment) @token.comment
[(string) (template_string) (regex)] @token.string
(number) @token.number
