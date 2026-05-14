# Errors

Errors are values, carried by the standard `Result<T, E>` type. The error
slot is a regular type, so it can be a union written inline:

```oneway
File.read = (Path) -> Result<Bytes, IoError | NotFound | PermissionDenied> {
    ...
}
```

This is more ergonomic than Rust's approach, where each call site
typically needs a dedicated error enum.

## The `?` Operator

The postfix `?` operator propagates failure. It works on both
`Result<T, E>` and `Option<T>`:

- On `Result<T, E>`: short-circuits with the error, otherwise unwraps to
  `T`.
- On `Option<T>`: short-circuits with `None`, otherwise unwraps to `T`.

```oneway
main = (Stdout) -> Result<Noop, Noop> {
    Ok(42)?.print(Stdout)
    match Some(7) {
        None    => "absent".print(Stdout),
        Some(_) => "present".print(Stdout),
    }
    Ok(Noop)
}
```

`Ok(42)?` evaluates to `42` (because the `Result` is `Ok`); if it were
`Err(_)`, the function would return early with that error.

## Option vs Result

`Option<T>` and `Result<T, Empty>` are structurally similar but **kept
distinct**:

- `None` means *absent*.
- `Err(_)` means *failed*.

The semantic difference is worth the duplication. Use `Option` when a
value can legitimately be missing; use `Result` when an operation can
legitimately fail.

## Chaining

Because `?` is postfix, error-propagating pipelines read top-down,
left-to-right:

```oneway
File.readConfig = (Path) -> Result<Config, IoError | ParseError> {
    File.read(Path)?
        .parse()?
        .validate()
}
```

Each `?` unwraps the success case and lets the chain continue; the first
failure short-circuits the whole function.
