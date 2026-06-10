## Grammar

```
expr           → assignment

assignment     → ternary ("=" expr)?

ternary        → pipe ("?" expr ":" expr)?

pipe           → nullish_coalescing ( "|>" nullish_coalescing )*

nullish_coalescing → logical_or ( "??" logical_or )*

logical_or     → logical_and ( "||" logical_and )*

logical_and    → bitwise_or ( "&&" bitwise_or )*

bitwise_or     → bitwise_xor ( "|" bitwise_xor )*

bitwise_xor    → bitwise_and ( "^" bitwise_and )*

bitwise_and    → equality ( "&" equality )*

equality       → relational ( ("===" | "==" | "!==" | "!=") relational )*

relational     → additive ( (">=" | "<=" | ">" | "<") additive )*

additive       → multiplicative ( ("+" | "-") multiplicative )*

multiplicative → unary ( ("*" | "/" | "%") unary )*

unary          → ("+" | "-" | "!") unary | postfix

postfix        → primary ( "." ident | "[" expr "]" | "(" comma_expr ")" )*

primary        → literal | ident | custom_ident | list | map | paren

literal        → "true" | "false" | "null" | "undefined" | number | string

ident          → identifier ("=>" expr)?

custom_ident   → "${" (any char except "}")* "}"

list           → "[" comma_expr "]"

map            → "{" comma_map "}"

map_entry      → (identifier | string) ":" expr

paren          → "(" comma_expr ")" ("=>" expr)?

comma_expr     → expr ("," expr)* ","?

comma_map      → map_entry ("," map_entry)* ","?

number         → float

string         → "'" char* "'" | '"' char* '"'
```

### Precedence (lowest to highest)

| Precedence | Operator(s)           | Associativity |
|------------|-----------------------|---------------|
| 1          | `=`                   | right         |
| 2          | `? :`                 | right         |
| 3          | `\|>`                 | left          |
| 4          | `??`                  | left          |
| 5          | `\|\|`                | left          |
| 6          | `&&`                  | left          |
| 7          | `\|`                  | left          |
| 8          | `^`                   | left          |
| 9          | `&`                   | left          |
| 10         | `===` `==` `!==` `!=` | left          |
| 11         | `>=` `<=` `>` `<`     | left          |
| 12         | `+` `-`               | left          |
| 13         | `*` `/` `%`           | left          |
| 14         | unary `+` `-` `!`     | right         |
| 15         | `.` `[]` `()`         | left          |
