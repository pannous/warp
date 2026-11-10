# Library zero-bake (card library-removal-core; folds in wasm-module-linking; decision library-zero-bake)

The goal: the core binary bakes the library zero times. No LIBRARY_FILES, no LIBRARY_PARSES. Each module publishes
its signatures, and the compiler loads it as compiled wasm when a program uses it.

## core-takes-what-it-uses (card baked-lib-split)
The compiler `use`s no lib module and module_source names only
core's and extensions' files: core/host_io.warp, core/integers.warp, core/runtime/case_tables.warp (lib/ links to them),
the typed builtin lines in core/analysis/builtins.warp and math_builtins.warp, the data files errors, warnings and
hints.data and core/passes/modes.data; prelude, math and c are read from disk when a compile first asks.
P3: the prelude's core part, core/prelude_core.warp: the 16 prelude words the compiler really calls (its call graph
from main, not its name section: ~45 of the 64 prelude functions in warp.wasm are reached only by name, a map key
`random:`, a local `emitted`, and never called) and the 12 aliases its code writes (`.keys`, `.has`, `.index_of(x)`,
`.length` …; alias lines of an included file don't apply, so they must be prelude text). One reader joins the parts:
std_module_text("prelude") is the core part, baked by module_source, then lib/prelude.warp; every read of a standard
module's text goes through it (prelude.wasm's signature and hash, the index spans, prelude_text's readers: aliases,
prelude_definitions, implicit uses), prelude_files() gives both files for positions, and the prelude module's stub
uses the core file before `use prelude`.
No operator in the core part while compiler.warp includes it: in the self build that operator and the prelude's same
one are two overloads the dispatch cannot tell apart (a function replaces, an operator overloads), so the lexer's
`(n + 1) times [n]` became a comprehension and `times` stays in lib/.
With lib/prelude.warp emptied
the compile stops in lib/text.warp's indent, which the compiler uses implicitly by name (its field and local
`indent`) and which needs `times`. scripts/compiler-inputs.sh --code-files lists no lib/ file. Left: the reach by name
still compiles dead prelude words, and lib/text.warp's, in from lib/ (card baked-lib-split note).
Data on demand (cards webidl-on-demand, entities-on-demand): lib/web.webidl and lib/entities.tsv are
read through library_data (core/passes/library.warp, loud when missing) the first time a program reaches the page's js
or writes an entity outside a text (`\:alpha`, or `\alpha` for its refusal; `"\:alpha"` reads it at the text escape).

## State
Nothing of lib/ is baked into warp.wasm. The sugar's rules (extensions/sugar), packed by file, are
modules/sugar_rules.packed, written beside the index (library.warp write_library_index). Everything else
of lib/ is read from disk once a compile (library.warp library_text) or linked as compiled wasm: every program links
modules/prelude.wasm and the modules it uses, each declared from its warp.signature. A prebuilt module is checked
against its source by `<length>-<sha256>` (library.warp source_hash).

## Steps (L1-L5, all landed)
- L1: lib/extensions is not bundled.
- L2: modules/index.tsv (library.warp LIBRARY_INDEX), a cache of what a compile has to know before uses are resolved,
  written by `warp rebuild` beside the modules (write_library_index), one line `<section>\t<key>\t<value>` of the
  sections INDEX_SECTIONS; a missing index is scanned off lib/ per section. tsv, not warp notation: `warp parse` of
  the same index as a 40 KB .warp is 0.47 s against 0.31 s for a whole hello compile, the tsv split ~1 ms.
- L3: warp.signature (core/passes/signature.warp) goes right after the wasm header (a custom section may be first),
  so reading a signature costs a few KB, not the whole module. pack() of {source hash, uses, slices [name, head line,
  parse], rules, foreign}; unpacked, never parsed. linked_libraries links a module when the hash is its file's, else
  warns one line why (no signature, or lib/<m>.warp differs) and compiles it from source. A linked module's words
  reach the program as its signature's parses (prelude_words.warp with_reached), its rules from the signature: no
  text of it is read. `warp signature <m>` shows one as warp text.
