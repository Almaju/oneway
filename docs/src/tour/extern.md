# Extern Rust

Oneway does **not** ship its own application-level standard library.
Beyond a small core (numerics, `String`, `Option`, `Result`, `Bool`,
`Ord`, capabilities, `List<T>`, `Map<K, V>`), all functionality —
HTTP, JSON, databases, regex, async runtimes — comes from the host
language's ecosystem (Rust + crates.io), reached via `extern Rust`.

## Declaring an Extern

A type or method can be declared as backed by a Rust item. The transpiler
emits direct calls — no runtime glue, no marshalling.

```oneway
extern Rust("std::cmp::min")
Int.min = (Int) -> Int

main = (Stdout) -> Noop {
    5.min(3).print(Stdout)
}
```

The string argument is the fully qualified Rust path. The Oneway signature
declares how the method is called from Oneway.

## Method-Style Externs

A Rust path that begins with `.` indicates a method call on the receiver,
not a free function:

```oneway
extern Rust(".to_lowercase")
String.toLower = () -> String

extern Rust(".to_uppercase")
String.toUpper = () -> String

main = (Stdout) -> Noop {
    "Hello, World".toUpper().print(Stdout)
    "GoodBye".toLower().print(Stdout)
}
```

## Extern Types

Capabilities like `Stdout` and wrappers over Rust types like
`axum::Router` are declared the same way:

```oneway
extern Rust("std::io::stdout")
Stdout

extern Rust("axum::Router")
HttpRouter

extern Rust("axum::Router::route")
HttpRouter.route = (Handler & Path) -> HttpRouter
```

## Binding Packages

Idiomatic Oneway code does not call `extern Rust` directly throughout the
codebase. Instead, the community (and the standard library) publishes
**binding packages** — thin Oneway facades over popular Rust crates:

```oneway
use Http        # wraps axum / reqwest
use Json        # wraps serde_json
use Database    # wraps sqlx
```

A binding package is a few hundred lines of Oneway declarations plus
minimal ergonomic glue. The same pattern as `ktor` over `okhttp` in
Kotlin, or `cljs-http` over `fetch` in ClojureScript.

## Tradeoffs

- **Error messages may leak Rust types** when crossing the FFI boundary.
  Mitigated by good bindings, but unavoidable in full generality.
- **Async-flavored crates** are exposed only through blocking facades, to
  preserve the no-coloring rule. Performance-sensitive async work is the
  awkward case.
- **Oneway is permanently coupled to Rust** unless a second backend is
  later added. A real strategic dependency, accepted in exchange for never
  shipping a stdlib.
