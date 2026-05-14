# Capabilities

A function's type should not lie about what it does. `String.print = () ->
Noop` claims "nothing happens", but writing to stdout is something.

Oneway models effects as **capabilities** — values that must be passed in
to perform an effect.

## The Pattern

A function that prints requires `Stdout`:

```oneway
String.print = (Stdout) -> Noop {
    Stdout.write(String)
}
```

A function that reads files requires `Filesystem`. A function that uses
the clock requires `Clock`. And so on. The capability is just a type,
passed as an ordinary argument.

## Where Capabilities Come From

The only place to obtain real-world capabilities is `main.ow`, which
receives them as parameters and threads them down to anything that needs
them:

```oneway
main = (Stdout) -> Noop {
    "hello".print(Stdout)
}
```

If a function does not receive a capability, it cannot perform the
corresponding effect — it cannot even *call* something that does. Effects
propagate through the type system: if `f` calls something needing
`Stdout`, then `f` must take `Stdout` too.

## Built-In Capabilities

The Oneway-owned core includes:

| Capability     | Effect                       |
|----------------|------------------------------|
| `Clock`        | Read the current time        |
| `Filesystem`   | Read and write files         |
| `Network`      | Open network connections     |
| `Random`       | Generate random values       |
| `Stderr`       | Write to standard error      |
| `Stdin`        | Read from standard input     |
| `Stdout`       | Write to standard output     |

## Why Not Monads?

A capability-passing model gives you the same honest type signatures as a
monadic effect system, without introducing a separate kind of value or
forcing all effectful code into a `do`-style block. Effects are just
arguments. Composition is just method calls.
