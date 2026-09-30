## Project Overview

This is **warp**, a rust implementation of **wasp**

Always commit work and progress, even when a task is not fully solved or tests remain failing. Record unresolved failures honestly, and push when progress is at or near the desired goal.

https://github.com/pannous/wasp
https://wasp.pannous.com/

wasp is a data format and wasm first programming language
C++ source code locally at ~/wasp/ documentation and specification at ./wiki 
The specification is not fully implemented yet 

- The project parses a custom syntax, builds a Node-based AST, and emits WebAssembly modules using:
- WIT (WebAssembly Interface Types) definitions
- WASM GC (Garbage Collection) bytecode
- Multiple WASM runtime backends (wasmtime, wasmer, wasmedge)

## Core Architecture

Wasp Wisp and Warp are similar to lisp in that they are a data format from first principles,
but instead of s-expressions they use a more modern syntax with blocks, lists, key-value pairs, tags, comments, and rich atomic types.

```wasp
Person {
    name: "Alice"    
    age: 30             
    hobbies: [ "reading", "hiking", "coding" ]
}
```

It is similar to JSON5 / ECMA Script but much simplified yet with a richer syntax and more precise and flexible data model.


### Node AST (`src/node.rs`)

The central data structure is `Node`, an enum representing all AST node types:

- **Empty, Number, Text, Codepoint, Symbol** - Atomic values
- **Key** - Binary structures (Pair, Tag)
- **List/Block** - Collection of nodes with separator and grouping via `Grouper` (parentheses, brackets, braces)
- **Data** - Generic container using `Dada` for arbitrary Rust types with `CloneAny` trait
- **Meta** - Node wrapper that adds `Meta` (comments, line/column positions)

### Parser (`src/wasp_parser.rs`)

Recursive descent parser that converts text input to Node AST:

- Tracks position (line/column) for all nodes
- Handles comments (`//` line and `/* */` block) attached as metadata
- Parses literals (numbers, strings, symbols), groups ((), [], {}), and structures

### Emitters

Three distinct code generation backends:

0. **Generic Emitter** (`src/emitter.rs`)
    - Textual emitter similar to json5

1. **WIT Emitter** (`src/wit_emitter.rs`)
    - Generates WebAssembly Interface Type definitions
    - Outputs `.wit` files defining type shapes for Node variants

2. **WASM GC Emitter** (`src/wasm_gc_emitter.rs`)
    - Generates WASM GC bytecode using `wasm-encoder` crate
    - Creates GC struct types for each Node variant with proper tagging
    - Uses `NodeKind` enum for runtime type discrimination

### WASM Runtime Support (`src/run/`)

Multiple runtime backends for executing generated WASM:

- `wasmtime_runner.rs` - Primary runtime
- `wasmedge_runner.rs` - Alternative
- `wasmer_runner.rs` - Alternative

### Compiler Utilities (`src/compiler/`)

- `wasm_reader.rs` - Reads WASM modules using wasmparser
- `parity_wasm_reader.rs` - Alternative reader using parity-wasm

## Build and Test Commands

