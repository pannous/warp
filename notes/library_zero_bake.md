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
