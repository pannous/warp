# Library zero-bake (card library-removal-core; folds in wasm-module-linking; decision library-zero-bake)

The goal: the core binary bakes the library zero times. No LIBRARY_FILES, no LIBRARY_PARSES. Each module publishes
its signatures, and the compiler loads it as compiled wasm when a program uses it.

## Assessment (main c0a082, 2026-11-05)

warp.wasm is 5.05 MB, and 3.04 MB of it (60%) is baked library tables:

| table | bytes | what reads it |
|---|---|---|
| LIBRARY_FILES | 1 106 271 | everything below; the text of all of lib/ (comments blanked) |
| LIBRARY_PARSES | 1 761 151 | slice_parse (reached prelude/module slices), source_parse (alias table, rules) |
| LIBRARY_SLICES | 98 339 | definition_slices of bundled text |
| LIBRARY_RULES | 70 568 | module_rules_of: rewrite rules of modules and of extensions/sugar |
| STD_RULE_NEEDS | 1 639 | which module's rules a program's words need |
| BLOCK_HEADS | 137 | parser: words taking a block last |

LIBRARY_FILES also bundles lib/extensions, the link to the compiler's own extensions/: 487 KB of its 1.1 MB
text, plus their parses. The compiler needs only extensions/sugar's rules (45 KB) from that folder.

These compile-time uses need library source or parses today:
1. **Reached definitions (the prelude).** with_reached parses each slice a program reaches and compiles it with the
   program. The prelude is never linked ("every program's own"). 108 of its 289 definitions have untyped
   parameters, which take the kind of their first call (functions.warp widen_parameter). That per-program
   specialization is the only "generic" in the library: there are no templates, `inline` or macro words.
2. **Linked modules.** declared_only takes each word's head from the slice parse, linked_slices and linked_needs
   take classes and globals, and the program runs the module's global initializers. The analyzer already takes
   kinds from the module's warp.module section (declared_linked).
3. **Rewrite rules.** There are 123 `rewrite` lines in top-level modules and 295 in extensions/sugar.
   module_rules_of reads the baked packed rules when the file text matches, and STD_RULE_NEEDS says which
   module's rules a program needs.
4. **Parse-time tables, needed before any `use` is known.** These are BLOCK_HEADS, the alias table
   (aliases.warp, parsed through LIBRARY_PARSES), 65 `infix operator` declarations, and 87 `alias` lines
   (module_aliases(source)).
5. **Scans of all of LIBRARY_FILES at every compile.** declarations_scanned finds global words, classes and tags
   by module, for implicit uses. with_library_host_words finds the `from host` lines of 9 files.
   type_module_providers and bundled_slices find other words by module.
6. **Small scattered readers.** These are protocols.warp module_words, foreign_types, sugar_module_names (which
   lists sugar files from the bundle), write_modules (calls_c), and the linkability test
   `bundled == file_text`.
7. **The type checker.** It needs nothing beyond 1 and 2: library words reach it as parsed slices or declared heads.

## Plan

**What a module publishes.** modules/<name>.wasm gets a `warp.signature` custom section in warp notation,
beside warp.module. It holds:
- each exported word: its arity variants, parameter names and types, result type, and whether a block comes last;
- classes: fields with types, method heads, parents;
- globals: name and kind;
- aliases, operator rows (spelling, precedence, function), foreign types, host lines;
- its rewrite rules, packed as LIBRARY_RULES packs them today;
- the hash of the source it was built from.

The module runs its own global initializers (a start function), so a program no longer copies them.

