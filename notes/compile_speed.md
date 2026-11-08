# Compile speed of the warp compiler (card rebuild-pure-slowdown, 2026-10-30/31)

A rebuild grew from ~140 s (10-29) to 380–420 s (10-30). Measured on compiler.warp compiled by the compiler itself,
CPU seconds between phase markers:

| state | compile CPU |
|---|---|
| 10-30 evening | 607 s |
| method_calls module rule fixed (b15a98773e) | 285 s |
| with_expected_type_uses fixed (9b01b1bb2a) | 230 s |

## How to measure
- Profiling, first: `scripts/profile.sh [-s <seconds>] <warp words…>`, e.g. `scripts/profile.sh -s 90 rebuild pure`
  (WARP_LIB/WARP_SEED as for any run). The run gets WARP_PROFILE=perfmap (the native driver writes
  /tmp/perf-<pid>.map), macOS's sample takes its main thread's stacks, and the script prints each wasm function's own
  samples and those with its callees: no rebuild, no markers. `-r <pid>` reports a kept data/profile/<pid>.sample
  again. sample manages ~30 ms a sample on wasmtime's threads, so profile a minute or more. Its first find: 55% of a
  parse in alias_at → longest_spelled_at, 39% in operator_at → spelled_operator_at (2026-10-31).
  Cost (2026-10-31, bench.warp on parser.warp, ~94 s CPU): WARP_PROFILE=perfmap alone changes nothing (93.3 vs 93.5 s
  CPU); `sample` attached for the whole run keeps CPU the same (94.0 s) and adds ~10–15% wall (108 vs 94–95 s) while
  it pauses the thread. On that run it took 76 k main-thread samples in 108 s, ~1.4 ms each.
