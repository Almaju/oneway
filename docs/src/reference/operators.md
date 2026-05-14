# Operators

## Type-Level Precedence

Tightest first:

1. `T[N]` — postfix repetition
2. `...T` — prefix spread
3. `T<...>` — generic application
4. `&` — product
5. `|` — union

So `A | B & C[3]` parses as `A | (B & (C[3]))`.

## Expression-Level Precedence

Tightest first:

1. `.` — method call / field access
2. `()` — function application
3. `?` — postfix error propagation
4. `&` — value-level product (only inside a constructor argument)

So `foo.bar()?` is `((foo.bar)())?`.

## Glossary of Operators and Sigils

| Symbol     | Meaning                                  |
|------------|------------------------------------------|
| `\|`       | Union                                    |
| `&`        | Product                                  |
| `Type[N]`  | Fixed repetition (N copies)              |
| `...Type`  | Unbounded repetition                     |
| `<T>`      | Generic parameter                        |
| `<T: Tr>`  | Generic with trait constraint            |
| `.`        | Method call / field access               |
| `?`        | Propagate `Result` / `Option` failure    |
| `*name`    | Private method (file-local)              |
| `"..."`    | String literal sugar                     |
| `mut`      | Mutable parameter                        |
