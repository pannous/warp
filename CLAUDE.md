# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **warp**, a rust implementation of **wasp** 
wasp is a data format and wasm first programming language
C++ source code locally at ~/wasp/ documentation at ~/wasp/wiki
https://github.com/pannous/wasp
https://wasp.pannous.com/

 - a WebAssembly parser and code generator for a custom AST (Abstract Syntax Tree) format. The project parses a custom syntax, builds a Node-based AST, and emits WebAssembly modules using:
- WIT (WebAssembly Interface Types) definitions
- WASM GC (Garbage Collection) bytecode
- Multiple WASM runtime backends (wasmtime, wasmer, wasmedge)

## Core Architecture

### Node AST (`src/node.rs`)
The central data structure is `Node`, an enum representing all AST node types:
- **Empty, Number, Text, Codepoint, Symbol** - Atomic values
- **KeyValue, Pair, Tag** - Binary structures
- **Block** - Contains child nodes with grouping via `Grouper` (parentheses, brackets, braces)
- **List** - Collection of nodes
- **Data** - Generic container using `Dada` for arbitrary Rust types with `CloneAny` trait
- **WithMeta** - Node wrapper that adds `Meta` (comments, line/column positions)

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

The project uses vendored dependencies (see `vendor/`) to support offline builds. The `Cargo.toml` warns against online compilation delays.

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

The project follows patterns from `~/dev/script/rust/rasm` for ergonomic WASM GC object introspection. See `docs/wasm-gc-reading-guide.md` for:
- Loading WAT modules with GC types enabled
- Reading GC struct fields by index
- Type-safe wrappers with `gc_struct!` macro
- Ergonomic `GcObject` wrapper hiding store management
- Creating GC objects from Rust

## Important Notes

Use WASM names excessively! Wasm provides custom sections for names, use ALL of them!

### Offline Development
The project is configured for **offline-first** development to avoid compilation delays. Dependencies are vendored and Cargo.toml has offline mode notes. Use `--offline` flag when building.


### Test File Locations
Tests are in `tests/` directory (not `src/`). Each test file is named `*_test.rs` and tests a specific module or feature.

### Extension Utilities
The `src/extensions/` directory provides Rust standard library extensions:
- `numbers.rs` - Extended number types (Complex, Quotient, etc.)
- `strings.rs` - String manipulation helpers
- `lists.rs` - Collection utilities
- `utils.rs` - General utilities (download, file I/O)
- more on demand

These are reexported in `lib.rs` for test access via `use wasp::*`.

## Development Workflow

0. **Run tests** - `cargo test` to verify we start from a clean state, check git logs
1. **Modify parser or emitter** - Edit files in `src/`
2. **Add tests** - Create or update tests in `tests/`
3. **Run tests** - `cargo test` to verify
4. **Check examples** - Run examples to see output
5. **Build offline** - Use `--offline` for reproducible builds

## Serialization
=== Node Serialization Workflow (Design) ===

to be checked via test_wasm_roundtrip

1. Define Node GC struct type in WAT:  (still subject to change)

   (type $node (struct
     (field $name (ref $string))   ;; For Tag nodes e.g. 'html{test=42}'
     (field $tag i32)              ;; NodeTag Kind / Type discriminant
     (field $int_value i64)        ;; For Number nodes
     (field $float_value f64)      ;; For Number nodes
     (field $text (ref $string))   ;; For Text/Symbol nodes
     (field $left (ref null $node)) ;; For Pair/Block/List nodes
     (field $right (ref null $node))
     (field $meta (ref null $node)) ;; For Pair/Block nodes
   ))
   the same type struct must be emitted to wasm bytecode

2. Create Rust wrapper:
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

3. Convert wasp::Node to WASM:
   let wasm_node = WaspNode::create(&template, obj! {
       tag: NodeTag::Number as i32,
       int_value: 42,
   })?;

   TODO save it!

   TODO automatically convert Node tree to WaspNode tree

   TODO kitchensink

Always verify it via wasm-tools print (passes, wasm-tools v1.243.0 has better GC support than wasm2wat)
DONE auto verify it via wasm verification crate (wasmparser with GC features enabled, passes)
TODO verify it via wasmtime run --enable-gc (needs wasmtime 28.0+ for full GC introspection)


4. Read back fields:
   let tag = wasm_node.tag()?;  // Auto-generated getter
   let value = wasm_node.int_value()?;

5. Round-trip: WASM -> Rust Node:
   let rust_node = Node::from_wasm_node(&wasm_node)?;


## complete roundtrip test
is!("3",3); => parse("3") -> Node -> wasm_node -> test.wasm -> wasm_node -> Node == 3

### soon
compiletime and runtime evaluation
is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> Node -> eval() == 6
is!("def square:=it*it; square(3)",9);
is!("def fib:=it<1 ? 1 : fib(it-1) + fib it-2; fib(10)",55); 