**One index for parse time.** `warp rebuild` writes modules/index.warp beside words.tsv, merging into it. It
holds the union of what has to be known before uses are resolved: block heads, operator rows, the alias table,
global words, classes and tags by module, host words, and rule heads by module (today's STD_RULE_NEEDS). The
compiler reads it once per compile, as it reads words.tsv and runtime.wasm today.

**How rules travel.** As data, never as wasm. A module's rules are in its signature section and load when the
program uses the module, or when the index says a rule head appears in the program. The sugar rules
(extensions/sugar) belong to the compiler, so they stay compiled into it as packed rules (45 KB of source, much
less packed), and their texts and parses are no longer bundled.

**Where modules are found.**
- Compile time: beside runtime.wasm, in modules/, as now. For the native warp that is ~/dev/bin or the worktree's
  lib/. For the playground it is warp-lib/modules/, fetched (build.sh already copies modules/*.wasm).
- Run time: first the emitter splices the module in, as it does now. Later the host links the module instance at
  load time (wasm-module-linking): wasmtime's linker in driver.warp, and WebAssembly.instantiate imports in the
  playground. That deletes about 170 emitter lines.
- A module whose source hash differs from its signature's (a WARP_LIB edit), or one that doesn't prebuild (the
  C-calling ones), compiles from lib/ source read from disk. That path stays and is the only one that parses a
  library file.

**Seed and self build.** The order is unchanged: seed, then warp.wasm, then `warp runtime`, `warp modules` and
the index (each module compiled from disk source), then the compiler compiles itself against those modules. A
compiler that bakes nothing reads lib/ from disk only through the from-source path. That path already exists for
WARP_LIB edits and older seeds, so the seed needs no new ability. The own build must link its own uses (math,
file, os …) from modules/, or keep compiling them from source. The second is simpler for the first landings.

**The prelude** is the real cost. Linking it means its 108 untyped words become Node-parameter words, so every
`max(a, b)` of ints boxes. Before linking it, its hot untyped words get typed overloads (int/float/text, already
supported: typed overloads and arity variants). Until then the prelude stays reached from source, read from
lib/prelude.warp on disk (one file, about 3 ms to parse per compile, to be measured), not baked.

**Order of landings, each small and measured:**
1. Stop bundling lib/extensions except sugar (UNBUNDLED_FOLDERS). This saves about 0.5 MB of text and
   proportionally of parses. No behavior changes.
2. modules/index.warp. BLOCK_HEADS, STD_RULE_NEEDS, declarations_scanned, the host-word scan, the alias and
   operator tables and sugar_module_names read it instead of LIBRARY_FILES.
3. The warp.signature section. declared_only, linked_slices, linked_needs, module_rules_of, protocols'
   module_words and foreign_types read it. Linkability is a hash compare instead of `bundled == file_text`.
4. The prelude as above: typed overloads, then either linked or read from disk unbaked. The other remaining
   readers of LIBRARY_FILES/PARSES/SLICES/RULES move to the disk path, and the tables go: warp.wasm loses about
   3 MB.
5. Load-time linking (wasm-module-linking) once 3 holds: the host links the instances, and the emitter's splice
   and rebase code goes.

node-wasm-type comes after 5.

## The module interface (L3 writes it, L5 links by it; a draft to agree before either lands)
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
  dispatcher calls it by number, as the splice path numbers `<module>·<target>` today.
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
**Class group (agreed with L5, point 3).** Instance and a Class descriptor struct join the fixed gc_names types, so
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

`warp.module` (the binary layout: kinds, closures, relocations for the splicing path) stays until L5 removes the
splice. The parse-time union of all modules' `aliases`, `operators`, `block_last` words, `words`→module, `classes`
and `tags` is gathered from these sections or kept in an index file; L2 measures which (question (a)).

## Load-time linking: the probe (data/wasm-linking/: L.wat, P.wat, run.mjs)
Dynamic linking of separately compiled wasm GC modules works in wasmtime 49.0.2 (`--preload`, with gc,
function-references and exceptions) and in Node 26 / V8 14.6 (no flags). All 17 checks pass:
- $Node passes both ways.
- A $MetaNode subtype that only P defines passes into L and is cast there.
- Shared mutable and immutable globals work.
- funcref and typed tables work with call_ref.
- Exception tags with i32 or $Node payloads are caught across modules.

The emitter must guarantee these for L3–L5:
- Every shared type has the same rec-group shape in every module. A reordered or split group fails at link with
  "incompatible import type".
- $Node stays open; $MetaNode stays final.
- A library signature never names a program's own class types (module_layout.warp:62 type_entries differ per
  program). Those pass as $Node/$Instance/anyref and are cast inside.
- The warp_error tag is declared once (in the runtime or the library) and imported everywhere.

## Progress (warp-sugar)
- L1 landed 6f3a15e394: lib/extensions unbundled but sugar/, uncertain/ (and rules/modules/ with 1b); warp.wasm 5.05 -> 4.22 MB.
- 1b (card compile-time-rules): 21 rules ($program / where program, paint, shader_of, gpu_render, law, typed json)
  in extensions/rules/modules/<module>.warp, taken with their module (rewrites.warp module_rule_paths,
  rules_of_module); lib/rule_words.warp, lib/wgsl.warp -> extensions/rules/, still used as modules
  (uses.warp std_module_path finds extensions/rules/<name>.warp; an include retyped the emitter: card
  makes-compiling). warp.wasm 4.22 -> 4.01 MB.
- L2 next: modules/index.warp written by `warp rebuild` (write_modules beside words.tsv): BLOCK_HEADS
  (parser.warp block_heads_in), STD_RULE_NEEDS (rewrites.warp scanned_rule_needs), declarations_scanned
  (parser.warp ~1750: global words/classes/tags by module), with_library_host_words (builtin_signatures.warp:53),
  alias table (parser.warp ALIAS_TABLE, alias_table_tree), sugar_module_names; the readers read the index, the
  scans of LIBRARY_FILES go. Question (a): measure gathering the union from the modules' sections instead.
- L2 decided (a): index.warp is a cache of the union, derived by `warp modules` from the signatures in the same step,
  each module's source hash beside it (reading the 45 modules, 15 MB, at each compile was too dear). L2a: block heads,
  rule needs, words/classes/tags by module, host words; L2b: alias and operator tables, sugar_module_names.
- L2a landed (aa5cbdf430): modules/index.tsv (library.warp LIBRARY_INDEX, library_index_section): sections block,
  global, class, tag, word (absorbs words.tsv), needs, host, hash (djb2 of each module's file, `<length>-<hash>`: the
  host's sha256 takes no module-long text from the compiler). Written first by the runtime and modules steps
  (write_library_index), so no step reads an earlier build's; a missing index is scanned off lib/ per section. tsv, not
  warp notation: `warp parse` of the same index as a 40 KB .warp is 0.47 s against 0.31 s for a whole hello compile,
  the tsv split ~1 ms. BLOCK_HEADS and STD_RULE_NEEDS no longer baked. The seed compiles compiler.warp with its own
  bundled pre-1b rule_words, so word_index/word_index_file stay in prelude_words.warp until the next seed (card
  seed-compiles).
- L3: warp.signature goes right after the wasm header (a custom section may be first), so reading a signature costs
  a few KB, not the whole module.
- Measured before L3 (2026-11-05): the prelude's untyped parameters compiled as nodes, as a linked prelude would
  take them (a switch in emitter/functions.warp call_values, never committed; data/boxed-measurement.patch of the
  worktree). The emitter types a function by its first call's argument kinds (call_values), not the analyzer's slots.
  Instructions retired, compile and run:
  - hello: 5.148 G either way.
  - compiler.warp, built both ways, compiling compiler.warp: 3006.0 G vs 3006.2 G (+0.008%); the build +460 bytes.
  - a numeric loop, 200 000 rounds of fourth_root, round_to, pixel_channel and is_digit: 9.59 G vs 10.51 G (+9.5%;
    about +20% of its run, the compile being ~5.1 G).
  So L4 stays small: typed overloads only for the prelude's numeric words a hot loop calls (fourth_root, round_to,
  pixel_channel, is_digit and the like, random_*, mean); the compiler itself doesn't feel it.
- L5 (wasm-module-linking): points 1 (closure funcref), 3 (Instance), 2 (class descriptors) landed; L5c programs and
  modules import their modules' words (notes/runtime_linking.md "Load-time linking"). warp.module's content moves
  into warp.signature with L5e (agreed with warp-sugar).
