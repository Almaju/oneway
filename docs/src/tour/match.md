# Match

There is no `if`/`else`. All branching is `match` on a union.

## Basic Form

```oneway
Bool = False | True

main = (Stdout) -> Noop {
    match True {
        False => "no".print(Stdout),
        True  => "yes".print(Stdout),
    }
}
```

`match` is an expression. It can be the final line of a function body or
appear as a sub-expression.

## Arm Order

Match arms follow the union's variant order — which is itself
alphabetical. There is no `_` wildcard in the spec; every variant must be
spelled out:

```oneway
Ord = Equal | Greater | Less

Int.classify = () -> Sign {
    match Int.compare(Int(0)) {
        Equal   => Zero,
        Greater => Positive,
        Less    => Negative,
    }
}
```

## Matching Constructors with Payloads

For union variants that carry a payload, bind it with parentheses. Use
`_` inside the parens to ignore the payload:

```oneway
match List(7, 8, 9).first() {
    None    => "empty".print(Stdout),
    Some(_) => "non-empty".print(Stdout),
}
```

## Why No `if`?

`if cond then a else b` is a `match` on `Bool`. Since you already need
`match` for unions in general, a second branching construct would just be
another way to do the same thing. So there is one.
