# Traits

A trait is a callable type signature. It is declared like a function type:

```oneway
Show = () -> String
```

Because traits are types, they are written in `PascalCase`. The case
difference is how the compiler distinguishes a trait implementation
(`Type.Print`) from a regular method (`Type.print`) on the same type.

## Implementing a Trait

A trait is implemented on a type by assigning to `Type.TraitName`:

```oneway
Show = () -> String

Greeting = String
Name     = String

Greeting.Show = () -> String {
    "HELLO!"
}

Name.Show = () -> String {
    "Alice"
}

main = (Stdout) -> Noop {
    Greeting("hi").Show().print(Stdout)
    Name("Alice").Show().print(Stdout)
}
```

`Greeting.Show()` and `Name.Show()` both have the same signature
(`() -> String`) and are called the same way.

## Multi-Method Traits

A trait with multiple methods is just a product of single-method traits:

```oneway
Show = Debug & PrintString
```

## Default Implementations

A trait declaration can carry a default body marked `{ impl }`:

```oneway
Greet = () -> String { impl }
```

Implementing types may then either override or inherit the default.

## Using a Trait as a Parameter

A trait can be used directly as a parameter type. The parameter binds the
trait implementation, which is then invocable:

```oneway
Type.needsPrint = (Print) -> Noop {
    Print()
}
```

## Generic Constraints

Constraints on generic parameters use `:`, naming a trait the parameter
must implement:

```oneway
List.print = <T: Print>() -> Noop {
    ...
}
```