- Phase markers (card phase-markers): `timed(label)` (core/passes/passes.warp) prints `§T <milliseconds> <label>`
  when WARP_TIMES is set, silent otherwise, where a phase starts: compiler.warp's compile (implicit uses,
  checked_program, emit, emit done), with_used's steps, with_uses' (used files, module texts, each file's parse), each
  baked table's computing, each pure pass by name. A build run with WARP_TIMES=1 keeps them in its record's build.log
  (notes/compiler_rebuilds.md "Build records"; stage two's record for compiler.warp compiled by the tree's own
  compiler); `scripts/phase-times.py <build.log>…` sums the wall spans per label, biggest first, up to `emit done`.
  CPU and hotspots are the record's time and profile.txt: never build again to measure.
- A synthetic program at 100/200/400/1600 statements shows superlinear terms quickly with the installed warp
  (`/usr/bin/time -p`, user CPU): `def gN(xs: list): int := xs.f(N) + 1` against `f(N) + 1`.
- Rules in extensions/sugar/ are not baked: switching one off in a worktree's lib (WARP_LIB) needs no rebuild, so a rule's
  cost is bisected in seconds.
- The wasmtime CLI profiler trapped on the compiler (card wasmtime-cli-refusal); the markers need no profiler.

## What it was
1. extensions/sugar/method_calls.warp `$m.$f($a...) … $module = module_named($m, $program)` (e3ab37c931): `$m is not
   value` passes for a parameter (rule_scope is the program's), so module_named ran at nearly every `xs.f()`, and each
   call walked the whole program (names_bound_in) and read a module file. Quadratic: late_sugar 300 → 5.6 s.
   Fix: names_bound_in_kept (one-entry same_node cache) and used_module_text reads each module once.
2. with_expected_type_uses (`sleep(3ms)` finding units.warp) parsed a called word's definition text for every argument
   of every call (2956 calls): 54 → 0.2 s. Fix: heads parsed once per word, only for arguments naming an unknown word.
3. Not it: indexing builtin lines by name (lines_of filtered ~1000 lines per lookup) saved only 3 s; not landed.

## The lesson
A rule word or pass helper that reads the whole program (`$program`, names_bound_in, file reads, parses) must compute
once per program, never per node or call site. Cheap `where` guards don't save a rule whose guard passes broadly.

4. called_only (warp-quality, 94a0dfeea8): each called word scanned all statements, the pending list was copied per
   step: 24 → 1.1 s.
5. The parser's per-token cost (card compiler-parse-timing): every token tried every operator alias in turn
   (alias_at → longest_spelled_at, 55% of a parse), and each operand found superscripts by walking the superscript
   table text (position_in, ~25% of what was left). Fix: the aliases, truth glyphs, negated spellings and test words
   indexed by first character once (parser.warp by_first_character, starting_here), the superscript tables as maps
   (character_index). The 117 core files under 60 KB parse to the same trees, 30.4 → 16.0 s CPU; compiler.warp
   compiled by warp.wasm 616 → 556 s CPU, back to back on a loaded machine.
   Found with warp-quality's WARP_PROFILE=perfmap (scripts/profile.sh) on `warp parse-code "<2000 lines>" out.bin`;
   a parser timing needs no `warp parse` (its print is quadratic) and no compiler build per probe.

## Left (main 42a19df29a, stage two 2efb9b98f2f3eca9: 166 s CPU, 580 MB; was 820 s this morning)
No single fix of 5 s or more is left without a design change (cards):
- emit 46 s (27%): wasmtime's is_subtype libcall and GC (warp-quality, card emit-wasmtime).
- nodes_in lists (card builds-list): keep_nodes 13% of the compile across ~60 callers, none over 8%; half GC.
- rule tries (card sugar): rewritten_program 8.7 s, sugar 9.7 s, each kept rule tried at each node.
- LIBRARY_PARSES 7 s per compiler build (card library-parses-cost): the bake parses every std module anew.
- then lambdas 7.5 s, arity_overloads 6.2 s (mostly nodes_in), first/late/iteration/comprehension sugar 4–5 s each.
`warp parse` printing a big tree is quadratic (card parse-print-quadratic); the parse itself isn't.

## Three walks (2026-10-31, found on the def-first measurement)
Self-check's stage two (compiler.warp compiled by the tree's own warp.wasm), both on the GC growth fix (1cd7a5c2e3:
wasmtime grew a copying heap only when its active half was full; that run was 831 -> 227 s CPU and ~80% GC before),
records /Volumes/Build/compilers/{2cd7795a6e4464f8,044362cbf97a7d00}/stage-two.record: CPU 214.2 -> 194.9 s (-9.0%),
peak RSS 942 -> 625 MB (-34%), all from two walks:
- lambdas' captures_assigned_later gathered, for every statement, the assignments of all later ones (quadratic, 10% of
  the compile): each name's last assignment is gathered once (lambdas 18.8 -> 7.2 s).
