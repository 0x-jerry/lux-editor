; TypeScript highlights for Lux Editor.
; Adapted from the upstream tree-sitter-typescript queries (MIT) and the
; tree-sitter-javascript queries (MIT), restricted to node and token names
; that exist in BOTH the typescript and tsx grammars (the tsx-only JSX
; patterns live in typescript-tsx.scm).

; Variables
; ----------

(identifier) @variable
(property_identifier) @property
(shorthand_property_identifier) @property
(statement_identifier) @variable

; Parameters
; -----------

(required_parameter (identifier) @variable.parameter)
(optional_parameter (identifier) @variable.parameter)

; Comments and strings
; ---------------------

(comment) @comment

((comment) @comment.documentation
  (#match? @comment.documentation "^/\\*\\*"))

(string) @string
(string_fragment) @string
(escape_sequence) @string.escape
(template_string) @string
(regex) @string.special
(number) @number

; Constants and builtins
; -----------------------

[
  (true)
  (false)
  (null)
  (undefined)
] @constant.builtin

[
  (this)
  (super)
] @variable.builtin

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z\\d_]*$"))

; Types
; ------

(type_identifier) @type
(predefined_type) @type.builtin
((identifier) @type
  (#match? @type "^[A-Z]"))

(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; Functions and methods
; ----------------------

(function_signature
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)
(function_expression
  name: (identifier) @function)
(method_definition
  name: (property_identifier) @function.method)
(method_signature
  name: (property_identifier) @function.method)
(call_expression
  function: (identifier) @function)
(call_expression
  function: (member_expression
    property: (property_identifier) @function.method))
(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.method)
  right: [(arrow_function) (function_expression)])
(pair
  key: (property_identifier) @function.method
  value: [(arrow_function) (function_expression)])
(variable_declarator
  name: (identifier) @function
  value: [(arrow_function) (function_expression)])

(new_expression
  constructor: (identifier) @constructor)

(decorator) @attribute

; Punctuation
; ------------

[
  ";"
  ","
  "."
  (optional_chain)
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

[
  "=>"
  "..."
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "&&"
  "||"
  "??"
  "?"
  ":"
  "+"
  "-"
  "*"
  "/"
  "%"
  "**"
  "|"
  "&"
  "^"
  "~"
  "<"
  ">"
  "<="
  ">="
] @operator

; Keywords
; ---------

[
  "abstract"
  "as"
  "async"
  "await"
  "break"
  "case"
  "catch"
  "class"
  "const"
  "continue"
  "declare"
  "default"
  "delete"
  "do"
  "else"
  "enum"
  "export"
  "extends"
  "finally"
  "for"
  "from"
  "function"
  "if"
  "implements"
  "import"
  "in"
  "instanceof"
  "interface"
  "keyof"
  "let"
  "namespace"
  "new"
  "of"
  "private"
  "protected"
  "public"
  "readonly"
  "return"
  "satisfies"
  "static"
  "switch"
  "throw"
  "try"
  "type"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
] @keyword
