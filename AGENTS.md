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
- wasmtime to run them (feature `native`; the browser build in web/playground runs them in JavaScript)

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


### Node AST (`src/node/`, the enum in `mod.rs`)

The central data structure is `Node`, an enum representing all AST node types:

- **Empty, Number, Text, Codepoint, Symbol** - Atomic values
- **Key** - Binary structures (Pair, Tag)
- **List/Block** - Collection of nodes with separator and grouping via `Grouper` (parentheses, brackets, braces)
- **Data** - Generic container using `Dada` for arbitrary Rust types with `CloneAny` trait
- **Meta** - Node wrapper that adds `Meta` (comments, line/column positions)

### Parser (`src/wasp_parser/`)

Recursive descent parser that converts text input to Node AST:

- Tracks position (line/column) for all nodes
- Handles comments (`//` line and `/* */` block) attached as metadata
- Parses literals (numbers, strings, symbols), groups ((), [], {}), and structures

### Pipeline (`src/pipeline.rs`: `compile`, `eval`, `lower`)

parse → `lower_for_emission` (the lowering passes, flat `src/*.rs` files such as `mutation.rs`, `lambdas.rs`,
`library_words.rs`, `switch.rs`, plus `analyzer/`) → analysis and diagnostics → WASM GC emitter → run.

### Emitters

0. **Text**: `Node::serialize` (`src/node/serialization.rs`), wasp notation similar to json5; `src/wisp_parser.rs` reads and writes Wisp
1. **WASM GC Emitter** (`src/wasm_emitter/`)
    - Generates WASM GC bytecode using the `wasm-encoder` crate; `mod.rs` emits programs, the other files the runtime
      functions (texts, lists, maps, unbounded ints, exact numbers, equality, WASI, FFI)
    - `function_builder.rs`: `runtime_function` / `exported_function` emit a whole function in one call
    - Uses the `Kind` enum (`src/type_kinds.rs`) for runtime type discrimination

### WASM Runtime (`src/wasm_reader.rs`, `src/run/wasmtime_runner.rs`, `src/host.rs`, `src/ffi/`)

- `wasm_reader::run_main` instantiates a module with the host, WASI or FFI imports and calls `main`
- `wasm_reader` and `gc_traits/` read the resulting GC objects back into Nodes

## Build and Test Commands

