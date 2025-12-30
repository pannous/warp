# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **warp**, a rust implementation of **wasp** 
wasp is a data format and wasm first programming language
locally at ~/wasp/ ~/wasp/wiki
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

1. **WIT Emitter** (`src/wit_emitter.rs`)
   - Generates WebAssembly Interface Type definitions
   - Outputs `.wit` files defining type shapes for Node variants

2. **WASM GC Emitter** (`src/wasm_gc_emitter.rs`)
   - Generates WASM GC bytecode using `wasm-encoder` crate
   - Creates GC struct types for each Node variant with proper tagging
   - Uses `NodeKind` enum for runtime type discrimination
   - **Recently migrated** from deprecated WASM GC API to proper `wasm-encoder` GC API (see commit 9e1315c)

3. **Generic Emitter** (`src/emitter.rs`)
   - Legacy/fallback emitter

### WASM Runtime Support (`src/run/`)
Multiple runtime backends for executing generated WASM:
- `wasmtime_runner.rs` - Primary runtime (requires wasmtime 40.0+)
- `wasmer_runner.rs` - Alternative runtime
- `wasmedge_runner.rs` - Alternative runtime

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
- `tests/wasm_gc_emitter_test.rs` - Tests WASM GC code generation
- `tests/wasm_gc_features_test.rs` - Tests WASM GC feature support
- `tests/wasm_gc_read_test.rs` - Tests reading WASM GC objects (see guide below)
- `tests/wasp_parser_test.rs` - Tests parser functionality
- `tests/wasp_comments_test.rs` - Tests comment preservation
- `tests/wasp_position_test.rs` - Tests position tracking
- `tests/wit_emitter_test.rs` - Tests WIT generation
- `tests/json_test.rs` - Tests JSON interop
- `tests/node_test.rs` - Tests Node AST operations

### Running Examples
```bash
cargo run --example wasm_gc_generation
cargo run --example wit_generation
cargo run --example wasp_comments_demo
```

## Key Dependencies

### WASM Tooling
- `wasmtime` (v40.0.0) - Primary WASM runtime with GC support
- `wasm-encoder` - WASM module generation (proper GC API)
- `wasmparser` - WASM module parsing
- `wasm-compose`, `wasm-metadata` - Module composition and metadata
- `wasmprinter` - WASM text format printing
- `wast`, `wat` - WebAssembly text format parsing
- `wit-component` - Component model support
- `parity-wasm` - Alternative WASM serialization (incomplete GC support)

### Rust Utilities
- `syn` - Rust AST parsing (not WASM32 targets)
- `regex` - Pattern matching
- `serde`, `serde_json` - Serialization
- `paste` - Macro utilities

## WASM GC Reading Patterns

The project follows patterns from `~/dev/script/rust/rasm` for ergonomic WASM GC object introspection. See `docs/wasm-gc-reading-guide.md` for:
- Loading WAT modules with GC types enabled
- Reading GC struct fields by index
- Type-safe wrappers with `gc_struct!` macro
- Ergonomic `GcObject` wrapper hiding store management
- Creating GC objects from Rust

**Key requirement**: Wasmtime 28.0+ for full GC introspection (currently using 40.0.0)

## Important Notes

### Offline Development
The project is configured for **offline-first** development to avoid compilation delays. Dependencies are vendored and Cargo.toml has offline mode notes. Use `--offline` flag when building.

### WASM GC Migration
Recent architectural change (commit 9e1315c): Migrated from deprecated WASM GC API to proper `wasm-encoder` GC API. The `WasmGcEmitter` now uses:
- Proper `StructType` definitions instead of deprecated approaches
- Type section management with GC-aware constructors
- Validated against wasmtime 40.0.0 GC introspection support

### Test File Locations
Tests are in `tests/` directory (not `src/`). Each test file is named `*_test.rs` and tests a specific module or feature.

### Extension Utilities
The `src/extensions/` directory provides Rust standard library extensions:
- `numbers.rs` - Extended number types (Complex, Quotient, etc.)
- `strings.rs` - String manipulation helpers
- `lists.rs` - Collection utilities
- `utils.rs` - General utilities (download, file I/O)

These are reexported in `lib.rs` for test access via `use wasp::*`.

## Development Workflow

1. **Modify parser or emitter** - Edit files in `src/`
2. **Add tests** - Create or update tests in `tests/`
3. **Run tests** - `cargo test` to verify
4. **Check examples** - Run examples to see output
5. **Build offline** - Use `--offline` for reproducible builds

## Subproject: LO

The `LO/` directory contains a separate WebAssembly project (likely "Language Oriented" or similar) with its own build system (`build.sh`), WASM binary (`lo.wasm`), and VSCode extension (`vscode-ext/`). It appears to be related tooling but is independently maintained.
