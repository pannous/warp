## Project Overview

This codebase is yours: what it's for and how it's built are your call, and you'll keep building on it.
Own it like its long-term maintainer within the user's taste and philosophy: the language as the wiki specifies it and
the tests pin it, the principles (notes/agents/common.md: elegance first, first principles) and the three layers
below. Where they say nothing, decide as that maintainer would, and leave the code better than you found it. When a
rule blocks a clearly better design, say so with the reason (to warp-interviewer) instead of quietly obeying or
breaking it. notes/decisions.md is only an inbox of recent decisions on their way into the wiki: grep it, never read
it whole.

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
- the native warp (core/native/warp.warp, wasmtime through its C API) to run them; the browser build in web/playground
  runs them in JavaScript

# self hosted

warp began as a Rust implementation; Since 2026-10-29 the compiler is COMPLETELY written in warp (core/, notes/self_hosting.md) and compiles itself.
The old Rust compiler is the snapshot on branch `rust-bootstrap`; nothing of it is on main.
Most things are implemented as libraries (lib/) or compiler extensions (extensions/). Changes to the core
(core/) go through the interviewer. Ignore all old tasks about Rust features or 'both lanes'.
The seed, warp-seed.wasm beside the binary (~/dev/bin; no file of git, published on GitHub pre-releases seed-<date>), is
the seed compiler; it compiles compiler.warp to warp.wasm, which promotes itself to warp-self.wasm when it compiles
itself once more (no fixed point needed).