### Building

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --offline          # Offline mode (uses the local registry cache)
```

### Testing

Several agent sessions share this Mac, so test runs are rationed (roles and rules: notes/roles.md):
- Workers run targeted tests only, through the machine-wide queue: `tests/queue.sh -- <filter>`. A hook blocks direct
  `cargo test` / `cargo browser-test` runs.
- Only the Integrator session runs the full suite (`./test.sh`, which queues itself) and pushes code to main. Workers
  hand it "branch, tip, filters" and fix what it reports.

The underlying cargo commands (what tests/queue.sh runs):
```bash
cargo test                     # Run all tests
cargo test <test_name>         # Run specific test by name
cargo test --test tests <file_stem>::  # Run one test file: tests/<topic>/*.rs are modules of ONE test crate (tests/main.rs); add a new file as `mod x;` in its folder's mod.rs
```

#### Important Test Files

- `tests/node/test_node.rs` - Tests Node AST operations
- `tests/parser/test_parser.rs` - Tests parser functionality
- `tests/wasm/test_wasm_emitter.rs` - Tests WASM GC code generation
- `tests/wasm/test_wasm_reader.rs` - Tests reading WASM GC objects (see below)

## WASM GC Reading Patterns

The project follows patterns from `~/dev/script/rust/rasm` for ergonomic WASM GC object introspection
(`src/gc_traits/`, examples in `tests/wasm/test_wasm_reader.rs` and `tests/wasm/test_gc_struct.rs`):

- Loading WAT modules with GC types enabled
- Reading GC struct fields by index
- Type-safe wrappers with `gc_struct!` macro
- Ergonomic `GcObject` wrapper hiding store management
- Creating GC objects from Rust

## Important Notes

Use WASM names excessively! Wasm provides custom sections for names, use ALL of them!

### Offline Development

The project is configured for **offline-first** development to avoid compilation delays: dependencies come from the
local registry cache. Use `--offline` flag when building.

Vendoring is deactivated for now (user, 2026-10-04: "currently we don't need it but maybe we want to run an off-line
agent later again"): main has no `vendor/` and no source replacement. The machinery is kept as it is: the `vendor`
branch (vendored crates for main c5a44e05, 2026-10-03) and .github/workflows/offline-build-refresh.yml, which still
refreshes it when Cargo.lock changes on main; how to use it again: notes/cloud_offline_build.md.

### Test File Locations

Tests are in `tests/<topic>/` folders (not `src/`). Each test file is named `test_*.rs`, tests a specific module or
feature, and is declared in its folder's `mod.rs`; tests/main.rs declares the folders as modules of the one test crate.
Folder plan and condensing rules: notes/tests_layout.md.

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
4. **Build offline** - Use `--offline` for reproducible builds

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

   - `tests/wasm/test_wasm_emitter.rs` covers `test_wasm_roundtrip` and `test_wasm_roundtrip_via_is`
   - `tests/wasm/test_wasm_reader.rs` documents the GC reading pattern

5. Round-trip remains:

   parse -> Node -> wasm emitter -> WASM GC object -> Node

## complete roundtrip test

eq! is just a shortcut for assert_eq! but
is! invokes the whole machinery, to parse, analyze, emit to wasm, read back, run / convert to Node again :

the is! macro triggers the following roundtrip: 
is!("3",3); => parse("3") -> Node -> wasm_node -> test.wasm -> wasm_node -> Node == 3
via warp::wasm_emitter::eval and emit_node_main and Node::from_gc_object

### soon

compiletime and runtime evaluation
```
is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> Node -> eval() == 6
is!("def square:=it*it; square(3)",9); 
is!("def fib:=it<1 ? 1 : fib(it-1) + fib it-2; fib(10)",55); 
```

# Important
Don't cargo clean unless absolutely necessary!

The /probes/ folder is NOT a place to doublicate worktrees!
One branch per task, in a git worktree outside the repo: /Users/me/dev/angles/warp.worktrees.noindex/<name>
(notes/roles.md), or work on the same branch for small changes.
Create worktrees with `cowtree add <path> -b <branch> [<commit>]` (same arguments as `git worktree add`, which a hook blocks).

## Folders
- `probes/` = hand-written probe sources only (.wasp .md .rs .py .sh .lean .html, each under 100 KB), tracked: commit them, no `git add -f` needed.
- `scratch/` = disposable, ignored: repo exports/copies, worktrees, cargo homes, private index files (`scratch/<topic>.index`), temp builds. Deletable any time.
- `data/` = kept but ignored: logs, test result dumps, patches, json/txt outputs, benchmark data, agent logs (`data/<topic>/`).
- Rust build output goes to the one shared target dir set in ~/.cargo/config.toml (`target-dir`), never into the repo; agents and exports don't set CARGO_TARGET_DIR (a per-agent dir is ~20 GB and recompiles every dependency).
- `./test.sh` runs `probes/check_layout.sh`, which fails on tracked probes that are repo copies, too large, or of a non-source type.

Workers: before and after each task, run `git status` and the tests that cover your change:
`tests/queue.sh -- <filter>`. Do NOT run `./test.sh` (the full suite): only the Integrator runs it, after merging
your branch (notes/roles.md).
If previously passing test fail after the task as seen via git diff test_results.txt
try to fix failing tests and if it doesn't work roll back

When fixing a problem do not modify the test itself without consulting!
Standing permission (user, 2026-10-05: "allow all tests to be upgraded from a dumb thing to a better thing, from not
working to working"): a test may be upgraded without asking when the code now does better than the test pinned: an
expected error, refusal or "not supported yet" becomes the working value, an ignored test that passes is un-ignored,
a weaker assertion becomes the stronger one. Own commit, message naming this rule. Changing one working value into
a different working value (a change of meaning) still needs a user decision via the Interviewer.

git status before and after each task should show
Your branch is up to date with 'origin/main'.
nothing to commit

use `cargo fix --offline --allow-dirty --lib --bins` after each commit and commit again (`--lib --bins` keeps it out of tests/, `--allow-dirty` because cargo flags git-ignored non-.rs files as dirty)

## To-dos
This project keeps its to-dos on the board https://github.com/users/pannous/projects/1 (columns Now/Next/Soon/Later/Done),
not in todo.md: every issue you encounter goes in with `todo add "…"` (~/dev/bin/todo, column Next); `todo list`,
`todo move <card> <column>`, `todo done <card> [commit]` (never delete a card). `todo done` moves the card to Done and
links the fixing commit in its body (default HEAD, so run it right after committing the fix; a wiki or other-repo
change: pass the commit URL). Rule (user, 2026-10-06, issue #7 was closed without one): a card or issue is closed
only with a commit linked in its description. Close cards with `todo done` only; never `gh issue close` and never
the board UI; `todo move <card> Done` refuses a card whose description links no commit. Picking a card is
`todo take <card> <session name>` (user, 2026-10-06): the GitHub assignee is the user (agents have no accounts), the
board field Agent names the session, the card moves to Now. Without the board,
`todo add` falls back to todo.md "## Fallback"; `todo import` moves those entries later.

Other than fixme comment you can find new tasks via tests marked #[ignore = "next"] or even #[ignore = "soon"] 
un-ignore everything once it passes 

Add [profile.dev] debug = "line-tables-only" and incremental = false for probe builds!!
Never create a local target/ or *_target dir. Builds go to the shared target-dir from ~/.cargo/config.toml (sccache is on). Remove worktrees when done.