### Building

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --offline          # Offline mode (uses vendored dependencies)
```

The project uses vendored dependencies (see `vendor/`) to support offline builds. The `Cargo.toml` warns against online
compilation delays.

### Testing

```bash
cargo test                     # Run all tests
cargo test <test_name>         # Run specific test by name
cargo test --test <test_file>  # Run specific test file (without _test.rs suffix)
```

#### Important Test Files

- `tests/node_test.rs` - Tests Node AST operations
- `tests/wasp_parser_test.rs` - Tests parser functionality
- `tests/wasm_gc_emitter_test.rs` - Tests WASM GC code generation
- `tests/wasm_reader_test.rs` - Tests reading WASM GC objects (see guide below)

### Running Examples

```bash
cargo run --example wasm_gc_generation
cargo run --example wit_generation
cargo run --example wasp_comments_demo
```

## WASM GC Reading Patterns

The project follows patterns from `~/dev/script/rust/rasm` for ergonomic WASM GC object introspection. See
`docs/wasm-gc-reading-guide.md` for:

- Loading WAT modules with GC types enabled
- Reading GC struct fields by index
- Type-safe wrappers with `gc_struct!` macro
- Ergonomic `GcObject` wrapper hiding store management
- Creating GC objects from Rust

## Important Notes

Use WASM names excessively! Wasm provides custom sections for names, use ALL of them!

### Offline Development

The project is configured for **offline-first** development to avoid compilation delays. Dependencies are vendored and
Cargo.toml has offline mode notes. Use `--offline` flag when building.

### Test File Locations

Tests are in `tests/` directory (not `src/`). Each test file is named `*_test.rs` and tests a specific module or
feature.

### Extension Utilities

The `src/extensions/` directory provides Rust standard library extensions:

- `numbers.rs` - Extended number types (Complex, Quotient, etc.)
- `strings.rs` - String manipulation helpers
- `lists.rs` - Collection utilities
- `utils.rs` - General utilities (download, file I/O)
- more on demand

These are reexported in `lib.rs` for test access via `use warp::*`.

## Development Workflow

0. **Run tests** - `cargo test` to verify we start from a clean state, check git logs
1. **Modify parser or emitter** - Edit files in `src/`
2. **Add tests** - Create or update tests in `tests/`
3. **Run tests** - `cargo test` to verify
4. **Check examples** - Run examples to see output
5. **Build offline** - Use `--offline` for reproducible builds

## Serialization

=== Node Serialization Workflow (Current Spec) ===

Current spec lives in `src/wasm_emitter/type_manager.rs` and `src/wasm_emitter/constructors.rs`.

1. Core GC types emitted to WASM:

   (type $String (struct
     (field $ptr i32)
     (field $len i32)))

   (type $Node (struct
     (field $kind i64)            ;; Kind tag + extra info in high bits
     (field $data anyref)         ;; payload (boxed number, string, i31ref, or node)
     (field $value (ref null $Node)))) ;; secondary payload (node or null)

   (type $i64box (struct (field $value i64)))
   (type $f64box (struct (field $value f64)))

2. Kind encoding (see `src/type_kinds.rs`):

   - Lower 8 bits are `Kind` (Empty=0, Int=1, Float=2, Text=3, Codepoint=4, Symbol=5, Key=6, Block=7, List=8, Data=9, Meta=10, Error=11, TypeDef=12, ...)
   - For Key and List, extra info is stored in the high bits:
     - key: `kind = (op_info << 8) | Kind::Key`
     - list: `kind = (bracket_info << 8) | Kind::List`
       bracket_info: Curly=0, Square=1, Round=2, Less=3, Other=4, None=5

3. Data/value payload mapping:

   - Empty: `data = null`, `value = null`
   - Int: `data = ref $i64box(value)`, `value = null`
   - Float: `data = ref $f64box(value)`, `value = null`
   - Codepoint: `data = i31ref(char)`, `value = null`
   - Text/Symbol: `data = ref $String(ptr,len)`, `value = null`
     - strings live in linear memory via the string table (ptr/len)
   - Key: `data = left node`, `value = right node`
   - List/Block: `data = first node`, `value = rest list node` (cons cells)
   - TypeDef: `data = name node`, `value = body node`
   - True/False: encoded as Int 1/0
   - Meta is dropped during emission; Error currently emits the inner node
   - Data nodes currently serialize as `Symbol(type_name)` in the emitter

4. Verification pointers:

   - `tests/test_wasm_emitter.rs` covers `test_wasm_roundtrip` and `test_wasm_roundtrip_via_is`
   - `tests/test_wasm_reader.rs` documents the GC reading pattern

5. Round-trip remains:

   parse -> Node -> wasm emitter -> WASM GC object -> Node

## complete roundtrip test

eq! is just a shortcut for assert_eq! but
is! invokes the whole machinery, to parse, analyze, emit to wasm, read back, run / convert to Node again :

the is! macro triggers the following roundtrip: 
is!("3",3); => parse("3") -> Node -> wasm_node -> test.wasm -> wasm_node -> Node == 3
via warp::wasm_gc_emitter::eval and emit_node_main and Node::from_gc_object

### soon

compiletime and runtime evaluation
```
is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> Node -> eval() == 6
is!("def square:=it*it; square(3)",9); 
is!("def fib:=it<1 ? 1 : fib(it-1) + fib it-2; fib(10)",55); 
```

# Important
Don't cargo clean unless absolutely necessary!

Before and after each task run git status and ./test.sh to ensure we are in a clean state and all tests pass.
If previously passing test fail after the task as seen via git diff test_results.txt
try to fix failing tests and if it doesn't work roll back

When fixing a problem do not modify the test itself without consulting!

git status before and after each task should show
Your branch is up to date with 'origin/main'.
nothing to commit

use `cargo fix` after each commit and commit again

Other than fixme comment you can find new tasks via tests marked #[ignore = "next"] or even #[ignore = "soon"] 
