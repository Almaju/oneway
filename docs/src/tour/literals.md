# Literals

Oneway is values-only. There is no `new`, no implicit nullability, no
keywords like `true` or `false`. Every value is built by calling its
type's constructor.

## Constructors

Every type `T` has a constructor `T(_)`. The argument is a value matching
the type's underlying definition:

| Kind             | Constructor                              | Argument is…                                  |
|------------------|------------------------------------------|-----------------------------------------------|
| Primitive        | `Int(123)`, `Float(1.0)`, `String("hi")` | a literal of the corresponding lexical kind   |
| Hex              | `Hex(0xFF0000)`                          | a hex literal                                 |
| Product `A & B`  | `T(A(...) & B(...))`                     | a value-level product joined with `&`         |
| Union `A \| B`   | `T(A(...))` or `T(B(...))`               | a value of any variant                        |
| Newtype          | `T(inner)`                               | a value of the aliased type                   |

## Literal Sugar

A handful of literals desugar to their constructors:

| Literal     | Desugars to       |
|-------------|-------------------|
| `123`       | `Int(123)`        |
| `1.0`       | `Float(1.0)`      |
| `"abc"`     | `String("abc")`   |
| `0xFF0000`  | `Hex(0xFF0000)`   |

Numeric literals exist to avoid boilerplate in arithmetic-heavy code.
String literals exist to avoid the parsing ambiguity of bare `String(...)`
with spaces and punctuation.

## Singleton Values

A singleton type — one with no underlying composition — has one value,
referenced by writing the type name:

```oneway
Noop      // the sole value of type Noop
On        // the sole value of type On
```

## No Empty Constructors

`String()`, `Int()`, `User()` — calling any constructor with zero
arguments is a compile-time error. If a value can legitimately be
"missing", that absence belongs in the type as `Option<T>`. Otherwise the
type requires its data.

For factory-style construction (an empty list, etc.), use an explicit
method like `List.empty` or `String.empty`.

## Constructing a Product

The argument to a product's constructor is its components joined with
value-level `&`:

```oneway
user = User(Birthday(...) & Username("ahanot"))
red  = Hex(0xFF0000)
```

`&` is overloaded across the two levels: at the type level it forms a
product type, at the value level it forms a product value. The two never
appear in the same context.
