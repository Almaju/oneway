# Philosophy

> *There is one way to do everything.*

Most modern languages give you ten ways to do the same thing and then ask
you to pick. Oneway picks for you. If there's a best practice, it's the
*only* practice — and the compiler enforces it.

## Alphabetical Order, Everywhere

The single most pervasive rule. Whenever ordering is discretionary,
declarations must be in alphabetical order. This applies to:

- Components of a product type: `User = Birthday & Username`
- Variants of a union type: `Bool = False | True`
- Multiple methods on a type (declared top-to-bottom alphabetically)
- Arms of a `match` (in the order of the union's variants)
- Trait composition: `Show = Debug & PrintString`
- Error unions inside `Result`: `Result<T, IoError | NotFound>`
- Multiple `use` statements at the top of a file

Reordering is never a meaningful change. Diffs that only reshuffle a list
do not exist. Two programmers writing the same code produce the same
bytes.

## Types Are the Documentation

Oneway has **no local variables, no `let`**, and **no parameter names**.
The shape of a function is described entirely by its types.

```oneway
User.compare = (OtherUser) -> Ord {
    User.Birthday.compare(OtherUser.Birthday)
}
```

The receiver is referred to as `User` (its type). The parameter is referred
to as `OtherUser` (its type). If you need to disambiguate two parameters of
the same type, you define a newtype — that newtype becomes the
documentation:

```oneway
User      = Birthday & Username
OtherUser = User
```

The principle: **names lie, types don't**. Forcing every value through a
named type makes the data flow explicit and the documentation structural.

## Effects Are Honest

A function's signature should not lie about what it does. `print` writes to
the screen, so it requires a `Stdout` capability — passed as an ordinary
argument from `main`:

```oneway
String.print = (Stdout) -> Noop {
    ...
}
```

A function that does not receive a capability cannot perform the
corresponding effect. No monads, no effect system — just types.

## No Comments

There are no comments. Code must speak for itself through types and naming.
If you find yourself wanting to write a comment, the right answer is
usually to introduce a newtype or rename a method.

## Small Core, Big Ecosystem

Oneway ships almost nothing of its own beyond the type system and a handful
of standard unions and containers. HTTP, JSON, databases, regex, logging —
everything else is the host language's ecosystem (Rust + crates.io),
reached via [`extern Rust`](./extern.md) declarations.