- each sugar step listed the program's nodes five times for its rule lookups, now once (sugar steps, type_tests -8 s).
- index_positions copied every name the program assigns (most counting nothing) into each function's scope: a scope
  now passes on only the names that count (a function's own `i = t.find(x)` counts, its local): index_positions
  17.9 -> 1.4 s (stage two of 9a845e21fccdf599, on a loaded machine).
- type_tests and plain_enums copied the whole program's scope into each function: a function gets only the
  entries its own names ask for (scope.warp names_seen): type_tests 10.0 -> 0.9 s, plain_enums 10.7 -> 3.5 s; stage two
  166 s CPU, 580 MB (2efb9b98f2f3eca9).
def-first (every definition starting with `def`) gained nothing measurable and is parked on branch
function-definition-core; its two collection passes were not built (the collectors they'd replace are <0.5% each, the
definition gatherers ~1-1.7%; a table collected after with_used goes stale as later passes add, rename and lift
definitions; user: discuss later). Left: emit 44 s (22%), index_positions 18 s, plain_enums 9 s, LIBRARY_PARSES 7 s.

### Emit (~42 s; warp-quality 2026-10-31, profile.sh -f typed_wasm_module on rebuild pure, 39053 samples)
- 26% of emit is wasmtime's is_subtype libcall (StoreOpaque::is_subtype_cached): a `ref.test`/`ref.cast` whose
  types are not the same takes it, a failing test always. The runtime's number and node type tests do so on every
  call: node_type_test 24% with callees, ratio_node 27%, number_byte 20% (a value tested for a ratio, a box…).
  Fixed (card emit-wasmtime): wasmtime tests a final type by index equality, an open one of another type through the
  libcall; every class struct was open, so instance_place / instance_field_at tested each class in turn and
  is_class_instance the open $Instance (55% of the libcall). A class nobody extends now has a final struct and
  is_class_instance tests each class's: stage 3 (the compiler compiling itself through code its own emitter wrote)
  is_subtype 8.5% → 1.4%, 254.6 → 242.2 s CPU. The rest is the cast to the open $Node of a $MetaNode or an instance
  (node_equal, node_text). Measure an emitter change at stage 3: stage 2 runs the seed's code, and the store's
  warp-self.wasm is a copy of warp.wasm (data/emit-wasmtime/measure.sh).
- 19% is the copying collector (gc_alloc_raw → do_gc → collect_increment, CopyingHeap::forward/scan_field): a bigger
  initial GC heap (warp_config_gc_heap_initial_size_set) for the compiler may cut it, card wasmtime-copying-collector.
- The emitter's own top paths: compile_function → statement → effects → joint_kind → instance_of_fields →
  forwarded_kinds → no_field_unshared → call_values (60%), field·class_defaults 44%, emitter_kind 46%; one widening
  round only. By markers: 2975 functions, median 4.4 ms; a one-line function ~1.5 ms, a lambda ~0.3 ms.

## Marker pitfalls
- A marker must stand before the work it names: `f(label, computed)` prints after `computed` ran (timed_bake did).
- phase-times.py stops at a log's first `emit done`.
- The marks are printed (stdout): WARP_TIMES set for a test or gate run makes every case's output differ; set it only
  for the build or self-check measured.

## Hints and warnings cost nothing worth skipping (card hints-at-parse-cost, 2026-11-01)
WARP_HINTS=0 / --no-hints only filters hints when printing (core/compiler.warp said_at); the checks still run. Measured
on stage two's stored profile (record 1221d950b1ac6a95, 25 k samples, compiler.warp): diagnose, said_fact, rows,
rows_of, written_at, place_text together 0.1 %; dropped_value 0.2 %; source_checks/lints run only on the main file
(the used files go through checked_parse), so they don't show at all. The two hints (alias_word, implicit_use) only
say what the rewrite and use resolution do anyway. kind_changes (2.0 %) and main_globals (0.3 %) find errors too,
so they must run. A switch that skips the producers would save under 0.5 %: not built.

## Compile-cost drift after the parser moves (card compile-cost-drift, 2026-11-04)
Each landing's compiler compiling one fixed source (idioms 3f968216af, worktree drift-src), instructions retired, one
warm-up run discarded (notes/compiler_rebuilds.md "Build records"; data/drift/landings.sh, landings.times):
idioms 3.6496e12; 704bb24c11 (attributes, sum types leave the parser) 3.6939e12 (+1.2%); 41c1e0d036 (checked tries,
lints; a sibling branch) 3.7198e12 (+1.9%); 8225781320 (both) 3.7465e12 (+2.7%); dd2e1b5bcc, 27084eb53b, cf91fce346 flat.
- Joining every extension row once into one Extension row (extension.warp hooks(), landed 181f8a6500) took 1.86%
  back: hooks were looked up by field name in a map per call.
- What is left grows no function of its own: the profiles spread it over runtime words (is_class_instance,
  holds_entries, slot_free, node_equal, GC). WARP_TIMES put idioms vs 41c1's +1.9 s in "used modules" (wall, GC
  included); the checked_tries pass itself is 0.16 s.
- Suffix readers keyed by their first character (extension.warp read_suffix, one map get an operator round) took
  2.13% more: 3.7227e12 -> 3.6433e12, below idioms. Each round had asked every suffix reader in turn, and read_percent
  read a whole operator there (operator_at, aliases included), superscript_power scanned for raised terms: a hook on a
  hot path keyed by what starts its form costs one lookup, a list of hooks costs each one's own scan. The drift is gone.
- Not it: the map index's 64-slot scan. Home slots by first-name hash (data/drift/home-slot.patch) measured +0.08%,
  index_last already catches most lookups (card map-hash-index keeps the proper fix for its own sake).

