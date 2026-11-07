
; TypeScript, after ecmascript.scm.

(function_signature name: (_) @name) @item.prototype
(abstract_class_declaration name: (_) @name) @item.class
(method_signature name: (_) @name) @item.prototype
(abstract_method_signature name: (_) @name) @item.prototype
(interface_declaration name: (_) @name) @item.interface
(enum_declaration name: (_) @name) @item.enum
(enum_assignment name: (_) @name) @item.variant
(enum_body (property_identifier) @name @item.variant)
(type_alias_declaration name: (_) @name) @item.type
(internal_module name: (_) @name) @item.module
(module name: (_) @name) @item.module
(ambient_declaration "global" @name (statement_block)) @item.module
(public_field_definition name: (_) @name value: [(arrow_function) (function_expression)]) @item.function
(public_field_definition name: (_) @name) @item.field
(property_signature name: (_) @name) @item.field