- L4: the compiler bakes no library. lib/ is read from disk, once a file a compile (library.warp library_text,
  file_source). What a bake had hidden, each found by scripts/profile.sh and moved:
  - cutting the prelude and the type modules into definitions (~70 ms): their spans are the index's section `slice`
    (prelude_words.warp spans_of), used while the text is the file the index was written of.
  - a used module parsed whole for its `use` lines: use lines read off the lines a use may start (parser.warp
    use_candidates), a linked module's uses off its signature (modules_used_by), its words off the index.
  - parses of the prelude's reached definitions taken from its signature cost more than parsing the few reached.
  Step 2, the prelude linked: prelude.wasm exports every word; every program links it (linked_libraries names it
  first); a host word the driver doesn't answer finds its library at its first call; runtime.wasm carries
  warp.fingerprint; a linked module's globals are imported, never brought in and re-run; link precedence program >
  used module > prelude (`by_precedence`); only a refusal marked by link_refusal retries from source. The prelude's
  hot numeric words have float/codepoint overloads: its untyped parameters, linked, would box.
- L5 (wasm-module-linking): programs and modules import their modules' words (notes/runtime_linking.md "Load-time
  linking"); the playground links them too; a module's image lives in its warp.signature.

## The module interface (L3 writes it, L5 links by it)
A standard module `<name>` is modules/<name>.wasm, compiled once behind runtime.wasm (`warp module <name>`).

**Imports.**
- Everything of the runtime comes from module `warp_runtime` under the runtime's warp names, as a separate program
  module imports it today (`warp build`'s x.mod.wasm): its words, its globals, `memory`, the error tag `warp_error`
  and the table `hooks`.
- Another standard module's word comes from module `warp/<other>`.

**Types.** canonical_type_entries() first, each fixed GC type in its own singleton group in the runtime's order,
so every module's shared types are the same. Then the module's own class structs (one rec group) and its function
types. A library's own classes may cross the boundary typed. They sit in their own rec group, whose exact shape
(field order and types, supertypes, finality) warp.signature carries. Every importer emits that group identically,
so the two canonicalize to one type. A program class extending one goes in a later group, with the library struct
as its supertype. Only a program's own classes never appear in a module's signature: they cross as $Node,
$Instance or anyref and are cast inside.

**Exports.**
- Every word under its written name. A word with several arity variants exports each as `<name>/<arity>`. The plain
  name is the one variant a program without an arity calls, as linked_variant chooses today.
- Its globals under their names.
- A start function `warp/start` that runs the module's global initializers. The host calls it once, after linking
  and before the program's main.
- `export f(…)` words (host callbacks) keep their names.
- Each lifted lambda as `warp/closure/<n>`, numbered as the module's closure list. The program's closure
  dispatcher calls it by number.
- Each class method as `Class.method`, which the program's text()/== hook dispatchers call.

**`warp.signature`** is a custom section holding one warp-notation value, readable by the compiler's own parser:
```
{ source: "<sha-256 of lib/<name>.warp>", uses: [other modules],
  words: { name: [ { parameters: [[name, type, default?, rest?]…], result: type, block_last: yes?, export: name } … ] },
  classes: { Name: { parent: P?, fields: [[name, type]…], methods: { name: [variant…] } } },
  globals: { name: type }, aliases: { alias: word }, operators: { "±": { power: [140, 141], function: name } },
  foreign_types: { … }, host: { word: [parameters, result] }, rules: <packed rules>, tags: [ … ],
  closures: [[n, target, captured, shown]…] }
```
**Class group.** Instance and a Class descriptor struct join the fixed gc_names types, so
there is no prelude class group. A module's classes form one rec group, directly after the fixed types and the
function types its fields name. Each class is `sub Instance` or `sub` its parent, in classes_parents_first order
(depth 0 first, then declaration order).
- warp.signature `class_group`: a list of [name, parent ("" for Instance, "module/Class" for another module's
  class, whose group comes earlier), the struct's definition bytes as the module wrote them]. Inside those bytes, a
  type reference is tagged: [0, gc_names index] for a fixed type, [1, index from the group's start] for a class of
  the group, [2, the function signature written out] for a function type. An importer rebuilds the bytes against
  its own type section and emits the group byte for byte.
- Class identity goes through a descriptor global exported as `class/<Name>`, never the struct type: two groups of
  one shape canonicalize to one type.
- `classes` (fields with warp types, method heads) stays beside it for the analyzer.

`warp.module` (the binary layout: kinds, closures) stays beside it. The parse-time union of all modules' `aliases`,
`operators`, `block_last` words, `words`→module, `classes` and `tags` is kept in the index file (L2).

The emitter must guarantee for linking:
- Every shared type has the same rec-group shape in every module. A reordered or split group fails at link with
  "incompatible import type".
- $Node stays open; $MetaNode stays final.
- A library signature never names a program's own class types (module_layout.warp type_entries differ per
  program). Those pass as $Node/$Instance/anyref and are cast inside.
- The warp_error tag is declared once (in the runtime or the library) and imported everywhere.