## Typed nodes (card node-wasm-type, 2026-11-06, 5e2cf051bb)
A node is held as `(ref null $Node)`, no longer anyref: self compile 2582 G → 2351 G instructions (−9.0 %), hello −1 %,
units −0.6 %. wasmtime runs a `ref.cast` even when the value is statically of the target type (200 M casts of a
$MetaNode to $Node: 1.13 s, none 0.19 s), so the win came from dropping the casts, not from the type alone. Casts stay
only where an anyref enters (emitter `entered_node`): a list item (`item_read`), a $Node's data, a host word's node
result, a call_ref result. Host imports keep anyref; closure entries and class words return anyref (a class word's
place/set is an i31). A closure's item 0 is a $Closure: read by `closure_slot`, never through the node cast.
Each class struct is its own wasm type group: a module grouping two classes made another Instant type than a program
declaring only Instant (cast failure across modules).

## Hot spots after typed nodes (2026-11-06, profile data/profile/53103.sample)
- lambdas-definitions-index (a14808ff41): parameters_given_functions filtered every definition per call node; by
  name now: self compile −5.0 %.
- lexer-codepoints (9dec4062f4): code_only was asked of one source 4–5 times per parse (declared operators, type
  names, reached modules), each a walk of every character; kept per source now, and rules_reaching's picked a
  yes/no per rule: self compile −12.5 %, units −18 %, hello −5.4 %. The character compares were not the cost: a
  pre-check skipping non-delimiters and code points instead of the text measured the same.
  The kept map (lexer.warp code_only_known) is a global holding every source of a compile: bounded, as each
  compile gets a fresh instance (the native driver a fresh store per run, the playground a fresh instance per compile,
  web/playground/driver.js). A host keeping one compiler instance across compiles would grow it: clear it there.
- Open for the user (morning): typed $items (seed ABI), map_index 14.8 % (scope_with copies maps via field_with,
  enum ·case lookups by name), GC 15.8 %.

