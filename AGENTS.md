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
- a small Rust harness (wasmtime) to run them; the browser build in web/playground runs them in JavaScript

# self hosted

warp began as a Rust implementation; since 2026-10-26 the compiler is written in warp (core/*.warp,
notes/self_hosting.md) and compiles itself. The old Rust compiler is the snapshot on branch `rust-bootstrap`; nothing
of it is on main. What stays Rust is the harness in src/ and crates/warp-runtime: the native host (wasmtime, host
words, FFI), the runtime that reads results back, and the CLI. Most things are implemented as libraries (lib/) or
compiler extensions (core/extensions/). If the core compiler (core/*.warp) or the Rust harness needs changes, this needs
to go through the interviewer. Ignore all old tasks about Rust features or  'both lanes'. 

The seed, warp-seed.wasm beside the binary (~/dev/bin; no file of git, published on GitHub pre-releases seed-<date>), is
the seed compiler; it compiles compiler.warp to
warp.wasm, which promotes itself to warp-self.wasm when it compiles itself once more (no fixed point needed, user
2026-10-26).

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
whose runtime words are warp too (runtime.warp, runtime/*.warp). core/node.warp declares the Node enum.
- `warp <file|code>`: runs warp.wasm, compiler.warp compiled by the seed (warp-seed.wasm beside the binary, a
  compiler that compiles itself; scripts/refresh-seed.sh refreshes and publishes it). No Rust-compiled stage builds either compiler
- `warp self <file|code>`: runs warp-self.wasm, a copy of warp.wasm promoted when warp.wasm compiles
  compiler.warp once more (a fixpoint, stage N+1 == N, is only reported)
- `warp rebuild` / `warp rebuild pure`: builds warp.wasm anew from lib/ (nothing rebuilds it automatically); the gate
  needs only warp.wasm. `warp rebuild self` runs that self test and promotes; `scripts/promote-self.sh
  [--background] [<tree>]` does that every once in a while, in parallel, and never fails its caller. A worktree's
  compilers live in <worktree>/lib/ itself (`WARP_LIB=<worktree>/lib`), main's beside the binary. Until 2026-10-26 they were
  named compiler.wasm and compiler-self.wasm; those names are still read when the new ones are missing
- A form outside emitter.warp's core is refused loudly (`outside the core: …`)

Plain `warp <file>` and `warp run` run through the same compiler (warp.wasm).

### Node AST (the enum in `core/node.warp`)

The central data structure is `Node`, an enum representing all AST node types:

- **Empty, True, False, Number, Char, Text, Symbol** - Atomic values
- **Key** - Binary structures (Pair, Tag), with an `Op`
- **List** - Collection of nodes with a `Bracket` (curly, square, round …) and a `Separator`
- **Data** - Generic container for an arbitrary value (`Data(value: any)`)
- **Meta** - Node wrapper that adds metadata (comments, line/column positions)
- **Error, Type** - an error; a type definition (name + body)

### Warp parser (`core/parser.warp`)

Recursive descent parser that converts text input to Node AST:

- Tracks position (line/column) for all nodes
- Handles comments (`//` line and `/* */` block) attached as metadata
- Parses literals (numbers, strings, symbols), groups ((), [], {}), and structures

### Emitter (`core/emitter.warp`)

Generates the WASM GC bytecode and its runtime words (texts, lists, maps, unbounded ints, exact numbers, equality,
WASI, FFI) in warp; the value layout and the `Kind` enum are in notes/self_hosting.md "Value layout".

### Rust harness (`src/`, `crates/warp-runtime`)

Runs the compiled modules and nothing else: `src/run/` (wasmtime), `src/host/` and `src/host.rs` (host words, sandbox,
channels, database), `src/ffi/`, `src/gc_traits/` (reads GC objects back into Nodes), `src/node/` (printing results),
`src/cli/`, `src/compiler/standalone.rs` and `pure_tests.rs` (run warp.wasm, `warp test`).
This harness should barely ever be rebuilt, and if so, independent of the warp files, and all tests should be completely parallel and non-blocking.

## Build and Test Commands

### Building

Agents must inherit a clean baseline and start working from there. 
Agents must not rebuild and retest the baseline, this will save many rebuilds.
Whoever pushes keeps main clean, functional and up to date.
As soon as a branch, merged with origin/main, passes the smoke test (see Testing), its worker pushes it to main.
The full suite runs only at rare intervals, never per branch or batch.

The `warp` binary (the Rust harness) is built by cargo; the warp compiler it runs by `warp rebuild`:
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo build --offline          # Offline mode (uses the local registry cache)
warp rebuild pure              # warp.wasm from lib/ by the seed
scripts/promote-self.sh        # warp.wasm -> warp-self.wasm once it compiles itself (`warp rebuild self`); never fails
```


## To-dos
This project keeps its to-dos on the board https://github.com/users/pannous/projects/1 (columns Now/Next/Soon/Later/Done),
every issue you encounter goes in with `todo add "…"` (~/dev/bin/todo, column Next) todo.md as fallback
`todo list`, `todo move <card> <column>`, `todo done <card> [commit]` (never delete a card). 
Picking a card is `todo take <card> <session name>` The branch will and must be named identical to the card. 

When encountering a TODO without corresponding card in my main checkout ~/dev/angles/warp/lib/ Create a card for that to do and mark the card name behind the todo and commit 

When implementing gaps that the old Rust legacy implementation had, but are not part of the pure WARP implementation yet, do not resort to blindly copying old spaghetti code. Instead, create a clean, idiomatic new implementation from first principles. 


## Pre-approved plans
Every worker gets a plan from Supervisor, and everything in it is pre-approved: implement it without asking.
A good idea of your own beyond the plan may be implemented too, as long as it is undoable (its own commits on a branch).
Main moved again in the meantime?
AVOID full rebuild for small changes! If the minimal smoke test (Compiling Hello World) passes on the branch merged
with origin/main, the worker pushes it to main itself and closes the card with `todo done <card> <commit>`
(notes/agents/common.md "Done"). The merge commit message MUST include a summary of the changes.

### Testing

Several agent sessions share this Mac and the full suite is slow, so run the cheapest check that covers the change
(roles: notes/roles.md):
- Smoke test (minutes), every branch: `scripts/rebuild.sh pure <worktree>`, then `WARP_LIB=<worktree>/lib`
  `warp samples/hello/hello.warp` prints Hello.
- Self test, only for parser/compiler changes (core/): `scripts/self-check.sh <worktree>`, the compiler compiles
  itself and that build prints Hello. For core/ changes smoke + self test is the whole check: no test folders.
- Changes outside core/: the smoke test is enough to merge. `warp test <folders>` of the few test files a lib/
  feature touches is feedback for the worker, run in the background, never a merge condition.
- Full suite (`warp test`): only the Integrator, at rare intervals (nightly, before a release), never per branch.
We always want the very quick feedback loop and quick merges: anything slower than smoke + self test runs in the
background (`run_in_background`) and never holds a merge; what it finds is fixed forward on a card.
Workers merge origin/main into their branch, run the smoke test on that merge and push it to main themselves; nobody
repeats their test. The Integrator runs the slower tiers in the background on main and sends what fails back to the
pusher's card, who fixes it forward.

Warp tests run locally with the shared binary, through the warp compiler (src/compiler/pure_tests.rs,
notes/is_tests.md):
```bash
warp test                      # all of test/: each *.pairs.warp pair, each is! line of test_*.warp, via warp.wasm
warp test test/numbers         # only these folders; --self: via `warp self`
```

### Test File Locations

Tests are in `test/<topic>/` folders. 
Warp tests are `test_*.pairs.warp` (code ⇥ expected, one a line)
Folder plan and condensing rules: notes/tests_layout.md notes/is_tests.md 

## Development Workflow

Always choose the most elegant option, never the quick hack: picture how the most beautiful, most general code would
look and build that (notes/agents/common.md Principles).

0. **Check state** - git log, git status
1. Develop new features preferably in library /lib/
2. Only in rare cases touch core/parser.warp or emitter.warp 
   /Users/me/dev/bin/warp rebuild pure`, then `… warp <program>` 
3. **Experiment** - quick throwaway programs go in the session scratchpad or `data/<topic>/`, never in the repo
4. **Add tests** - Create or update clean short functional tests in `test/`: every kept test lives there
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

2. Kind encoding (`src/type_kinds.rs` is the harness's copy for reading results back):

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
   - List/Block: `data = $list` vector (length, items array)
   - TypeDef: `data = name node`, `value = body node`
   - True/False: a bool, the Int 1/0 marked BOOL_INFO above the kind bits (kind = BOOL_KIND); prints yes/no (notes/bool_type.md)
   - Meta is dropped during emission; Error currently emits the inner node
   - Data nodes currently serialize as `Symbol(type_name)` in the emitter

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
/Volumes/Worktrees/integrate and integrate2 are the Integrator's (tools: scripts/integrate/, notes/agents/integrator.md).
Create worktrees with `cowtree add <path> -b <branch> [<commit>]` 
Before removing a worktree: `git status --short -uall` (untracked files are hidden on this Mac) + files changed in 2 h.


## Folders
- `test/` = every kept test (test/<topic>/, test/gate/ for the gate); there is no probes/ folder any more
- `data/` = kept but ignored: logs, test result dumps, patches, json/txt outputs, benchmark data, agent logs (`data/<topic>/`).

Workers: before and after each task, run `git status` and the tests that cover your change.
Do NOT run the full suite: only the Integrator runs it, at rare intervals (Testing above).
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