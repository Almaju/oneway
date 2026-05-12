# `oneway::type_derived_naming`

**Severity:** deny
**Enforced by:** `oneway_lints` (dylint)

The implementation enforces this for function parameters and `let` bindings with an explicit type ascription. For inferred types, the rule is not enforced today — add `: Type` to opt the binding in.

**Exemptions:** primitives (`i32`, `bool`, `String`, …) and a handful of stdlib containers whose idiomatic short names are too entrenched to flag (`Option`, `Result`, `Vec`, `Box`, `Rc`, `Arc`, `HashMap`, `Path`, …). Use whatever name you like for those.

Every binding's name must be the `snake_case` version of its type. This applies to both `let` bindings and function parameters — wherever you give a value a name, that name should echo the type.

The reason: at every use site, the reader can map the variable back to its type without scrolling up to the declaration. It also eliminates bikeshedding ("`id` or `uid` or `user_id`?" — only one answer). When two bindings of the same type need to coexist, add a descriptive prefix.

## ❌ Bad — short, type-unrelated names

```rust
let id = UserId(42);
let db = Database::connect();
let u = User::find(id);
```

## ✅ Good

```rust
let user_id = UserId(42);
let database = Database::connect();
let user = User::find(user_id);
```

## ❌ Bad — function parameter doesn't echo its type

```rust
fn find_user(id: UserId, db: &Database) -> Option<User> {
    db.query(id)
}
```

## ✅ Good

```rust
fn find_user(user_id: UserId, database: &Database) -> Option<User> {
    database.query(user_id)
}
```

## ❌ Bad — two of the same type without disambiguation

```rust
let src = AccountId(1);
let dst = AccountId(2);
```

## ✅ Good

```rust
let sender_account_id = AccountId(1);
let receiver_account_id = AccountId(2);
```