## Hello takes half a second (card hello-takes, 2026-11-07)
`time warp hello.warp`: 1.5.1 0.22 s / 1.9 G instructions, 1.6.1 0.47 s / 6.1 G, main 0.47 s / 5.5 G. WARP_TIMES
marks are printed where a phase *starts*, so the time after `pass sugar` is sugar's, not iteration_inlining's.
- The native warp sets WARP_LIB for every run (core/native/warp.warp, so warp.wasm's commands read its lib/), and
  the compiler read any WARP_LIB as "a lib/ edited since its build": every compile rescanned all modules' rule needs
  (scanned_rule_needs, which re-cut extensions/rules/rule_words.warp), hashed the prelude and the sugar files. First
  fixed by a trust flag (WARP_LIB_INSTALLED, d152612c4d), then, by decision no-lib-reads (user: "we do not touch any
  library files after the compiler is built"), the whole edited-lib detection was deleted (4e6b583518): a compile
  trusts lib/ as its build left it, an edited lib/ is a rebuild. hello 5.5 G -> 3.1 G instructions.
- text == (equal_texts) ran NFC's quick check (maybe_decomposed) code point by code point over both texts at every
  unequal compare, ~40 % of tree_patterns' compile through node_equal: bytes below 0xCC (U+0300's lead) are skipped
  bytewise now. tree_patterns 23.6 G -> 17.6 G with the fix above.
- one_typed_definitions cut prelude + math.warp joined, a text the index never holds: each is cut on its own now and
  math is a sliced module of the index.
- Profiling hello: `sample` attaches ~0.2 s late, so profile.sh catches only a hello's tail; use WARP_TIMES phases
  for it, or a longer program.
- mentions_word split the whole source once per word asked: implicit uses ask each of the index's ~1800 words.
  names_mentioned finds the source's names in one walk.
- A source with a character beyond ASCII (hello's 🌍) read and scanned all 47 module texts for their glyph operators
  (declared_operators.warp library_glyph_operators): the index's section `operator` holds them now. With the two
  above: hello 3.1 G -> 2.05 G, 0.16 s (4e6b583518).
- WARP_TIMES also prints `§R <path>` for each file a compile reads (library.warp file_source): the audit tool of
  no-lib-reads. At 4e6b583518 a hello read prelude, math, primitive, c, extensions/uncertain/operators and the five
  type modules besides modules/index.tsv and sugar_rules.packed.
- P2/P3 (90e4d4a4fa): prelude, math, primitive, c and the uncertain operator rows are built into the compiler by
  module_source (scripts/compiler-inputs.sh counts every module_source file as the compiler's code, so editing one is
  a compiler rebuild); the type modules' tables (sections provider, nullary, unary) and the standard modules'
  compile-time rules (extensions/rules/modules, packed beside the modules with the sugar's) come from what the build
  writes. A hello compile reads no lib/ or extensions/ source: 2.05 G -> 1.83 G instructions, 0.13 s.
- Sample audit (warp compile of each sample, WARP_TIMES §R lines): at 90e4d4a4fa only 27 of 263 samples read no lib/
  source. The most read: reals 173, units 158, unit_names and uncertain 147, native/units 139, i18n 112. They come
  from rule needs naming a module the program doesn't link: rules_of_module and modules_used_by read its text, and
  used_module_text probed lib/native/<m>.warp and the crates cache first. Next (P4): an unlinked module is known by
  its prebuilt signature (modules/<m>.wasm's warp.signature).
- P4 (ba51489d85): a module a compile doesn't link is known by its prebuilt signature (signature.warp
  prebuilt_signature: modules/<m>.wasm's warp.signature, read once a compile), never its text; the signature gained
  the module's rule patterns. fizz-buzz 6.16 G -> 2.39 G instructions, 0.55 -> 0.21 s; samples compiling without a
  lib/ read 27 -> 52 of 263 (data/reads.sh in the worktree; note bash 3 has no globstar, so samples/*/*.warp only).
- Left after P4: programs that `use` a module read it (the parser's declared operators, reached modules, with_reached
  of modules not linked), and 48 of lib/'s 95 modules (csv, path, tables, toml, html…) are not prebuilt at all (only
  std_module_names, 47, are), so a program using one compiles it from its source: prebuilding every module is the
  general fix.
- P5 (hello-takes-interfaces): implicit uses find the used modules by their signatures (used_module_names), a used
  module's declared operators come from its signature (new field `operators`, one text each as the index's section
  holds them), a standard module's `use` probes no lib/native file for C lines, and whether a name is a module asks
  the disk (is_module_name), not its text. `use json` reads lib/json.warp still: with_reached pulls the linked
  module's source through the passes (1.0 s / 8.7 G instructions on main and here, sugar + first_sugar the most);
  first_sugar reads memory.warp and sound.warp. Next: a linked prebuilt module is linked, never recompiled.
- Pitfall: a function named `operator_<x>` declares the operator <x> (lib/aliases.warp's operator_·): helpers
  operator_written/operator_read broke the index scan (cast failure in key_left under alias_fields, found with
  WARP_BACKTRACE=1); they are written_operator/read_operator.
- P6 (cbd32c5972): a called word's definitions (definitions_of: data_positions_called, the definition heads) are
  parses from the prebuilt signature, no longer the defining module's text cut once a word; and a module's signature
  is cut out of its wasm read as text (byte_slice) instead of decoded a code point at a time (utf8_text, ~15 % of a
  `use json` compile). hello 1.84 G -> 1.42 G instructions (0.11 s), fizzbuzz 2.0 G -> 1.53 G, `use json` 8.6 G ->
  7.0 G. Pitfall: a text holding wasm bytes is asked for its byte_size, never compared (`== ""`): text equality
  normalizes (NFC) and trapped on the bytes.
- Left: `use json` still lowers the reached module definitions with the program (lowered_reached: sugar and walks,
  ~80 %; card sound-start-speed, warp-sprint-web); imports.warp's reached_within and the libm/foreign line scan read
  each reached module's text (next: from the signature); prebuilding the 9 modules prebuilt ones link waits for
  numeric-dispatch phase 2 (warp-sprint-units-time).
