## Project Overview

warp is a data format and wasm first programming language

https://github.com/pannous/warp
https://warp.pannous.com/

This is **warp**, the successor of **wasp** (C++). 

C++ source code locally at ~/wasp/ 
documentation and specification at ./wiki 
The specification is not fully implemented yet 

- The project parses a custom syntax, builds a Node-based AST, and emits WebAssembly modules using:
- WIT (WebAssembly Interface Types) definitions
- WASM GC (Garbage Collection) bytecode
- wasmtime to run them (feature `native`; the browser build in web/playground runs them in JavaScript)

# self hosted

It began as a Rust implementation; the compiler is now self-hosted
in warp (core/*.warp, notes/self_hosting.md). The Rust code in src/ and crates/ is frozen legacy: the bootstrap
that first compiles compiler.warp, the native host (wasmtime, host words, FFI) and the runtime (notes/agents/common.md
"Principles"). 

The core WARP compiler is functional and able to compile itself. From now on, most things should be implemented as libraries, or as compiler extensions. If the core compiler (or even the Rust bootstrap) needs changes, this needs to go through the interviewer. 

warp0.wasm bootstrapped by Rust implementation, partially using .warp files? legacy branch rust-bootstrap
warp.wasm compiler written in pure warp (compiled by warp0.wasm or warp2.wasm)
warp2.wasm recompiled fix point


## Core Architecture

Warp is similar to lisp: a data and code format from first principles,
but instead of s-expressions they use a more modern syntax with blocks, lists, key-value pairs, tags, comments, and rich atomic types.

```warp
Person {
    name: "Alice"    
    age: 30             
    hobbies: [ "reading", "hiking", "coding" ]
}
```

It is similar to JSON5 / ECMA Script but much simplified yet with a richer syntax and more precise and flexible data model.

### The warp compiler (`core/`; `lib/compiler` links to it for a transition)

`compiler.warp` is the whole compiler as one warp program, source text in, wasm bytes out:
parser.warp → passes.warp (the lowering passes; sugar as rewrite rules in lib/sugar/) → analyzer.warp → emitter.warp,
whose runtime words are warp too (runtime.warp, runtime/*.warp). core/node.warp declares the Node enum case for
case as src/node/mod.rs has it.
- `warp pure <file|code>`: runs warp.wasm, compiler.warp compiled by the seed (seed/warp-self.wasm, a fixpoint
  compiler checked in; scripts/refresh-seed.sh refreshes it). No Rust-compiled stage builds either compiler
- `warp self <file|code>`: runs warp-self.wasm, a copy of warp.wasm promoted only when warp.wasm passes the fixpoint
  test (it compiles compiler.warp to its own bytes, stage N+1 == N)
- `warp rebuild` / `warp rebuild pure`: builds warp.wasm anew from lib/ (nothing rebuilds it automatically); the gate
  needs only warp.wasm. `warp rebuild self` runs the fixpoint test and promotes; `scripts/promote-self.sh
  [--background] [<tree>]` does that every once in a while, in parallel, and never fails its caller. A worktree's
  compilers live in <worktree>/data/ (`WARP_LIB=<worktree>/lib`), main's beside the binary. Until 2026-10-26 they were
  named compiler.wasm and compiler-self.wasm; those names are still read when the new ones are missing
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

### Warp parser

Recursive descent parser that converts text input to Node AST:

- Tracks position (line/column) for all nodes
- Handles comments (`//` line and `/* */` block) attached as metadata
- Parses literals (numbers, strings, symbols), groups ((), [], {}), and structures

### Legacy rust pipeline (`src/pipeline.rs`: `compile`, `eval`, `lower`)

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

Agents must inherit a clean baseline and start working from there. 
Agents must not rebuild and retest the baseline, this will save many rebuilds.
The integrator must make sure that the main branch is always clean, functional, and up to date.
As soon as a branch passes the quick fix test, that's signal enough to be worthy of merging.
A full suite test can be done in parallel or later or batched.

The `warp` binary (the Rust host and bootstrap) is built by cargo; the warp compiler it runs by `warp rebuild`:
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --offline          # Offline mode (uses the local registry cache)
warp rebuild pure              # warp.wasm from lib/ by the seed
scripts/promote-self.sh        # warp.wasm -> warp-self.wasm only at a fixpoint (`warp rebuild self`); never fails
```


## To-dos
This project keeps its to-dos on the board https://github.com/users/pannous/projects/1 (columns Now/Next/Soon/Later/Done),
every issue you encounter goes in with `todo add "…"` (~/dev/bin/todo, column Next) todo.md as fallback
`todo list`, `todo move <card> <column>`, `todo done <card> [commit]` (never delete a card). 
Picking a card is `todo take <card> <session name>` The branch will and must be named identical to the card. 

When encountering a TODO without corresponding card in my main checkout ~/dev/angles/warp/lib/ Create a card for that to do and mark the card name behind the todo and commit 

When implementing gaps that the old Rust legacy implementation had, but are not part of the pure WARP implementation yet, do not resort to blindly copying old spaghetti code. Instead, create a clean, idiomatic new implementation from first principles. 

<!-- un-ignore every #[ignore = "next"] or even #[ignore = "soon"]  once it passes  -->


## Pre-approved plans
Every worker gets a plan from Supervisor, and everything in it is pre-approved: implement it without asking.
A good idea of your own beyond the plan may be implemented too, as long as it is undoable (its own commits on a branch).
<!-- `todo add "merge: <branch> <sha>" Ready -b "<one-line summary, tests run>"`.  -->
Main moved again in the meantime?
If the minimal smoke test (Compiling Hello World) passes its task is done, and it can go fully to the integrator.
Then `todo ready <card> [branch]`
The Integrator merges and closes them with `todo done`.
The Git commit message Merge branch MUST Also include a summary of the changes (TODO via todo tool?)

### Testing

Several agent sessions share this Mac, so test runs are rationed (roles and rules: notes/roles.md):
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

#### Important Test Files

- `tests/parser/test_parser.rs`, `test_parser.pairs.warp` - warp data notation read into values
- `tests/wasm/test_wasm.rs`, `test_wasm.pairs.warp`, `test_wasm.warp` - programs compiled to WASM GC and run (is!)

### Test File Locations

Tests are in `test/<topic>/` folders. 
Warp tests are `test_*.pairs.warp` (code ⇥ expected, one a line)
Folder plan and condensing rules: notes/tests_layout.md notes/is_tests.md 

## Development Workflow

0. **Check state** - git log, git status
1. Develop new features preferably in library /lib/
2. Only in rare cases touch core/parser.warp or emitter.warp 
   /Users/me/dev/bin/warp rebuild pure`, then `… warp pure <program>` 
3. **Add probes** - Create or update quick tests in `probes/`
4. **Add tests** - Create or update clean short functional tests in `test/`
5. **Run tests** - warp test <folders>`

## Serialization

=== Node Serialization Workflow (Current Spec) ===
node.warp via
emitter.warp in notes/self_hosting.md "Value layout".

1. Core GC types emitted to WASM:

   (type $Text (array (mut i8)))      ;; type 0: a text's UTF-8 bytes

   (type $Node (struct
     (field $kind i64)            ;; Kind tag + extra info in high bits
     (field $data anyref)         ;; payload (boxed number, $Text, i31ref, list or node)
     (field $value (ref null $Node)))) ;; secondary payload (node or null)

   (type $i64box (struct (field $value i64)))
   (type $f64box (struct (field $value f64)))

   Text literals come from one passive data segment via array.new_data 
   Linear memory holds a text only at the host/FFI edge, or in optimized algorithms, as 
   (type $String (;0;) (struct (field $ptr i32) (field $len i32))) 
   <!-- `text_address` copies a $Text there for a host
   or C call, `text_from_memory` copies host bytes into a new $Text (`new_text(ptr, len)` makes a Text node so).
   Readers (`src/gc_traits/gc_string.rs`, `src/run/wasm_reader.rs`, `crates/warp-runtime/src/gc_values.rs`) still also
   accept the old `$String (struct (field $ptr i32) (field $len i32))`.
   Lists held as arrays: `$IntList`, `$FloatList`, `$NodeList` (length, items array), maps `$NodeMap`; a program that
   quotes code gets `$MetaNode`, a $Node subtype with line, column and comment. -->

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

<!-- 4. Verification pointers:

   - `tests/wasm/test_wasm_emitter.rs` covers `test_wasm_roundtrip_via_is`
 -->
5. Round-trip:

   code -> parse -> Node -> wasm emitter -> WASM GC object -> Node -> value


## complete roundtrip test

is! <code> <value>   in lib/prelude.warp

is! invokes the whole machinery, to parse, analyze, emit to wasm, read back, run / convert to Node again

is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> test.wasm -> wasm_node -> Node == 6
```
is!("def fib:=it<1 ? 1 : fib(it-1) + fib it-2; fib(10)",55); 
```

## Outcome tests only

A test gives warp code and checks what it produces: the value, printed output, error, effect or CLI output.

No tests of intermediates: parse tree shapes, node views, pass outputs, analyzer kind tables, wasm bytes, module or GC
layouts, internal APIs. 


## Commit everything
Commit and push all meaningful progress all the time, including notes, handovers and WIP: a box or session can
disappear

# Important
NEVER modify a README without the user's review and explicit confirmation.

## worktrees
Small or non-conflicting changes may go straight onto main without a worktree. 
Otherwise one branch per task, in a git worktree outside the repo: /Volumes/Worktrees/<name>
/Volumes/Worktrees/warp is cowtree's donor, never removed.
/Volumes/Worktrees/integrator holds the Integrator's tools and logs; notes/roles.md),
Create worktrees with `cowtree add <path> -b <branch> [<commit>]` 
The probes/ folder is NOT a place to doublicate worktrees!

## Folders
- `probes/` = hand-written probe sources only 
- `data/` = kept but ignored: logs, test result dumps, patches, json/txt outputs, benchmark data, agent logs (`data/<topic>/`).

Workers: before and after each task, run `git status` and the tests that cover your change 
<!-- (cloud lane: `scripts/cloud-test.sh`).  -->
Do NOT run `./test.sh` (the full suite): only the Integrator runs it, after merging your branch (notes/roles.md).
If previously passing test fail after the task as seen via git diff test_results.txt try to fix failing tests and if it doesn't work roll back

When fixing a problem NEVER modify the test itself without consulting!
A test may be upgraded without asking when the code now does better 
<!-- when the code now does better than the test pinned: an
expected error, refusal or "not supported yet" becomes the working value, an ignored test that passes is un-ignored,
a weaker assertion becomes the stronger one.
 -->
git status before and after each task should show
Your branch is up to date with 'origin/main'.
nothing to commit

<!-- ### Associated projects
~/Library/Application Support/Sublime Text 3/Packages/Warp
 -->
Remove worktrees and branches when done!