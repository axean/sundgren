; Rust.

(function_item name: (identifier) @name) @item.function
(function_signature_item name: (identifier) @name) @item.prototype
(struct_item name: (type_identifier) @name) @item.struct
(enum_item name: (type_identifier) @name) @item.enum
(union_item name: (type_identifier) @name) @item.union
(trait_item name: (type_identifier) @name) @item.trait
(impl_item trait: (_) @name type: (_) @name) @item.impl
(impl_item !trait type: (_) @name) @item.impl
(mod_item name: (identifier) @name) @item.module
(type_item name: (type_identifier) @name) @item.type
(associated_type name: (type_identifier) @name) @item.type
(const_item name: (identifier) @name) @item.const
(static_item name: (identifier) @name) @item.static
(macro_definition name: (identifier) @name) @item.macro
(enum_variant name: (identifier) @name) @item.variant
(field_declaration name: (field_identifier) @name) @item.field
(use_declaration argument: (_) @name) @binding.import
(source_file (macro_invocation macro: (_) @name "!" @name) @binding.call)
(declaration_list (macro_invocation macro: (_) @name "!" @name) @binding.call)
(closure_expression) @scope.function

(attribute_item) @lead
((line_comment) @lead (#match? @lead "^///"))
((block_comment) @lead (#match? @lead "^/\\*\\*"))

[(line_comment) (block_comment)] @token.comment
[(string_literal) (raw_string_literal) (char_literal)] @token.string
[(integer_literal) (float_literal)] @token.number