## Three layers (decision core-is-self: keeps the core from bloating)
- **core/**: only what the compiler needs to compile itself. Aim: about 30% of the compiler, and shrinking.
- **extensions/**: every other language feature, baked in through one static hook table.
- **lib/**: modules programs import. A new feature goes into an extension or a library, never into the core;
  the language is never simplified to shrink the core (scripts/vitals.py measures the split).
- Everything else has its own place: demos and examples in samples/, tests in test/, tools in scripts/. Never a
  demo, sample or test fixture in core/, extensions/ or lib/ (an aquarium demo is a sample, not an extension).
- The compiler `use`s no lib/ module: what it needs of one lives in core/ (decision core-takes-what-it-uses).
- Before adding a file or a function, ask where it belongs; when unsure, ask the Supervisor. The Integrator refuses
  a landing that puts something in the wrong layer (notes/agents/integrator.md "Layers").

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

`core/compiler.warp` is the whole compiler as one warp program, source text in, wasm bytes out:
parsing/ → passes/ (the lowering passes; sugar as rewrite rules in extensions/sugar/) → analysis/ → emitter/,
whose runtime words are warp too (runtime.warp, runtime/). core/node.warp declares the Node enum, core/commands/ the
one command table.
- `warp <file|code>` (and `warp run`): runs warp.wasm, compiler.warp compiled by the seed (warp-seed.wasm beside the
  binary; scripts/refresh-seed.sh refreshes and publishes it)
- `warp self <file|code>`: runs warp-self.wasm, a copy of warp.wasm promoted when warp.wasm compiles
  compiler.warp once more (a fixpoint, stage N+1 == N, is only reported)
- `warp rebuild`: builds warp.wasm anew from lib/ (nothing rebuilds it automatically); the gate needs only warp.wasm.
  `warp rebuild self` runs that self test and promotes; `scripts/promote-self.sh [--background] [<tree>]` does that
  every once in a while, in parallel, and never fails its caller. A worktree's compilers live in <worktree>/lib/
  itself (`WARP_LIB=<worktree>/lib`), main's beside the binary
- A form outside the emitter's core is refused loudly (`outside the core: …`)

### Node AST (the enum in `core/node.warp`)

The central data structure is `Node`, an enum representing all AST node types:

- **Empty, True, False, Number, Char, Text, Symbol** - Atomic values
- **Key** - Binary structures (Pair, Tag), with an `Op`
- **List** - Collection of nodes with a `Bracket` (curly, square, round …) and a `Separator`
- **Data** - Generic container for an arbitrary value (`Data(value: any)`)
- **Meta** - Node wrapper that adds metadata (comments, line/column positions)
- **Error, Type** - an error; a type definition (name + body)

### Warp parser (`core/parsing/parser.warp`)

Recursive descent parser that converts text input to Node AST:

- Tracks position (line/column) for all nodes
- Handles comments (`//` line and `/* */` block) attached as metadata
- Parses literals (numbers, strings, symbols), groups ((), [], {}), and structures

### Emitter (`core/emitter/emitter.warp`)

Generates the WASM GC bytecode and its runtime words (texts, lists, maps, unbounded ints, exact numbers, equality,
WASI, FFI) in warp; the value layout and the `Kind` enum are in notes/self_hosting.md "Value layout".

### Native warp (`core/native/`)

warp.warp is the `warp` command, an `@primitive` warp program built to a native executable (wasm2c + cc); driver.warp
runs warp.wasm and the programs through wasmtime's C API and answers their host words over libc; tests.warp is
`warp test`. Library host words (graphics, sound, web) are libraries in lib/, not the core binary
(notes/roadmap_native_harness.md).

## Build and Test Commands

How agents work together (baseline, cards, plans, worktrees, which test tier when, handoff to the integrator) is
notes/agents/common.md, the one rule book of the fleet; this file only describes the project.

The `warp` binary is the native warp, built by warp itself; the compiler it runs by `warp rebuild`.
After each push scripts/integrate/after-push.sh installs both beside ~/dev/bin/warp:
```bash
warp rebuild                              # warp.wasm from lib/ by the seed, then the native warp; `warp rebuild pure`: the compilers alone
scripts/promote-self.sh                   # warp.wasm -> warp-self.wasm once it compiles itself; never fails
```


## To-dos
This project keeps its to-dos on the board https://github.com/users/pannous/projects/1 (columns Now/Next/Soon/Later/Done),
every issue you encounter goes in with `todo add "…"` (~/dev/bin/todo, column Next), todo.md as fallback; how cards
are taken, branched and closed: notes/agents/common.md "Cards".

### Testing

The checks are tiers, cheapest first: smoke (`scripts/smoke.sh <worktree>`, Hello), self test for core/
(`scripts/self-check.sh <worktree>`), a few test folders as feedback, the full suite rarely; who runs which:
notes/agents/common.md "Testing" and "Done".
Warp tests run locally with the shared binary, through the warp compiler (core/native/tests.warp, notes/is_tests.md):
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
2. Only in rare cases touch core/ (parser, emitter …): then `warp rebuild`, then `warp <program>`
3. **Experiment** - quick throwaway programs go in the session scratchpad or `data/<topic>/`, never in the repo
4. **Add tests** - Create or update clean short functional tests in `test/`: every kept test lives there
5. **Run tests** - `warp test <folders>`

## Serialization

=== Node Serialization Workflow (Current Spec) ===
node.warp via emitter.warp in notes/self_hosting.md "Value layout"

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
   Lists held as arrays: `$IntList`, `$FloatList`, `$NodeList` (length, items array), maps `$NodeMap`; a program that
   quotes code gets `$MetaNode`, a $Node subtype with line, column and comment. -->

2. Kind encoding:

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

4. Round-trip:

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

A test pins only behavior we want. Never write an expectation for a known flaw (eg .1+.2!=.3)

No tests of intermediates: parse tree shapes, node views, pass outputs, analyzer kind tables, wasm bytes, module or GC layouts, internal APIs.


When fixing a problem NEVER modify the test itself without consulting!
A test may be upgraded without asking when the code now does better: an expected error, refusal or "not supported
yet" becomes the working value, an ignored test that passes is un-ignored, a weaker assertion becomes the stronger one.

# Important
NEVER modify a README without the user's review and explicit confirmation.

## Folders
- `test/` = every kept test (test/<topic>/, test/gate/ for the gate); there is no probes/ folder any more
- `data/` = kept but ignored: logs, test result dumps, patches, json/txt outputs, benchmark data, agent logs (`data/<topic>/`).

<!-- ### Associated projects
~/Library/Application Support/Sublime Text 3/Packages/Warp
 -->
Remove worktrees and branches when done!