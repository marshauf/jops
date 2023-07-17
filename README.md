# JSON Operators

A collection of tools to help operate on [serde_json::Values](https://docs.rs/serde_json/latest/serde_json/enum.Value.html).

## partial_cmp

JOPS provides a function for comparing two [serde_json::Value](https://docs.rs/serde_json/latest/serde_json/enum.Value.html).

### Examples

The partial_cmp function.

```rust
use std::cmp::Ordering;
use serde_json::Value;

let a = Value::Null;
let b = Value::Null;
let res : Ordering = jops::partial_cmp(&a, &b);
```

JsonValue wrapper provides partial_cmp.

```rust
use jops;
use serde_json::Value;

let a = jops::JsonValue(&Value::Null);
let b = jops::JsonValue(&Value::Null);
let res = a > b;
```

## JsonPath

An [SQLite JSON Path](https://www.sqlite.org/json1.html#jptr) implementation.
Access Values inside an Object or Array.

### Operators

* `$` represents the root Value
* `.<name>` points to a sub value with key `name` inside an Object
* `<name>[<index>]` points to a value inside an Array `name` at `index` (zero indexed).
* `<name>[#-<offset>]` points to a value inside an Array `name` at length of array minus offset.
* `<index>` points to a value inside a root Array at `index` (zero indexed).

### Examples

```rust
let value = serde_json::json!({ "a": "example", "b": [0,1,2] });
value.path("$"); // Returns a reference to value
value.path("$.a"); // Returns a reference to the String value "example"
value.path("$.b[1]"); // Returns a reference to the Number value 1 inside the array b
value.path("$.b[#-1]"); // Returns a reference to the Number value 1 inside the array b
let value = serde_json::json!([0,2]);
value.path("1"); // Returns a reference to the Number value 2 inside the array
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE)
or [MIT license](LICENSE-MIT) at your option.

