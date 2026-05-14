# Loops

Oneway has standard imperative loop constructs: `while` and `for`. Both
are expressions of type `Noop`.

## `while`

```oneway
Bool = False | True

main = (Stdout) -> Noop {
    while False {
        "looping".print(Stdout)
    }
    "done".print(Stdout)
}
```

The condition is an expression of type `Bool` — which is just the
two-variant union `False | True`. The body runs as long as the condition
evaluates to `True`.

## `for`

`for` iterates over a sequence. The exact iteration protocol is
implementation-defined and may change.

## Higher-Order Forms

For most collection work, prefer higher-order methods on the collection
itself — `map`, `fold`, `length`, `first`, and friends — rather than
explicit loops:

```oneway
List(10, 20, 30)
    .map((Int) -> Int { Int.mul(2) })
    .length()
    .print(Stdout)
```

A `while`/`for` loop is the right answer when you need a side effect on
each iteration; method chaining is the right answer when you are
transforming a value.
