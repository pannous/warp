## Project Overview

This is **warp**, the successor of **wasp** (C++). It began as a Rust implementation; the compiler is now self-hosted
in warp (lib/compiler/*.warp, notes/self_hosting.md). The Rust code in src/ and crates/ is frozen legacy: the bootstrap
that first compiles compiler.warp, the native host (wasmtime, host words, FFI) and the runtime (notes/agents/common.md
"Principles"). 

# self hosted
The core WARP compiler is functional and able to compile itself. From now on, most things should be implemented as libraries, or as compiler extensions. If the core compiler (or even the Rust bootstrap) needs changes, this needs to go through the interviewer. 

https://github.com/pannous/warp
https://warp.pannous.com/

warp is a data format and wasm first programming language
C++ source code locally at ~/wasp/ documentation and specification at ./wiki 
The specification is not fully implemented yet 

- The project parses a custom syntax, builds a Node-based AST, and emits WebAssembly modules using:
- WIT (WebAssembly Interface Types) definitions
- WASM GC (Garbage Collection) bytecode
- wasmtime to run them (feature `native`; the browser build in web/playground runs them in JavaScript)

## Core Architecture

Wasp Wisp and Warp are similar to lisp in that they are a data format from first principles,
but instead of s-expressions they use a more modern syntax with blocks, lists, key-value pairs, tags, comments, and rich atomic types.

```warp
Person {
    name: "Alice"    
    age: 30             
    hobbies: [ "reading", "hiking", "coding" ]
}
```

It is similar to JSON5 / ECMA Script but much simplified yet with a richer syntax and more precise and flexible data model.


### The warp compiler (`lib/compiler/`, `core/` links to it)

`compiler.warp` is the whole compiler as one warp program, source text in, wasm bytes out:
parser.warp → passes.warp (the lowering passes; sugar as rewrite rules in lib/sugar/) → analyzer.warp → emitter.warp,
whose runtime words are warp too (runtime.warp, runtime/*.warp). lib/compiler/node.warp declares the Node enum case for
case as src/node/mod.rs has it.
- `warp pure <file|code>`: runs compiler.wasm, compiler.warp compiled by the Rust compiler
- `warp self <file|code>`: runs compiler-self.wasm, compiler.warp compiled by itself
- `warp rebuild [pure|self]`: builds them anew from lib/ (nothing rebuilds them automatically); a worktree's pair lives
  in <worktree>/data/ (`WARP_LIB=<worktree>/lib`), main's beside the binary
- A form outside emitter.warp's core is refused loudly (`outside the core: …`)

Plain `warp <file>` still compiles with the Rust pipeline below.

### Node AST (the enum in `core/node.warp`)

The central data structure is `Node`, an enum representing all AST node types:

- **Empty, True, False, Number, Char, Text, Symbol** - Atomic values
- **Key** - Binary structures (Pair, Tag), with an `Op`
- **List** - Collection of nodes with a `Bracket` (curly, square, round …) and a `Separator`
- **Data** - Generic container (`DataValue`) for arbitrary Rust types with the `CloneAny` trait
- **Meta** - Node wrapper that adds metadata (comments, line/column positions)
- **Error, Type** - an error; a type definition (name + body)

### Rust parser (`src/warp_parser/`, the bootstrap)

Recursive descent parser that converts text input to Node AST:

- Tracks position (line/column) for all nodes
- Handles comments (`//` line and `/* */` block) attached as metadata
- Parses literals (numbers, strings, symbols), groups ((), [], {}), and structures

### Rust pipeline (`src/pipeline.rs`: `compile`, `eval`, `lower`)

parse → `lower_for_emission` (the lowering passes, flat `src/*.rs` files such as `mutation.rs`, `lambdas.rs`,
`library_words.rs`, `switch.rs`, plus `analyzer/`) → analysis and diagnostics → WASM GC emitter → run.

### Emitters

0. **Text**: `Node::serialize` (`src/node/serialization.rs`), warp notation similar to json5; `src/formats/wisp_parser.rs` reads and writes Wisp
1. **Rust WASM GC Emitter** (`src/wasm_emitter/`; emitter.warp is its warp successor)
    - Generates WASM GC bytecode using the `wasm-encoder` crate; `mod.rs` emits programs, the other files the runtime
      functions (texts, lists, maps, unbounded ints, exact numbers, equality, WASI, FFI)
    - `function_builder.rs`: `runtime_function` / `exported_function` emit a whole function in one call
    - Uses the `Kind` enum (`src/type_kinds.rs`) for runtime type discrimination

### WASM Runtime (`src/run/wasm_reader.rs`, `src/run/wasmtime_runner.rs`, `src/host.rs`, `src/ffi/`)

- `wasm_reader::run_main` instantiates a module with the host, WASI or FFI imports and calls `main`
- `wasm_reader` and `gc_traits/` read the resulting GC objects back into Nodes

## Build and Test Commands

### Building

The `warp` binary (the Rust host and bootstrap) is built by cargo; the warp compiler it runs by `warp rebuild`:
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --offline          # Offline mode (uses the local registry cache)
warp rebuild pure              # compiler.wasm from lib/ (`warp rebuild self`: compiler-self.wasm)
```


## To-dos
This project keeps its to-dos on the board https://github.com/users/pannous/projects/1 (columns Now/Next/Soon/Later/Done),
every issue you encounter goes in with `todo add "…"` (~/dev/bin/todo, column Next) todo.md as fallback
`todo list`, `todo move <card> <column>`, `todo done <card> [commit]` (never delete a card). 
Picking a card is `todo take <card> <session name>` The branch will and must be named identical to the card. 

TODO: Old cards with bugs in the Rust implementation must be dropped unless that bug is critical for bootstrapping of warp
TODO: Old cards with features for the Rust implementation must be converted to WARP implementation or dropped 

When encountering a TODO without corresponding card in my main checkout ~/dev/angles/warp/lib/ Create a card for that to do and mark the card name behind the todo and commit 

<!-- un-ignore every #[ignore = "next"] or even #[ignore = "soon"]  once it passes  -->


## Pre-approved plans
Every worker gets a plan from Supervisor, and everything in it is pre-approved: implement it without asking.
A good idea of your own beyond the plan may be implemented too, as long as it is undoable (its own commits on a branch).
<!-- `todo add "merge: <branch> <sha>" Ready -b "<one-line summary, tests run>"`.  -->
Then `todo ready <card> [branch]`
The Integrator merges and closes them with `todo done`.
The Git commit message Merge branch MUST Also include a summary of the changes (TODO via todo tool?)

### Testing

Several agent sessions share this Mac, so test runs are rationed (roles and rules: notes/roles.md):
- Rust tests Should only be run irregularly in the rare cases that modifications to the legacy Bootstrap Rust are needed 
-  run in the cloud : workers run targeted tests only, with `scripts/cloud-test.sh`;
  tests/queue.sh only for the Mac-only tests (scripts/cloud-test-environmental.txt). A hook blocks direct
  `cargo test` / `cargo browser-test` runs.
- Only the Integrator session runs the full suite (`./test.sh`, which queues itself) and pushes code to main. Workers
  hand it "branch, tip, filters" and fix what it reports.

Warp tests run locally with the shared binary, through the warp compiler (src/compiler/pure_tests.rs,
notes/is_tests.md):
```bash
warp test                      # all of tests/: each *.pairs.warp pair, each is! line of test_*.warp, via warp pure
warp test tests/numbers        # only these folders; --self: via `warp self`
```

The Rust tests' cargo commands (what the cloud lane and tests/queue.sh run):
```bash
cargo test                     # the whole Rust suite
cargo test <test_name>         # Run specific test by name
cargo test --test tests <file_stem>::  # Run one test file: tests/<topic>/*.rs are modules of ONE test crate (tests/main.rs); add a new file as `mod x;` in its folder's mod.rs
```

#### Important Test Files

- `tests/parser/test_parser.rs`, `test_parser.pairs.warp` - warp data notation read into values
- `tests/wasm/test_wasm.rs`, `test_wasm.pairs.warp`, `test_wasm.warp` - programs compiled to WASM GC and run (is!)

<!-- ## WASM GC Reading Patterns -->

<!-- The project follows patterns from `~/dev/script/rust/rasm` for ergonomic WASM GC object introspection -->
<!-- (`src/gc_traits/`): -->
<!-- 
- Loading WAT modules with GC types enabled
- Reading GC struct fields by index
- Type-safe wrappers with `gc_struct!` macro
- Ergonomic `GcObject` wrapper hiding store management
- Creating GC objects from Rust
 -->
## Important Notes

Use WASM names excessively! Wasm provides custom sections for names, use ALL of them!

### Offline Development
Legacy Rust:
The project is configured for **offline-first** development to avoid compilation delays: dependencies come from the
local registry cache. Use `--offline` flag when building.

### Test File Locations

Tests are in `tests/<topic>/` folders (not `src/`). Warp tests are `test_*.pairs.warp` (code ⇥ expected, one a line)
and `test_*.warp` (is! lines), beside the Rust file of the same name (notes/is_tests.md). Each Rust test file is named
`test_*.rs`, tests a specific module or feature, and is declared in its folder's `mod.rs`; tests/main.rs declares the
folders as modules of the one test crate.
Folder plan and condensing rules: notes/tests_layout.md.

### Extension Utilities

The `src/extensions/` directory provides Rust standard library extensions (for the Rust side):

- `numbers.rs`, `reals.rs` - Extended number types (Complex, Quotient, etc.)
- `strings.rs` - String manipulation helpers
- `lists.rs` - Collection utilities
- `utils.rs` - General utilities (download, file I/O)
- more on demand

These are reexported in `lib.rs` for test access via `use warp::*`.

## Development Workflow

0. **Check state** - git log, git status
1. **Modify parser or emitter** - in lib/compiler/*.warp (src/ is frozen), checked with the one shared build:
   `WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp rebuild pure`, then `… warp pure <program>` (no local cargo builds
   besides main's)
2. **Add tests** - Create or update tests in `tests/`
3. **Run tests** - Warp tests locally: `WARP_LIB=<worktree>/lib /Users/me/dev/bin/warp test <folders>`; Rust tests
   in the cloud: `scripts/cloud-test.sh --card <card> <branch> <tip> <filters>`

## Serialization

=== Node Serialization Workflow (Current Spec) ===

Current spec lives in `src/wasm_emitter/type_manager.rs` and `src/wasm_emitter/constructors.rs` (Rust emitter), and for
emitter.warp in notes/self_hosting.md "Value layout".

1. Core GC types emitted to WASM:

   (type $Text (array (mut i8)))      ;; type 0: a text's UTF-8 bytes

   (type $Node (struct
     (field $kind i64)            ;; Kind tag + extra info in high bits
     (field $data anyref)         ;; payload (boxed number, $Text, i31ref, list or node)
     (field $value (ref null $Node)))) ;; secondary payload (node or null)

   (type $i64box (struct (field $value i64)))
   (type $f64box (struct (field $value f64)))

   Text literals come from one passive data segment via array.new_data (`src/wasm_emitter/string_table.rs`).
   Linear memory holds a text only at the host/FFI edge, as (ptr, len): `text_address` copies a $Text there for a host
   or C call, `text_from_memory` copies host bytes into a new $Text (`new_text(ptr, len)` makes a Text node so).
   Readers (`src/gc_traits/gc_string.rs`, `src/run/wasm_reader.rs`, `crates/warp-runtime/src/gc_values.rs`) still also
   accept the old `$String (struct (field $ptr i32) (field $len i32))`.
   Lists held as arrays: `$IntList`, `$FloatList`, `$NodeList` (length, items array), maps `$NodeMap`; a program that
   quotes code gets `$MetaNode`, a $Node subtype with line, column and comment.

2. Kind encoding (see `src/type_kinds.rs`):

   - Lower 8 bits are `Kind` (Empty=0, Int=1, Float=2, Text=3, Codepoint=4, Symbol=5, Key=6, Block=7, List=8, Data=9, Meta=10, Error=11, TypeDef=12, ... Time=18)
   - For Key and List, extra info is stored in the high bits:
     - key: `kind = (op_info << 8) | Kind::Key`
     - list: `kind = (bracket_info << 8) | Kind::List`
       bracket_info: Curly=0, Square=1, Round=2, Less=3, Other=4, None=5

3. Data/value payload mapping:

   - Empty: `data = null`, `value = null`
   - Int: `data = ref $i64box(value)`, `value = null`
   - Float: `data = ref $f64box(value)`, `value = null`
   - Codepoint: `data = i31ref(char)`, `value = null`
   - Text/Symbol: `data = ref $Text`, `value = null`
   - Key: `data = left node`, `value = right node`
   - List/Block: Rust emitter: `data = first node`, `value = rest list node` (cons cells); emitter.warp: `data = $list`
     vector (length, items array); readers tell them apart by the first field
   - TypeDef: `data = name node`, `value = body node`
   - True/False: a bool, the Int 1/0 marked BOOL_INFO above the kind bits (kind = BOOL_KIND); prints yes/no (notes/bool_type.md)
   - Meta is dropped during emission; Error currently emits the inner node
   - Data nodes currently serialize as `Symbol(type_name)` in the emitter

4. Verification pointers:

   - `tests/wasm/test_wasm_emitter.rs` covers `test_wasm_roundtrip_via_is`

5. Round-trip remains:

   parse -> Node -> wasm emitter -> WASM GC object -> Node

## Outcome tests only

A test gives warp code and checks what it produces: the value (is!), printed output, error, effect or CLI output.
No tests of intermediates: parse tree shapes, node views, pass outputs, analyzer kind tables, wasm bytes, module or GC
layouts, internal Rust APIs. Parsing warp data notation to a value is an outcome. The remaining twins that compare
warp-written compiler parts with Rust (emitter and analyzer, judged by the program's value) go once the first pipeline
runs completely in warp; from then on only outcome tests.

## complete roundtrip test

eq! is just a shortcut for assert_eq! but
is! invokes the whole machinery, to parse, analyze, emit to wasm, read back, run / convert to Node again
(in warp tests `is!` is lib/prelude.warp's `eval(warp) == expected`, notes/is_tests.md):

the is! function triggers the following roundtrip: 
is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> test.wasm -> wasm_node -> Node == 6


compiletime and runtime evaluation
```
is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> Node -> eval() == 6
is!("def square:=it*it; square(3)",9); 
is!("def fib:=it<1 ? 1 : fib(it-1) + fib it-2; fib(10)",55); 
```

## Commit everything
Commit and push all meaningful progress all the time, including notes, handovers and WIP: a box or session can
disappear

# Important
Don't cargo clean unless absolutely necessary!

NEVER modify a README without the user's review and explicit confirmation.

## worktrees
The /probes/ folder is NOT a place to doublicate worktrees!
Small or non-conflicting changes may go straight onto main without a worktree. 
Otherwise one branch per task, in a git worktree outside the repo: /Volumes/Worktrees/<name>
/Volumes/Worktrees/warp is cowtree's donor, never removed.
/Volumes/Worktrees/integrator holds the Integrator's tools and logs; notes/roles.md),
Create worktrees with `cowtree add <path> -b <branch> [<commit>]` 

## Folders
- `probes/` = hand-written probe sources only 
- `data/` = kept but ignored: logs, test result dumps, patches, json/txt outputs, benchmark data, agent logs (`data/<topic>/`).
- Rust build output goes to the one shared target dir set in ~/.cargo/config.toml (`target-dir`), never into the repo; agents and exports don't set CARGO_TARGET_DIR (a per-agent dir is ~20 GB and recompiles every dependency).
- `./test.sh` runs `probes/check_layout.sh`, which fails on tracked probes that are repo copies, too large, or of a non-source type.

Workers: before and after each task, run `git status` and the tests that cover your change (cloud lane:
`scripts/cloud-test.sh`). Do NOT run `./test.sh` (the full suite): only the Integrator runs it, after merging
your branch (notes/roles.md).
If previously passing test fail after the task as seen via git diff test_results.txt
try to fix failing tests and if it doesn't work roll back

When fixing a problem do not modify the test itself without consulting!
A test may be upgraded without asking when the code now does better than the test pinned: an
expected error, refusal or "not supported yet" becomes the working value, an ignored test that passes is un-ignored,
a weaker assertion becomes the stronger one.

git status before and after each task should show
Your branch is up to date with 'origin/main'.
nothing to commit

cargo fix runs in the Integrator's gate for every batch (integrator_tools/gate.sh); workers need not run it.

### Associated projects
~/Library/Application Support/Sublime Text 3/Packages/Warp

Builds go to the shared target-dir from ~/.cargo/config.toml (sccache is on). 
Never create a local target/ or *_target dir. 
Remove worktrees and branches when done!
