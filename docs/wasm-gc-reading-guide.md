# WebAssembly GC Object Reading Guide

## Overview

This guide documents patterns for reading WebAssembly GC object properties from Rust, based on code from `~/dev/script/rust/rasm` that demonstrates ergonomic WASM GC introspection.

## Key Patterns from rasm

### 1. Loading WAT with GC Types

```rust
use wasmtime::{Config, Engine, Module, Store};

let mut config = Config::new();
config.wasm_gc(true);  // Requires wasmtime 28.0+
config.wasm_function_references(true);

let engine = Engine::new(&config)?;
let mut store = Store::new(&engine, ());

let wat_source = std::fs::read_to_string("gc_types.wat")?;
let wasm_bytes = wat::parse_str(&wat_source)?;
let module = Module::new(&engine, wasm_bytes)?;
```

**From:** `rasm/src/wasm_helpers.rs:23-41`

### 2. GC Struct Introspection

```rust
// After calling a WASM function that returns a GC struct
let results = vec![Val::I32(0)];
create_person.call(&mut store, &params, &mut results)?;
let person_val = results[0].clone();

// Access the struct fields directly
let anyref = person_val.unwrap_anyref()?;
let struct_ref = anyref.unwrap_struct(&store)?;

// Read fields by index
let name_val = struct_ref.field(&mut store, 0)?;
let age_val = struct_ref.field(&mut store, 1)?;
```

**From:** `rasm/src/gc_struct_demo.rs:14`

### 3. Ergonomic GcObject Wrapper

The core pattern uses interior mutability to hide store management:

```rust
pub struct GcObject<T> {
    inner: Rooted<StructRef>,
    store: T,  // Rc<RefCell<Store<()>>>
    instance: Option<Instance>,
}

impl GcObject<Rc<RefCell<Store<()>>>> {
    pub fn get<T: FromVal>(&self, field: &str) -> Result<T> {
        let mut store = self.store.borrow_mut();
        let struct_ref = self.inner.to_ref(&store);
        let index = self.field_name_to_index(field, &struct_ref, &store)?;
        let val = struct_ref.field(&mut store, index)?;
        T::from_val(val, &mut store, self.instance.as_ref())
    }
}
```

**Concept from:** `rasm/BLOG.md:207-212`

### 4. Type-Safe Wrappers with gc_struct! Macro

```rust
gc_struct! {
    Person {
        name: 0 => mut String,
        age: 1 => mut i32,
        email: 2 => String,
    }
}

// Generated methods:
// - person.name() -> Result<String>
// - person.age() -> Result<i32>
// - person.set_name(&str) -> Result<()>
// - person.set_age(i32) -> Result<()>
```

**From:** `rasm/src/gc_object_demo.rs:8-13`

### 5. Creating GC Objects from Rust

```rust
// Bootstrap: Create one object via WASM
let template = Person::from_val(wasm_created_val, store, instance)?;

// Then create more objects using extracted type info
let diana = Person::create(
    &template,
    obj! {
        name: "Diana 🚀",
        age: 29,
        email: "diana@example.com",
    },
)?;
```

**From:** `rasm/src/gc_object_demo.rs:103-111`

### 6. Field Access by Name

Instead of numeric indices, use field names:

```rust
impl FieldIndex for &str {
    fn to_field_index(&self, struct_ref: &StructRef, store: &Store) -> Result<usize> {
        // Map field names to indices
        // In rasm, this uses WASM name section introspection
        match *self {
            "name" => Ok(0),
            "age" => Ok(1),
            "email" => Ok(2),
            _ => Err(anyhow!("unknown field: {}", self)),
        }
    }
}

// Then use:
let name: String = person.get("name")?;
let age: i32 = person.get("age")?;
```

**Concept from:** `rasm/BLOG.md:99-115`

## Application to WASP Nodes

### WAT Type Definition

```wat
(type $node (struct
  (field $tag i32)              ;; NodeTag discriminant
  (field $int_value i64)        ;; For Number::Int
  (field $float_value f64)      ;; For Number::Float
  (field $text (ref $string))   ;; For Text/Symbol
  (field $left (ref null $node)) ;; For Pair/Block left child
  (field $right (ref null $node)) ;; For Pair/Block right child
))

(type $string (array (mut i8)))  ;; GC string type
```

### Rust Wrapper (Proposed)

```rust
gc_struct! {
    WaspNode {
        tag: 0 => i32,
        int_value: 1 => i64,
        float_value: 2 => f64,
        text: 3 => String,
        left: 4 => Option<WaspNode>,
        right: 5 => Option<WaspNode>,
    }
}

impl WaspNode {
    pub fn to_node(&self) -> Result<Node> {
        let tag = NodeTag::from(self.tag()?);
        match tag {
            NodeTag::Empty => Ok(Node::Empty),
            NodeTag::Number => {
                let int_val = self.int_value()?;
                Ok(Node::Number(Number::Int(int_val)))
            }
            NodeTag::Text => {
                let text = self.text()?;
                Ok(Node::Text(text))
            }
            NodeTag::Pair => {
                let left = self.left()?.ok_or(anyhow!("Missing left"))?;
                let right = self.right()?.ok_or(anyhow!("Missing right"))?;
                Ok(Node::Pair(
                    Box::new(left.to_node()?),
                    Box::new(right.to_node()?)
                ))
            }
            // ... other variants
        }
    }
}
```

## Benefits

1. **No Store Passing**: Store hidden in `Rc<RefCell<Store>>`
2. **Type Safety**: Compile-time checked field access
3. **IDE Support**: Auto-complete for field names
4. **Named Fields**: Use `person.name()` not `person.get(0)`
5. **Ergonomic Creation**: Object-literal syntax with `obj!` macro

## Requirements

- Wasmtime 28.0+ for full GC introspection
- GC proposal enabled in WASM module
- Function references proposal enabled

## References

- rasm project: `~/dev/script/rust/rasm`
- Key files:
  - `src/gc_object_demo.rs` - Full demo with all features
  - `src/gc_struct_demo.rs` - Minimal struct reading example
  - `src/wasm_helpers.rs` - Module loading utilities
  - `BLOG.md` - Detailed explanation of 7 abstraction levels

## Testing

See `tests/wasm_gc_read_test.rs` for examples adapted to wasp.

Run with:
```bash
cargo test --test wasm_gc_read_test
```
