
; C++, after c.scm.

(class_specifier name: (_) @name body: (_)) @item.class
(namespace_definition name: (_) @name) @item.module
(function_definition declarator: (reference_declarator (function_declarator declarator: (_) @name))) @item.function
(field_declaration declarator: (reference_declarator (function_declarator declarator: (_) @name))) @item.prototype
(alias_declaration name: (_) @name) @item.type
(concept_definition name: (_) @name) @item.type
(using_declaration (_) @name) @binding.import
(lambda_expression) @scope.function

(raw_string_literal) @token.string
