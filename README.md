# 🌀 Warp

**A data format that is also a programming language, compiled straight to WebAssembly GC.**

```warp
contact{ name: James  age: 33 }
contact.age + 1                      // 34
```

The same notation holds your config, your markup and your program. Warp compiles it to WebAssembly GC structs
(no linear-memory runtime, no JS glue), and the compiler is itself a WebAssembly module.

▶ **[Try it in your browser](https://warp.pannous.com/)**: nothing to install. The compiler runs locally in a Web
Worker.

## Why Warp

- **Data is code.** Like Lisp, but with blocks, lists, key-value pairs and tags instead of s-expressions.
  JSON-like, but no quotes around keys or symbols. Reads and writes JSON, XML and YAML.
- **Wasm GC first.** A `class Person{name:String age:i64}` becomes a real `(struct …)` type, and the module carries
  full name sections. Rust hosts read these objects back as typed values (see below).
- **No silent footguns.** When code could mean two things, the compiler doesn't guess quietly. It warns with the
  reading it took and the explicit form, or it refuses when a wrong guess would corrupt results
  ([Footguns](https://github.com/pannous/warp/wiki/Footguns)):
  ```
  for i in 1 upto 4 { x += i }
  warning: does `upto 4` include 4? (`..<` or `..` exclude it, `to` or `...` include it) (taking exclusive); fix: ..<
  ```
- **Little ceremony.** Implicit `it`, inferred parameter and return types, Unicode operators, and English words for
  operators where they read better:
  ```warp
  square := it²
  square(3) + square(4)              // 25

  fib := if it < 2 then it else fib(it - 1) + fib(it - 2)
  fib(20)                            // 6765
  ```
- **Markup is just data.**
  ```warp
  html{ body{ h1: "Welcome"  p: "made of data" } }
  ```

## Install & run

```bash
git clone https://github.com/pannous/warp && cd warp
cargo build --release
target/release/warp samples/fibonacci.wasp   # run a file
target/release/warp eval "6*7"               # evaluate code
target/release/warp repl                     # interactive console
```

`samples/` contains classics (game of life, Dijkstra, Levenshtein, a JSON parser, a neural net) in both a
Python-like style and an idiomatic Warp style.

## Rust ⇄ Wasm GC round trip

```rust
let alice = Person { name: "Alice".into(), age: 30 };
is!("class Person{name:String age:i64}; Person{name:'Alice' age:30}", alice);
```
`is!` parses, compiles, runs the module in wasmtime and reads the GC object back. The compiler emits:
```wat
(type $String (struct (field $ptr i32) (field $len i32)))
(type $Person (struct (field $name (ref $String)) (field $age i64)))
(func $main (result (ref $Person))
  i32.const 0  i32.const 5  struct.new $String
  i64.const 30
  struct.new $Person)
(data (i32.const 0) "Alice")
```
Declare the Rust side with `wasm_struct! { Person { name: String, age: i64 } }`, or inline:
`wasm_object! { Person { name: String = "Alice", age: i64 = 30 } }`.

Untyped data travels as one universal GC type, `$Node {kind: i64, data: anyref, value: (ref null $Node)}`. It mirrors
the host enum `Node` (`Text`, `Symbol`, `Number`, `Key`, `List`, `Type`, `Meta`, `Data`) in
[src/node.rs](src/node.rs).

## Status

Experimental and moving fast. 2036 tests pass (`cargo test`). The language spec lives in the
[wiki](https://github.com/pannous/warp/wiki), and parts of it are not implemented yet. Warp is a Rust rewrite of
[Wasp](https://github.com/pannous/wasp) (C++), which follows the Python experiments
[Angle](https://github.com/pannous/angle) and [english-script](https://github.com/pannous/english-script).

Feedback, issues and especially "this surprised me" reports are welcome.
