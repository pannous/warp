# A small kernel and one rule mechanism (card core-kernel-census)

Goal (user 2026-11-17): warp's core derives from a kernel of a few forms instead of ad hoc passes; everything else is
a **rule**. Model: Spritely Hoot / Guile (Scheme → Tree-IL, ~10 forms → CPS → wasm GC). Unlike Scheme's syntax-only
macros, warp's rules are **semantic pattern matching**: a pattern may match meaning (`$a + $b` where both are Meters),
and its output is warp code that must type-check (Scala 3 typed quotes; Rust's macro_rules only match syntax
categories). notes/layers.md (notation / kernel / sugar) is the starting point; this note makes the kernel exact and
measures what still knows surface forms. Research only: no code changed.

## 1. The kernel: 13 forms

One canonical Node shape each (today the emitter accepts several: `if` alone arrives as `Else(Then(If c))`,
`Then(If c)` and `Question(c, Colon)`). A kernel head is a reserved `·` word no program can write, as the hidden words
`ran·without·error`, `loop·step` are today (core/analysis/words.warp HiddenWord).

| # | form | warp it comes from | note |
|---|------|--------------------|------|
| 1 | literal: number, text, char, yes/no, ø, symbol; `quote` of a Node with `splice` holes | `3`, `"a"`, `'a'`, `data a + b`, `data f($x)` | quote = data that is not run; splice = the emitter's spliced quote |
| 2 | name: a local, parameter or global read | `x` | |
| 3 | `define name(params: T = d …): R := body` | functions, constants, `global` as an attribute | binds |
| 4 | place: `get place` and `set place = value`, place = name, `x.f`, `x#i` | `p.x`, `xs#1`, `x = 1`, `p.x = 2`, `xs#1 = 3` | read and write are twins |
| 5 | `fn (params) body` | `x => x + 1`, `{ it * 2 }`, `\|x\| …` | closure conversion stays a kernel lowering |
| 6 | `call f args` | `f(1, 2)`, `x.f(y)`, operators `a + b` | one call; dispatch by arity/type is a typed rule |
| 7 | `prim word args` | see the prim list below | today ~10 separate emitter branches |
| 8 | `if c a b` | `if/unless/?:/then-else`, `x if c` | |
| 9 | `seq a b …` (a block, its own scope) | `{…}`, `(a; b)`, indented lines | |
| 10 | `loop body` with `break`, `continue`, `return v` | `while`, `for`, `n times`, `until` | `while c {b}` = `loop { if not c break; b }` |
| 11 | `record Name {fields} variants` | `class`, `enum`, `type X = A \| B`, records | the one type declaration |
| 12 | `case subject arms` (kind/variant test + bind) | `match`, node cases | kept as kernel: decision in layers.md #13 |
| 13 | `try body handler` | `try x else y`, `catch e`, `except` | wasm try_table |

The prims, each one emitter case instead of a branch per surface spelling: `convert x T` (every `as`: the
emitter's `Op.As` branches per target today), `new_list items`, `new_map entries`, `new Name fields` (list, map and
instance literals; today `Name{…}` vs `name:{…}` is decided inside the emitter), intrinsics, runtime words, host
words, `wasm i64.rem_s`. A prim the kernel lacks means the emitter keeps a branch: that is the test of the list.

