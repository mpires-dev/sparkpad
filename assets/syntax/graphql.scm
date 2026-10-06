; Sparkpad queries for joowani/tree-sitter-graphql 0.1.0.
(name) @variable
(field (name) @property)
(named_type (name) @type)
(variable (name) @variable.special)
(string_value) @string
[(int_value) (float_value)] @number
["true" "false"] @boolean
(null_value) @constant
(comment) @comment
["query" "mutation" "subscription" "fragment" "on" "schema" "type" "input" "enum" "scalar" "union" "interface" "implements" "extend" "directive" "repeatable"] @keyword
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
[":" "=" "!" "@" "$" "|"] @operator
