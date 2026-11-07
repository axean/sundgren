; C, and C++ with cpp.scm after it.

(function_definition declarator: (function_declarator declarator: (_) @name)) @item.function
(function_definition declarator: (pointer_declarator declarator: (function_declarator declarator: (_) @name))) @item.function
(function_definition declarator: (pointer_declarator declarator: (pointer_declarator declarator: (function_declarator declarator: (_) @name)))) @item.function
(declaration declarator: (function_declarator declarator: (_) @name)) @binding.prototype
(declaration declarator: (pointer_declarator declarator: (function_declarator declarator: (_) @name))) @binding.prototype
(field_declaration declarator: (function_declarator declarator: (_) @name)) @item.prototype
(field_declaration declarator: (pointer_declarator declarator: (function_declarator declarator: (_) @name))) @item.prototype
(struct_specifier name: (_) @name body: (_)) @item.struct
(union_specifier name: (_) @name body: (_)) @item.union
(enum_specifier name: (_) @name body: (_)) @item.enum
(type_definition type: (struct_specifier !name body: (_)) declarator: (type_identifier) @name) @item.struct
(type_definition type: (union_specifier !name body: (_)) declarator: (type_identifier) @name) @item.union
(type_definition type: (enum_specifier !name body: (_)) declarator: (type_identifier) @name) @item.enum
(type_definition declarator: (type_identifier) @name) @item.type
(type_definition declarator: (pointer_declarator declarator: (type_identifier) @name)) @item.type
(enumerator name: (identifier) @name) @item.variant
(field_declaration declarator: (field_identifier) @name) @item.field
(field_declaration declarator: (pointer_declarator declarator: (field_identifier) @name)) @item.field
(field_declaration declarator: (array_declarator declarator: (field_identifier) @name)) @item.field
(preproc_def name: (identifier) @name) @item.macro
(preproc_function_def name: (identifier) @name) @item.macro
(preproc_include path: (_) @name) @item.import
(declaration declarator: (init_declarator declarator: (identifier) @name)) @binding.variable
(declaration declarator: (init_declarator declarator: (pointer_declarator declarator: (identifier) @name))) @binding.variable
(declaration declarator: (init_declarator declarator: (array_declarator declarator: (identifier) @name))) @binding.variable
(declaration declarator: (identifier) @name) @binding.variable
(declaration declarator: (pointer_declarator declarator: (identifier) @name)) @binding.variable
(declaration declarator: (array_declarator declarator: (identifier) @name)) @binding.variable
(translation_unit (expression_statement (call_expression function: (identifier) @name)) @binding.call)

((comment) @lead (#match? @lead "^(///|//!|/\\*\\*|/\\*!)"))

(comment) @token.comment
[(string_literal) (char_literal) (system_lib_string)] @token.string
(number_literal) @token.number
[(preproc_include) (preproc_def) (preproc_function_def) (preproc_call)] @token.directive
["#if" "#ifdef" "#ifndef" "#else" "#elif" "#endif"] @token.directive
