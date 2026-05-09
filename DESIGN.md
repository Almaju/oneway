# Oneway Language Design

**Version:** v0.2.0 — Draft

> *"There is ONE WAY to do everything."*

Oneway is a maximally opinionated, strict, functional-leaning systems language that compiles to Rust (Phase 1) and later LLVM IR (Phase 2). Every design decision eliminates choice. If there's a best practice, it's the *only* practice. Comments are banned — the code IS the documentation.

---

## Table of Contents

- [Philosophy](#philosophy)
- [Naming Convention](#naming-convention)
- [Core Design Principles](#core-design-principles)
  - [1. Everything Sorted](#1-everything-sorted-alphabetical-compiler-enforced)
  - [2. Type-Derived Variable Names (No `let`)](#2-type-derived-variable-names-no-let)
  - [3. Functions Only — No Methods (UFCS)](#3-functions-only--no-methods-ufcs)
  - [4. Two Parameters Maximum](#4-two-parameters-maximum-receiver--input)
  - [5. Newtype Enforcement](#5-newtype-enforcement)
  - [6. Contracts](#6-contracts-replaces-traitsgenericsdyn)
  - [7. Match Is the Only Control Flow](#7-match-is-the-only-control-flow)
  - [8. Immutable by Default](#8-immutable-by-default)
  - [9. Effect Type System](#9-effecta-e-r-type-system)
  - [10. No Comments](#10-no-comments)
  - [11. No Lifetime Annotations](#11-no-lifetime-annotations)
  - [12. No Async Coloring](#12-no-async-coloring)
  - [13. Composition via Delegation](#13-composition-via-delegation)
- [Syntax Specification](#syntax-specification)
  - [No Comments](#no-comments)
  - [Imports](#imports)
  - [Newtypes](#newtypes)
  - [Struct Definitions](#struct-definitions)
  - [Enum Definitions](#enum-definitions)
  - [Function Definitions](#function-definitions)
  - [Contracts](#contracts)
  - [Match Expressions](#match-expressions)
  - [String Interpolation](#string-interpolation)
  - [Chaining (UFCS)](#chaining-ufcs)
- [Built-in Types](#built-in-types)
- [Error Handling](#error-handling)
- [Module System](#module-system)
- [Formatting](#formatting)
- [The "One Way" Decision Table](#the-one-way-decision-table)
- [Compilation](#compilation)
  - [Phase 1: Oneway → Rust](#phase-1-oneway--rust-transpilation)
  - [Phase 2: Oneway → LLVM IR](#phase-2-oneway--llvm-ir)
- [Open Design Questions](#open-design-questions)

---

## Philosophy

Most languages give you ten ways to do something and leave you to argue about which is best. Oneway gives you one way and lets you get on with your work.

- **One syntax** for each concept. No aliases, no shortcuts, no alternatives.
- **Compiler-enforced style.** No linter debates. No style guides. The compiler *is* the style guide.
- **Functional-leaning, systems-capable.** Immutable by default, expression-based, but compiles down to efficient native code.
- **Transparent effects.** Dependencies and errors are tracked in the type system, not hidden behind globals or frameworks.

---

## Naming Convention

There is ONE WAY to name things. The compiler enforces this — no exceptions.

| Convention | Used For | Examples |
|------------|----------|----------|
| **PascalCase** | Types, Contracts, Enums, Enum Variants, Newtypes | `Person`, `TaskId`, `HttpClient` |
| **camelCase** | Functions, variables, struct fields | `findByName`, `addBalance`, `firstName` |

No `snake_case`. No `SCREAMING_CASE`. No `kebab-case`. One convention per category, enforced at compile time.

---

## Core Design Principles

### 1. Everything Sorted (Alphabetical, Compiler-Enforced)

Struct fields, enum variants, function definitions in a module, use imports, match arms, contract functions — **ALL** sorted alphabetically. The compiler rejects unsorted code.

**Module-level ordering:**

1. `use` imports (sorted by path)
2. Type definitions (contracts, enums, newtypes, structs — mixed together, sorted by name)
3. Function definitions (sorted by name)

**Match arms:** sorted by pattern text. The `_` wildcard is always last.

**Why?** Eliminates bikeshedding over organization. Makes every file instantly navigable. Diffs are minimal when adding new items.

---

### 2. Type-Derived Variable Names (No `let`)

There is no `let` keyword. Variable bindings are automatically derived from the type name by lowercasing the first character:

| Expression | Auto-derived binding |
|------------|---------------------|
| `Person { ... }` | `person` |
| `HttpClient { ... }` | `httpClient` |
| `TaskId(42)` | `taskId` |

Custom names are allowed **only** when disambiguation is needed:

```
admin = Person { Age(30), Name("Admin") }
guest = Person { Age(25), Name("Guest") }
```

**Compiler rule:** If only one value of a given type is in scope, you **MUST** use the auto-derived name. Custom names are an error when they aren't needed.

---

### 3. Functions Only — No Methods (UFCS)

There are no methods. No `impl` blocks. Only standalone functions.

**Uniform Function Call Syntax (UFCS)** bridges the gap:

| Dot syntax | Equivalent to |
|------------|---------------|
| `value.function()` | `function(value)` |
| `value.function(arg)` | `function(value, arg)` |

**Chaining** works naturally:

```
value.f().g().h()
```

is equivalent to:

```
h(g(f(value)))
```

The dot syntax is just sugar. There is no dispatch, no vtables, no method resolution order. Functions are functions.

---

### 4. Two Parameters Maximum (Receiver + Input)

Functions take **at most TWO** parameters:

- **Param 1** (receiver): what the function acts ON
- **Param 2** (input): the data being passed

| Params | Definition | Call Styles |
|--------|-----------|-------------|
| 0 | `fn now() -> Timestamp` | `now()` |
| 1 | `fn double(Int) -> Int` | `5.double()` or `double(5)` |
| 2 | `fn addBalance(Wallet, Amount) -> Wallet` | `wallet.addBalance(amount)` |

Both parameters are auto-bound by their type name (camelCase).

**Destructuring rule:** If the type is a struct, the **first** parameter (receiver) is auto-destructured — its fields are directly in scope. The second parameter is bound as a variable only.

This enables fluent, readable APIs:

```
findByName(Name("Alan"))
  .assertIsAdmin()?
  .getWallet()
  .addBalance(Amount(100))
  .getBalance()
  .print()
```

**Why two?** Most well-designed functions naturally fit this model: a subject and an action, or a container and an element. If you need more data, group it into a struct.

---

### 5. Newtype Enforcement

Raw primitives (`Int`, `String`, `Float`, `Bool`) should **NOT** appear in function signatures or struct fields. Use newtypes:

```
type Age = Int
type Balance = Int
type Name = String
type TaskId = Int
```

`type X = Y` creates a **distinct type** (not an alias). They are not interchangeable.

| Operation | Syntax |
|-----------|--------|
| Construction | `TaskId(42)` |
| Access inner value | `taskId.value` |

The compiler **warns** when raw primitives are used in struct fields or function parameters. Low-level utility functions are exempt.

**Why?** `fn transfer(Int, Int, Int)` is a bug waiting to happen. `fn transfer(Account, Account, Amount)` is self-documenting and type-safe.

---

### 6. Contracts (Replaces Traits/dyn)

One abstraction mechanism: **contracts**. Structural typing (like Go interfaces).

```
contract Printable {
  fn toString(Self) -> String,
}
```

Types satisfy contracts **structurally** — if a matching function exists in scope, the type satisfies the contract. No explicit `impl` required.

**No orphan rule.** You can implement contracts on any type from anywhere. If conflicting implementations exist, the compiler errors at the use site, not at the definition site.

---

### 7. Match Is the Only Control Flow

There is no `if`, `else`, `for`, `while`, `loop`, or `return`.

**Value-based matching** (pattern matching):

```
match shape {
  Shape.Circle(radius) => "Circle with radius {radius.value}",
  Shape.Rectangle(data) => "Rectangle",
  Shape.Triangle(data) => "Triangle",
}
```

**Condition-based matching** (replaces if/else):

```
match {
  int < 0 => "negative",
  int == 0 => "zero",
  _ => "positive",
}
```

**Iteration:** `.map()`, `.filter()`, `.fold()` + recursion (with guaranteed tail-call optimization).

**Returns:** The last expression in a block is the return value. No `return` keyword.

---

### 8. Immutable by Default

All bindings are immutable. The `mut` keyword opts into mutation when necessary:

```
mut counter = Counter(0)
```

Mutation is available but discouraged. The functional style — transforming values and returning new ones — is the idiomatic path.

---

### 9. Effect<A, E, R> Type System

The unified type for computations that interact with the world:

- **A** = Success type
- **E** = Error union (inferred from `?` usage)
- **R** = Requirements / Dependencies (inferred from what services are used)

**Pure functions** — no errors, no dependencies:

```
fn double(Int) -> Int {
  int * 2
}
```

**Fallible functions** — can fail, no dependencies:

```
fn parse(String) -> Result<Int> {
  ...
}
```

**Effectful functions** — can fail AND have dependencies:

```
fn findUser(UserId) -> Effect<User> {
  logger.info("Finding user {userId.value}")
  database.query(userId)?
}
```

**Signature hierarchy:**

| Signature | Meaning |
|-----------|---------|
| `-> Int` | Pure. Cannot fail. No dependencies. |
| `-> Result<User>` | Can fail. No dependencies. E inferred. |
| `-> Effect<User>` | Can fail. Has dependencies. E + R inferred. |
| `-> Effect<User, QueryError, Database>` | Fully explicit. |

**Requirement auto-binding:** If `Database` is in R, then `database` is automatically in scope as a variable. No manual wiring.

**Providing dependencies:**

```
fn main() {
  findUser(UserId(42))
    .provide(PostgresDatabase { Url("postgres://...") })
    .provide(ConsoleLogger {})
    .run()
}
```

`.run()` is only available when **all** requirements are satisfied (R is empty). The compiler guarantees at compile time that nothing is missing.

---

### 10. No Comments

**Comments are not allowed.** The compiler rejects `//`. If code needs explaining, refactor it. The code IS the documentation. Function and type names should be descriptive enough.

There are no line comments, no block comments, no doc-comment variants. If you feel the need to write a comment, that's a signal to:

- Rename the function or type to be more descriptive
- Extract a well-named helper function
- Use more precise newtypes

---

### 11. No Lifetime Annotations

No `'a`, `'static`, `&'a`. Everything is owned by default. The compiler infers borrowing and cloning when generating Rust code. If it can't determine the best strategy, it clones — emitting a **warning** (not an error) so you can optimize later.

---

### 12. No Async Coloring

No `async`, `await`, or `Future`. All functions look synchronous. The runtime uses green threads (like Go). I/O operations automatically yield to the scheduler.

**Why?** Async coloring splits every ecosystem in two. In Oneway, there's one world. A function that reads from disk looks exactly like a function that adds two numbers — the runtime handles the rest.

---

### 13. Composition via Delegation

No inheritance. No mixins. Use the `delegates` keyword for struct embedding:

```
struct Admin {
  delegates User,
  List<Permission>,
}
```

`Admin` automatically satisfies all contracts that `User` satisfies. `User`'s fields are accessible directly through the `Admin` value: `admin.name`, `admin.email`, etc.

---

## Syntax Specification

### No Comments

Comments are not allowed. The compiler rejects `//`. There are no line comments, no block comments, no doc-comment variants. The code is the documentation.

---

### Imports

```
use io
use math
use net.http
```

Sorted alphabetically. Always at the top of the file.

---

### Newtypes

```
type Balance = Int
type Name = String
type TaskId = Int
```

Sorted alphabetically alongside other type definitions.

---

### Struct Definitions

```
struct User {
  Age,
  Email,
  Name,
}
```

Fields are types, sorted alphabetically by type name. No field names, no colons. Trailing comma on every field. Field access uses camelCase derived from the type: `user.age`, `user.email`, `user.name`.

With delegation:

```
struct Admin {
  delegates User,
  List<Permission>,
}
```

---

### Enum Definitions

```
enum Color {
  Blue,
  Green,
  Red,
}

enum Shape {
  Circle(Radius),
  Rectangle(RectangleData),
  Triangle(TriangleData),
}
```

Variants sorted alphabetically. PascalCase variant names. Trailing comma.

---

### Function Definitions

```
fn now() -> Timestamp {
  ...
}

fn double(Int) -> Int {
  int * 2
}

fn addBalance(Wallet, Amount) -> Wallet {
  Wallet { Balance(wallet.balance.value + amount.value), wallet.name }
}
```

Functions sorted alphabetically within their module. camelCase names.

---

### Contracts

```
contract Comparable {
  fn compare(Self, Self) -> Int,
  fn equal(Self, Self) -> Bool,
}

contract Printable {
  fn toString(Self) -> String,
}
```

Contract function signatures sorted alphabetically. Trailing comma.

---

### Match Expressions

**Value-based** (pattern matching on a subject):

```
fn describe(Shape) -> String {
  match shape {
    Shape.Circle(radius) => "Circle with radius {radius.value}",
    Shape.Rectangle(data) => "Rectangle",
    Shape.Triangle(data) => "Triangle",
  }
}
```

**Condition-based** (replaces if/else — no subject):

```
fn category(Int) -> String {
  match {
    int < 0 => "negative",
    int == 0 => "zero",
    _ => "positive",
  }
}
```

**Rules:**
- Arms sorted by pattern text.
- `_` wildcard always last.
- Must be exhaustive.

---

### String Interpolation

`"{expr}"` is the ONE way to format strings:

```
"Hello, {name.value}!"
"{a} + {b} = {result}"
```

No `format!`, no `println!` macros, no string concatenation operators.

---

### Chaining (UFCS)

```
findByName(Name("Alan"))
  .assertIsAdmin()?
  .getWallet()
  .addBalance(Amount(100))
  .getBalance()
  .print()
```

Every line reads as a verb acting on the result of the previous line.

---

## Built-in Types

| Type | Description |
|------|-------------|
| `Int` | 64-bit signed integer |
| `Float` | 64-bit float |
| `Bool` | `true` / `false` |
| `String` | UTF-8 string (ONE string type) |
| `List<T>` | Ordered collection |
| `Map<K, V>` | Key-value collection |
| `Set<T>` | Unique elements |
| `Option<T>` | Presence or absence |
| `Result<T, E>` | Success or failure (E inferred if omitted) |
| `Effect<A, E, R>` | Effectful computation |

No `i8`, `i16`, `u32`, `f32`, etc. One integer type. One float type. One string type.

---

## Error Handling

Error handling is Result-based. The `?` operator propagates errors. Error types are auto-unioned by the compiler:

```
fn findUser(UserId) -> Result<User> {
  database.connect()?.query(userId)?.parseUser()
}
```

**No panics. No unwrap. No exceptions.** If something can fail, it returns a `Result`. The type system tracks every possible failure mode.

---

## Module System

File = Module. Directory = Namespace. No `mod` declarations needed.

```
src/
  math/
    arithmetic.ow  → math.arithmetic
    geometry.ow    → math.geometry
  main.ow          → entry point
```

**Visibility:** Private by default. Use `pub` to export:

```
pub fn add(Int, Int) -> Int {
  ...
}
```

---

## Formatting

`ow fmt` is THE formatter. There is no configuration. It enforces:

- Alphabetical ordering of all sortable constructs
- 2-space indentation
- Trailing commas on all list-like items
- camelCase for functions and fields
- PascalCase for types
- One expression per line

There are no formatter options. No `.editorconfig` overrides. No "but my team prefers..." — the formatter decides, and that's the end of it.

---

## The "One Way" Decision Table

| Decision | The ONE Way | Eliminated |
|----------|-------------|------------|
| Comments | No comments allowed | `//`, `/* */`, `#`, `--` |
| Strings | `"double quotes"` | `'single'`, backticks |
| String formatting | `"{expr}"` interpolation | format!, println!, concat |
| Number types | `Int` and `Float` | i8/i16/i32/u8/u16/u32/f32/f64 |
| String types | `String` | &str, OsString, CStr |
| Abstraction | Contracts (structural) | Traits, interfaces, dyn |
| Generics | `<T>` angle brackets | `[T]` square brackets |
| Struct fields | By type (no keys) | `key: Type`, named fields |
| Control flow | `match` | if/else, for, while, loop, switch |
| Iteration | Functional (`.map` / `.filter` / `.fold`) | for loops, while loops |
| Callables | Functions (max 2 params) | Methods, closures, lambdas |
| Variables | Type-derived binding | let, var, const, auto |
| Mutability | Immutable default + `mut` | const/let, final/var |
| Returns | Last expression | return keyword |
| Error handling | `Result` + `?` + union types | Exceptions, panics, try/catch |
| Dependencies | `Effect<A, E, R>` requirements | DI frameworks, globals, param passing |
| Naming | PascalCase types, camelCase rest | snake_case, SCREAMING_CASE |
| Separators | Newlines | Semicolons |
| Struct creation | Literal `Type { ... }` | new(), builder, default() |
| Visibility | Private default + `pub` | public, protected, internal |
| Lifetimes | None (compiler-inferred) | 'a, 'static, &'a |
| Async | None (green threads) | async/await, Future |
| Code reuse | Composition + `delegates` | Inheritance, mixins |
| Orphan rule | None | Orphan rule, newtype workaround |
| Memory | Implicit ownership | Manual Rc/Arc, lifetime annotations |
| Primitives | Newtypes encouraged | Raw Int/String in API signatures |

---

## Compilation

### Phase 1: Oneway → Rust (Transpilation)

```
Source (.ow) → Lexer → Parser → AST → Checker → Rust Codegen → rustc → Binary
```

The Rust backend lets Oneway inherit Rust's optimizer, borrow checker, and ecosystem from day one. Ownership and borrowing decisions are made by the Oneway compiler when generating Rust code.

### Phase 2: Oneway → LLVM IR

```
Source (.ow) → Lexer → Parser → AST → Checker → LLVM IR → Native Binary
```

Direct compilation to LLVM IR for full control over code generation and to remove the Rust dependency.

---

## Open Design Questions

### 1. Higher-Order Functions with the 2-Param Rule

How does `.map(double)` work? The receiver is the list, the input is the function:

```
list.map(double)   // = map(list, double)
```

This fits the 2-param model naturally. **Solved.**

### 2. Closures / Anonymous Functions

Allow `{ x => x * 2 }` as anonymous function syntax? Useful for `.map` / `.filter` / `.fold`:

```
list.map({ x => x * 2 })
```

Probably yes — one syntax for anonymous functions. Details TBD.

### 3. Generic Type Definitions

Angle bracket syntax for type parameters:

```
struct Pair<A, B> {
  A,
  B,
}
```

### 4. Pattern Matching Sort Order for Numbers

Sort as text (lexicographic): `0` < `1` < `10` < `2` < `_`. Simple and unambiguous.

### 5. Standard Library Scope

Always available (no import needed): `Int`, `Float`, `Bool`, `String`, `List`, `Map`, `Set`, `Option`, `Result`, `Effect`, `print()`.

Everything else requires a `use` import.

### 6. FFI (Foreign Function Interface)

`extern "C" { ... }` blocks with size-specific types for interop with C libraries.

---

*This is a living document. Updated as the language evolves.*