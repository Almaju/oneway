; Function definitions
(function_declaration
  name: (identifier) @function)

; Type identifiers (PascalCase)
(type_identifier) @type

; Literals
(integer_literal) @number
(float_literal) @number
(string_literal) @string
(boolean_literal) @constant.builtin

; String interpolation
(interpolation) @embedded

; Identifiers
(identifier) @variable

; Keywords
[
  "fn"
  "struct"
  "enum"
  "contract"
  "type"
  "use"
  "pub"
  "match"
  "delegates"
] @keyword

"Self" @type.builtin
"true" @constant.builtin
"false" @constant.builtin

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "%"
  "="
  "=="
  "!="
  ">"
  "<"
  ">="
  "<="
  "&&"
  "||"
  "!"
  "->"
  "=>"
  "?"
  "|"
] @operator

; Punctuation
[
  "{"
  "}"
  "("
  ")"
] @punctuation.bracket

[
  ","
  "."
] @punctuation.delimiter

; Function calls
(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (dot_expression
    field: (identifier) @function.method))

; Match
(match_arm
  pattern: (identifier) @variable)