Meta forms beside the kernel (they build the language, they compute nothing): `use module`, `rule …` (today `rewrite`,
`infix operator`), `operator` rows (the notation's precedence table).

**`phrase` is not a kernel form.** Juxtaposition (`f x`, `print -1`, `width /height`) is a reader form: the reader
leaves it as a phrase, a rule gives it a meaning before the kernel (`call print (-1)` where print takes a value), and a
phrase no rule claims is refused with its readings. Refuse-never-guess becomes one place (the end of the signature
stage) instead of the ~30 parser and emitter spots of today, and no phrase ever reaches the analyzer.

Value layout assumed (unchanged, notes/self_hosting.md "Value layout"): int i64, float f64, yes/no i32, char i32,
text `(array (mut i8))`, list `struct(len, items)`, everything of unknown kind a boxed `$Node(kind, data, value)`.
The kernel needs nothing new from it: `prim` is the door to it.

## 2. Three stages, read off the pattern

| stage | runs | pattern may ask | output | examples today |
|---|---|---|---|---|
| **1 shape** | after parse | shape only (+ scope: bound/unbound) | any warp | `for` → `while`, `try x else y`, `unless`, `a ?: b`, Rust `Color::Red`, comprehensions, ranges |
| **2 signature** | after a declaration prescan, before analysis | declared signatures and types: `takes value`, `is function`, `is type`, `$x: Meters` of a declared parameter | kernel code | `print -1`, `x is T`, `print #xs`, overloads by arity, broadcasting over number parameters; every phrase resolved or refused here |
| **3 kind** | after analysis, on kernel code | inferred kinds of its holes | kernel code, re-checked; a type error in it is the rule's bug | `d[1]` on a map that started `{}` (number_keys), `xs.map(f)` inlined by the item kind, `Meters + Meters` of variables |

- The engine picks the stage from the pattern: a shape-only pattern is stage 1, one asking a declared fact stage 2,
  one asking an inferred kind stage 3. No author chooses a stage.
- **Typed patterns are written in surface syntax** (`$a + $b where $a: Meters`) and lowered by stage 1 like any
  code, so a stage-3 pattern is a kernel form and **the analyzer only ever sees kernel code**.
- Measured: of the 410 rules in extensions/sugar/, 82 (20%) ask about names or signatures (`is function`,
  `takes value`, `is type`): **all 82 need only stage 2**, a declaration prescan, no inference. None asks for an
  inferred kind yet; the operator overloads (notes/rewrites.md "Typed overloads") test the code's shape and leave
  run-time kinds to the built-in. Stage 3 lets `Meters + Meters` fire on variables, not only literals.
- Stage 3 is not monotone: the analyzer widens kinds (int → float, a parameter widened by a later call), so a rule that
  fired on an early kind could be wrong later. Stage-3 rules fire only on final kinds, then the changed functions are
  re-checked; a re-check that changes a kind a rule fired on is a compile error naming the rule, never a silent
  re-fire.
- Prerequisite: rules indexed by head and compiled to matchers (a `match` per head, not the interpreted rule list).
  Without it the cost gate (section 3) blocks the big moves; arity_overloads (945 lines) would be tried at every call.

Recommendation: these three stages, chosen by the pattern. Alternative (one stage, all rules after analysis) would
make the analyzer understand every surface spelling: the bloat this removes.

## 3. Rule mechanics

- **Positions.** Already right for template rules (notes/rewrites.md "Positions"): a template's own nodes take the
  rewritten code's position, holes keep theirs. Add: Meta records "expanded by rule R", so an error says
  `line 4: \`try x else y\` (rule extensions/sugar/tries.warp:3): …` and still points at the user's line. A typed rule's
  output that fails to type-check is reported as that rule's error at the user's line, never as the user's.
- **Hygiene.** Introduced names are hygienic already (F3 fresh `t·3`). Missing: referential transparency. A template's
  free word (`map_entries` in loop_heads' output) resolves where the rule fires, so a program defining its own
  `map_entries` captures it. Fix: a template's free words resolve in the rule's module (renamed `module·word`, as
  `prelude·` names already are); kernel heads are reserved and never captured.
- **Rules that compute.** Template rules cover the sugar. A rule whose output needs code (`function_clauses` merging
  several definitions) calls a warp function at compile time. For extensions this is what their passes are now: the
  function is part of warp.wasm. A program's own computing rules need its rule module compiled to wasm and run beside
  the compiler (native: the driver's wasmtime; browser: WebAssembly in JS), Nodes crossing as `$Node`. Not needed for
  the migration; card it when a program asks.
- **Bootstrapping.** Rules in extensions/sugar/ are read from lib/ at compile time (not baked), so moving a form from
  a pass to a rule needs no seed change: the seed still has the pass and builds the new compiler, the new compiler
  compiles itself with the rule (stage 2 proves the rule on the compiler's own source), then
  scripts/refresh-seed.sh (stages A B C). A rule form the seed cannot parse waits for a refresh (notes/rewrites.md
  already hit this with `infix operator as`).
- **Compile cost** (self build ≈3.1 T instructions, 11-13; full rebuild ≈10 T). Three risks: (a) rules tried
  per node: the engine already narrows by head word (`sugar_rule_heads`); every moved pass needs a head (a kernel
  head, an operator or a word) or it scans everything; (b) re-analysis after typed rules: per changed function only;
  (c) template interpretation vs. a compiled pass. Gate per step: `--times` phase of the rule stages and self-build
  instructions within +1%, else the step waits for rule indexing.

## 4. Census (measured 2026-11-17 on origin/main 644169bcb8: `wc -l`, `git grep`)

Marks: **K** kernel (stays), **I** kernel infrastructure (tree walk, modules, linking, the rule engine; stays),
**S** spelling rule, **T** typed rule, **C** check (diagnostic, no rewrite; an extension lint).

### core/passes (10,755 lines)
| file | lines | mark | what |
|---|---:|---|---|
| rewrites.warp | 2,718 | I | the rule engine itself: grows stages 2 and 3, shrinks once rules compile to head-indexed `match`es (see "Infrastructure rows") |
| prelude_words.warp | 1,331 | I | reached definitions of prelude and modules |
| arity_overloads.warp | 945 | T | `twice(n)` / `twice(a, b)`: call resolution by arity, receiver |
| lambdas.warp | 931 | K ≈830 / T ≈100 | closure conversion (K); `xs.map(f)` inlining and map-fold fusion (T) |
| uses.warp | 554 | I | `use` of files |
| walk.warp | 499 | I | shared tree walk, Node ↔ view |
| node_cases.warp | 342 | T | `Key(a, op, b)` builds a node unless the program names the case |
| node_matches.warp | 306 | S | `match n { Key(l, op, r): … }` onto kernel case |
| scope.warp | 289 | I | names and bindings (hygiene needs it) |
| library.warp | 262 | I | lib/ files and index |
| definitions.warp | 260 | S | `f(x) = e`, `f = { it * 2 }` as definitions |
| plain_enums.warp | 251 | S | `enum Color { Red }` → record with variants |
| passes.warp | 240 | I | the pipeline |
| kind_changes.warp | 205 | C | `x = [1]; x = 5` diagnosed |
| classes.warp | 200 | I | class declarations as the passes read them |
| number_keys.warp | 173 | T | `d[1]` on a `{}` map keyed by text |
| data_items.warp | 157 | T | writes shown, admitting Error |
| main_globals.warp | 156 | C | a function changing main's variable without `global` |
| data_blocks.warp | 138 | T | a `data`-typed parameter takes its argument as a tree |
| sugar.warp | 132 | S | `xs[i]`, class field types |
| prefix_calls.warp | 118 | T | `print -1`, `f -x` by "takes a value" |
| signature.warp | 113 | I | warp.signature of prebuilt modules |
| open_words.warp | 102 | I | open words across linked modules |
| tries.warp | 94 | S | `try x else y`, `catch e` |
| shapes.warp | 69 | T | spacing marks: `1 +1`, `a *b`, `width /height` |
| loop_heads.warp | 59 | S | `for k, v in m` heads |
| enum_declarations.warp | 55 | S | enum names and cases |
| library_calls.warp | 49 | T | `print #xs` |
| node_patterns.warp | 7 | K | named node shapes |

### core/parsing (4,363 lines; parser.warp's 389 functions grouped by the form they read)
| family | lines | fns | mark |
|---|---:|---:|---|
| notation reader + operator/alias table (includes `continued` 183, `prefixed` 57, `named` 59, which still hold keyword branches) | 1,726 | 168 | K |
| class / enum / type / global declarations | 365 | 26 | S |
| other-language spellings: pipes, `\|x\|`, `*rest`, `!` bang words, paths, URLs, turbofish, `int[]`, `wasm` | 354 | 42 | S |
| `end` / indentation / colon block styles | 250 | 23 | K (layout is notation) |
| `for` / `each` / comprehension heads | 237 | 17 | S |
| `if` / `while` / `unless` / `until` / `return` heads | 229 | 25 | S |
| spacing and phrase readings (`a -1`, `x is in`, test words) | 217 | 30 | T (become `phrase`) |
| definition heads, parameters, `operator` methods | 216 | 24 | S |
| `try` / `catch` / `except` / `finally` | 160 | 14 | S |
| prescans (type names, uses, globals) | 143 | 20 | I |
| lexer 328, interpolation 47, phrase_blocks 27, positions 42 | 444 | | K 370, S 74 (interpolation, phrase blocks) |

### core/analysis (2,389 lines)
| file | lines | mark |
|---|---:|---|
| analyzer.warp | 1,041 | K (knows ~6 surface words: `it`, `raise`, counting words; ~15 lines) |
| builtin_signatures, builtins, signatures, math_builtins, host_words, kinds, types, words, diagnostics | 1,013 | I |
| source_checks.warp + lints.warp | 357 | C |
| type_tests.warp | 81 | T (`x is T` where T is a type: already an engine condition) |

### core/emitter (6,297 lines): the biggest lever
The dispatch (`group_expression` + `key_expression`, emitter.warp 1216–1530, 315 lines) is an ordered if-chain of
130 branches over surface shapes; at least 25 name one directly (spaced calls `f 3 4`, `raise` words, three `if`
shapes, `as` per target incl. `as rational`, `+=`, `/=`, `++` on characters, `.name`, time literals, `Name{…}` vs
`name:{…}`). And the emitter infers kinds while it emits: every emit word returns `TypedCode(kind, code)`, and ~200
lines widen, join or re-read kinds (`widen_parameter`, `joint_kind`, `pending`, `rereads`; grep counts 63 + 88 + 47)
beside analyzer.warp's own inference. Splitting analysis from emission (the analyzer decides every kind, a canonical
kernel lowering runs once, the emitter only translates typed kernel to wasm) is the change that shrinks core most:
estimated 2–3 k of 6.3 k lines (advisor estimate, not measured; the measured floor is the 315-line dispatch plus the
~200 kind lines). The rest (byte codes, module layout, encoding, GC types, closures) is K.

### Infrastructure rows, re-examined
| file | lines | finding |
|---|---:|---|
| core/passes/rewrites.warp | 2,718 | the engine has 22 kinds of `where` condition (14 code properties, 4 context words, `takes`, origin, comparisons, `in […]`; ≥172 lines by function name) and 7 ordered sugar stages (first, early, declaration, comprehension, iteration, late, main; ≥251 lines). Typed holes and three stages chosen by the pattern replace both: ≥420 lines measured, ~1 k estimated |
| core/passes/prelude_words.warp | 1,331 | I: which definitions of the prelude and modules a program reaches (`with_reached` 117 lines), aliasing, slices. No surface form; its free-name walks (`names_within`, `names_bound_in`) are one of the duplicates below |
| core/emitter/prebuilt.warp | 1,144 | I: runtime.wasm and prebuilt module linking, relocation. No surface form |
| binding analysis, duplicated | — | at least 7 passes walk bindings themselves: scope.warp `bound_names`, lambdas `names_bound`, number_keys `names_assigned`, rewrites `bound_by_definition`, prelude_words `names_bound_in`/`names_within`, uses `names_in`, arity_overloads `bound_at`. One scope analysis on kernel code replaces them (not counted in the totals) |

### extensions/sugar rule families (802 lines, 410 rules; 82 ask about names/signatures → T, the rest S)
| family | lines | rules | T | | family | lines | rules | T |
|---|---:|---:|---:|---|---|---:|---:|---:|
| calls | 108 | 47 | 29 | | events | 53 | 29 | 3 |
| reflection | 86 | 49 | 12 | | functions | 47 | 28 | 3 |
| lists | 55 | 32 | 6 | | arithmetic | 43 | 19 | 0 |
| declarations | 54 | 30 | 2 | | broadcasting | 36 | 24 | 10 |
| methods | 51 | 25 | 8 | | iteration | 31 | 14 | 0 |
| enums | 24 | 9 | 0 | | arms | 22 | 12 | 0 |
| extremes | 21 | 16 | 0 | | tuples | 21 | 12 | 0 |
| test_words | 18 | 13 | 1 | | loops | 17 | 6 | 0 |
| method_calls | 16 | 4 | 1 | | ranges | 14 | 5 | 0 |
| tree_patterns | 13 | 6 | 0 | | comprehensions | 12 | 6 | 0 |
| defaults | 11 | 5 | 1 | | with_blocks | 9 | 4 | 2 |
| fallbacks | 8 | 3 | 0 | | heads | 8 | 2 | 0 |
| phrases | 6 | 3 | 0 | | articles, do_blocks, text_elements | 15 | 6 | 4 |
| safe_navigation | 3 | 1 | 0 | | | | | |

### The extension hook table (core/extension.warp 234 lines, 28 fields, 73 extensions with a row, 6,584 lines)
| hook kind | extensions | their lines | under rules |
|---|---:|---:|---|
| tree passes (`passes` 12, `passes_before` 27) | 36 | 3,778 | S or T rules (computing ones call their warp function) |
| `keyword_readers` | 12 | 1,360 | S rules over a `phrase` |
| `source_passes`, `text_passes` | 6 | 928 | reader hooks / S rules |
| character readers (atom, number, suffix, infix, prefix, escape, glyph) | ~15 | | stay: the notation's table |
| non-syntax (library_words, runtime, commands, implicit_uses, lints, attributes) | | | stay |

So 9 of the 28 hook fields (passes, passes_before, keyword_readers, arm_lowerings, text_passes, source_passes,
type_readers, value_settlers, block_words) fold into "rules".

### Total
| | lines today | could leave core |
|---|---:|---:|
| core/passes | 10,755 | 3,509 (S 1,157, T 1,991, C 361) |
| core/parsing | 4,363 | 1,852 (S 1,635, T 217) |
| core/analysis | 2,389 | 438 (T 81, C 357) |
| core/emitter | 6,297 | ≥515 measured, 2–3 k estimated (split analysis from emission) |
| rule engine (inside core/passes above) | | ≥420 measured, ~1 k estimated |
| runtime 4,019, native 4,498, rest 850 | 9,367 | 0 (not surface forms) |
| **core** | **33,171** | **measured ≥6.7 k (20%); with the estimates ≈9–10 k** |

Plainly: the surface-form knowledge alone (passes, parser, analysis) is ≈5.8 k lines; as rules it would be an
estimated 600–1,200 lines (sugar averages 2 lines a rule; a pass's rule set is guessed at 5–10× shorter), plus ~700
lines of C checks moving to extension lints unchanged. The second, larger lever is the emitter doing analysis and
dispatching on surface shapes. Realistic kernel core: **18–20 k lines** if the emitter split and the typed-hole engine
deliver their estimates, ≈26 k on the measured part alone. Runtime, native driver and linking (≈11 k) stay as they are.

## 5. What rule patterns need (from the census)

Counts over the 410 rules in extensions/sugar/ (`git grep`) and the census rows above. "Frees" = core lines that can
leave once the engine has the feature (S/T rows that need it).

| feature | who needs it (evidence) | today | frees |
|---|---|---|---|
| nested binding (`Key($l, Op.Colon, $r)`, holes inside groups) | 239 of 410 patterns; node_matches (306) | built | 306 (node_matches) |
| rest holes, per-item templates (`$xs...`, `T for $x in $xs...`) | 146 rest holes; loop_heads, definitions, arity_overloads | built, zip over two missing | 59 + 260 |
| guards (`where …`) | 236 of 410 rules | built (F2) | (all S/T rows) |
| literal and constant matching (`$n < 0`, `in [push, add]`, `Op.Colon`, enum cases) | 14 rules; node_matches value arms, plain_enums | built for literals and word lists | 251 (plain_enums) |
| **typed holes on inferred kinds** (`$a: Meters`) | number_keys 173, data_items 157, lambdas' inlining ~100, type_tests 81, operator overloads (notes/rewrites.md: today they test the shape, run-time kinds fall back to the built-in) | **missing**: holes test the code's shape (`$x: text` = a text literal) | 511 |
| signature questions (`takes value`, `is function`, `is type`) | 82 rules; prefix_calls 118, library_calls 49, data_blocks 138, shapes 69, parser spacing readings 217 | built for the program's and library words | 591 |
| type variables (`$a: T + $b: T`, `list of T` → `T`) | arity_overloads (945: one name, several signatures), map/filter inlining (the item kind), node_cases (Node's case types) | missing | 1,287 (arity_overloads + node_cases) |
| capability constraints (has `next()`, is a number, is a vector) | iterators (class_methods/iterators.warp), broadcasting (24 rules, 10 by "takes number"), `for` over anything iterable | ad hoc (`takes number`, `is vector` by word) | small; mostly lib |
| hygiene: fresh names | lambdas (`map·1`, `item·1`), loop_heads (`k·pair`), tries, 3 sugar files | built (F3) | |
| hygiene: template words resolve in the rule's module | every template naming a library word (`map_entries`, `is_type`) | **missing** (section 3) | (correctness, not lines) |
| source positions through expansion, "expanded by rule R" | every rule; refuse-never-guess errors | positions built; rule provenance missing | |
| computing rules (a warp function builds the output) | function_clauses 145, multiple_assignments 81, match_patterns 203, the 36 pass extensions | as extension passes | (hook table: 9 of 28 fields) |
| termination | F7 re-rewrite (`rewrite_depth_limit = 64`, rewrites.warp:66) | a depth limit | |
| confluence | rules try in order, first wins, bottom-up, each node once: deterministic, not confluent | order only | |

For termination and confluence the recommendation is the overload rule warp already has (notes/rewrites.md "Typed
overloads"): within one head, more specific patterns first; two typed rules of equal specificity that both match
are a compile refusal naming both, never a silent pick. The depth limit stays as the loud backstop.

### Dependent types? No.

Evidence:
- Units, the strongest candidate, keep a quantity's dimension as a run-time value: `Quantity { amount; dims: int; … }`
  (lib/units.warp:13, `power` multiplies `dims`), plus a class per named dimension (`duration: Time` takes durations
  only). Checking dimensions statically needs integer exponents in types (`Length^1 Time^-1`), i.e. type-level
  numbers with abelian-group unification (F#'s units of measure): decidable, no dependent types.
- Fixed-size arrays `int[3]` are rules onto lists (extensions/sugar/lists.warp); the phrase
  `x is a 100 element array` is named bloat in AGENTS.md "Principles".
- No pass in the census asks a question about a value inside a type; every T row asks "what kind / which signature".
- Dependent types make type checking undecidable in general (it runs programs) and would put an evaluator into the
  analyzer: the core bloat this card removes.

So: typed holes, type variables and, for units, type-level integers at most. Dependent types: not needed.

## 6. Migration (strangler: one surface form at a time, tests as the net)

Each step: a rule file in extensions/sugar/ (or an extension), the pass or parser branch deleted in the same commit,
the form's test folder plus self-check green, self-build instructions within +1%. Never two forms in one step.

0. **Ranges, the pilot the user picked** (a separate worker builds it): `a to b`, `a … b` as rules
   (extensions/sugar/ranges.warp, 5 rules today).
1. **The kernel contract, measured.** core/kernel.warp: `is_kernel(n)` for the 13 shapes, run under `--kernel-report`
   between passes and emitter, listing every non-kernel shape still reaching the emitter, by form and owner
   (passes.warp pass_owners already names owners). No behavior change; it is the migration's progress bar. Tests:
   smoke, self-check.
2. **Split analysis from emission (the highest yield).** One canonical lowering to kernel shapes before the emitter
   (the three `if` shapes become one, every `as` a `convert` prim, literals `new_*` prims); the analyzer decides every
   kind on kernel code; the emitter translates typed kernel and stops widening and dispatching on surface shapes. In
   slices, one emitter branch family at a time, each slice deleting its branches. Estimated 2–3 k emitter lines; it is
   also what stage 3 needs (kinds before lowering). Careful path (parser/emitter): self-check, gate, touched folders.
3. **`try` as a spelling-rule pilot.** core/passes/tries.warp (94 lines) → `extensions/sugar/tries.warp`:
   `rewrite try $x else $y => try·(… )`, `try $x catch $e $h`; the parser's `try` family (160 lines) shrinks to
   reading a phrase. Proves block holes and positions through a rule. Tests: test/control (test_bare_try,
   test_assert_else), test/errors.
4. **`number_keys` as the first stage-3 rule.** core/passes/number_keys.warp (173 lines): `d[1]` on a map that started
   as `{}` reads the entry "1". A true inferred-kind rule (the map's kind comes from its first value), so it proves stage
   3, final kinds and the re-check. (`x is T` is only stage 2 and proves less.) Tests: test/lists, test/types.

Before the big moves (arity_overloads 945, node_matches + node_cases 648): head-indexed compiled matchers (section
2). Then by size of win: plain_enums + enum_declarations (306), definitions (260), the parser's `if`/`for` heads (466),
one scope analysis replacing the duplicated binding walks.

Open for the user (via warp-interviewer): the three stages chosen by the pattern; the emitter split as the first big
step.
