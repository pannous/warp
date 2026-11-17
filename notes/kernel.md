# A small kernel and one rule mechanism (card core-kernel-census)

Goal (user 2026-11-17): warp's core derives from a kernel of a few forms instead of ad hoc passes; everything else is
a **rule**. Model: Spritely Hoot / Guile (Scheme → Tree-IL, ~10 forms → CPS → wasm GC). Unlike Scheme's syntax-only
macros, warp's rules are **semantic pattern matching**: a pattern may match meaning (`$a + $b` where both are Meters),
and its output is warp code that must type-check (Scala 3 typed quotes; Rust's macro_rules only match syntax
categories). notes/layers.md (notation / kernel / sugar) is the starting point; this note makes the kernel exact and
measures what still knows surface forms. Research only: no code changed.

## 1. The kernel: 14 forms

One canonical Node shape each (today the emitter accepts several: `if` alone arrives as `Else(Then(If c))`,
`Then(If c)` and `Question(c, Colon)`). A kernel head is a reserved `·` word no program can write, as the hidden words
`ran·without·error`, `loop·step` are today (core/analysis/words.warp HiddenWord).

| # | form | warp it comes from | note |
|---|------|--------------------|------|
| 1 | literal: number, text, char, yes/no, ø, symbol, quoted Node | `3`, `"a"`, `'a'`, `data a + b` | quote = data that is not run |
| 2 | name: a local, parameter or global read | `x` | |
| 3 | `define name(params: T = d …): R := body` | functions, constants, `global` as an attribute | binds |
| 4 | `set place = value`, place = name, `x.f`, `x#i` | `x = 1`, `p.x = 2`, `xs#1 = 3` | the effect |
| 5 | `fn (params) body` | `x => x + 1`, `{ it * 2 }`, `\|x\| …` | closure conversion stays a kernel lowering |
| 6 | `call f args` | `f(1, 2)`, `x.f(y)`, operators `a + b` | one call; dispatch by arity/type is a typed rule |
| 7 | `prim word args` | intrinsics, runtime words, host words, `wasm i64.rem_s` | today 6 separate emitter branches |
| 8 | `if c a b` | `if/unless/?:/then-else`, `x if c` | |
| 9 | `seq a b …` (a block, its own scope) | `{…}`, `(a; b)`, indented lines | |
| 10 | `loop body` with `break`, `continue`, `return v` | `while`, `for`, `n times`, `until` | `while c {b}` = `loop { if not c break; b }` |
| 11 | `record Name {fields} variants` | `class`, `enum`, `type X = A \| B`, records | the one type declaration |
| 12 | `case subject arms` (kind/variant test + bind) | `match`, node cases | kept as kernel: decision in layers.md #13 |
| 13 | `try body handler` | `try x else y`, `catch e`, `except` | wasm try_table |
| 14 | `phrase a b …` | `f x`, `print -1`, `width /height` | juxtaposition: the only form with no meaning of its own |

Meta forms beside the kernel (they build the language, they compute nothing): `use module`, `rule …` (today `rewrite`,
`infix operator`), `operator` rows (the notation's precedence table).

`phrase` is the honest form for warp's type-directed readings: the reader leaves `f x` / `print -1` as a phrase,
a typed rule gives it a meaning (`call print (-1)` where print takes a value), and a phrase no rule claims is the
refusal naming its readings. Refuse-never-guess becomes one place instead of the ~30 parser and emitter spots of today.

Value layout assumed (unchanged, notes/self_hosting.md "Value layout"): int i64, float f64, yes/no i32, char i32,
text `(array (mut i8))`, list `struct(len, items)`, everything of unknown kind a boxed `$Node(kind, data, value)`.
The kernel needs nothing new from it: `prim` is the door to it.

## 2. Two stages, read off the pattern

| stage | runs | pattern may ask | output | examples today |
|---|---|---|---|---|
| **spelling rule** | after parse, before analysis | shape only (+ scope: bound/unbound) | any warp, reaches the kernel | `for` → `while`, `try x else y`, `unless`, `a ?: b`, Rust `Color::Red`, comprehensions |
| **typed rule** | after analysis, on kernel code | kinds and signatures of its holes | kernel code, analysed again; a type error in it is the rule's bug | `$a + $b: Meters`, `print -1`, `x is T`, `xs.map(f)` inlined, `d[1]` on a number-keyed map, broadcasting |

- The engine picks the stage from the pattern: a typed hole (`$a: Meters`) or a `where … takes/is function/is type`
  makes a typed rule, everything else is a spelling rule. No author chooses a stage.
- A typed rule's pattern is always a kernel form (spelling rules ran before), so **the analyzer only ever sees kernel
  code**. That is what makes the split clean.
- Measured: of the 410 rules in extensions/sugar/, 82 (20%) ask about names or signatures (`is function`,
  `takes value`, `is type`); none asks for an inferred kind yet: the operator overloads (notes/rewrites.md "Typed
  overloads") test the code's shape and leave run-time kinds to the built-in. Typed rules on the analyzer's kinds would
  let `Meters + Meters` fire on variables, not only literals.
- The cost: the analyzer runs today inside the emitter, after every pass. The typed stage moves it before lowering
  and re-analyses each function a typed rule changed (a fixpoint, as the emitter's widening rounds already are).

Recommendation: these two stages, chosen by the pattern, with `phrase` as the one ambiguous kernel form. Alternative
(one stage, all rules after analysis) would make the analyzer understand every surface spelling: the bloat this
removes.

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
- **Compile cost** (stage 3 self build ≈3.1 T instructions, 11-13; full rebuild ≈10 T). Three risks: (a) rules tried
  per node: the engine already narrows by head word (`sugar_rule_heads`); every moved pass needs a head (a kernel
  head, an operator or a word) or it scans everything; (b) re-analysis after typed rules: per changed function only;
  (c) template interpretation vs. a compiled pass. Gate per step: `--times` phase of the rule stage and stage-3
  instructions within +1%, else the step waits for rule indexing.

## 4. Census (measured 2026-11-17 on origin/main 644169bcb8: `wc -l`, `git grep`)

Marks: **K** kernel (stays), **I** kernel infrastructure (tree walk, modules, linking, the rule engine; stays),
**S** spelling rule, **T** typed rule, **C** check (diagnostic, no rewrite; an extension lint).

### core/passes (10,755 lines)
| file | lines | mark | what |
|---|---:|---|---|
| rewrites.warp | 2,718 | I | the rule engine itself: grows the typed stage, may shrink once rules compile to `match` |
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

### core/emitter (6,297 lines)
The dispatch (`group_expression` + `key_expression`, emitter.warp 1216–1530) has 130 branches; at least 25 name a
surface form (spaced calls, `raise` words, three `if` shapes, `as` per target, `+=`, `/=`, `++`, `.name`). Estimate
~100 lines leave once kernel shapes are canonical. The rest is K.

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
| core/emitter | 6,297 | ~100 (estimate) |
| runtime 4,019, native 4,498, rest 850 | 9,367 | 0 (not surface forms) |
| **core** | **33,171** | **≈5,900 (18%; 34% of the front end parsing + passes + analysis)** |

Plainly: most of core is the emitter, runtime, native driver, rule engine and module linking, none of which knows a
surface form. Surface-form knowledge in core is ≈5.9 k lines; as rules it would be an estimated 600–1,200 lines (sugar
averages 2 lines a rule; a pass's rule set is guessed at 5–10× shorter), plus ~1,000 lines of C checks moving to
extension lints unchanged. The kernel core after the migration: ≈27 k lines, ≈21 k of it outside the front end.

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
the form's test folder plus self-check green, stage-3 instructions within +1%. Never two forms in one step.

1. **The kernel contract, measured.** core/kernel.warp: `is_kernel(n)` for the 14 shapes, run under `--kernel-report`
   between passes and emitter, listing every non-kernel shape still reaching the emitter, by form and owner
   (passes.warp pass_owners already names owners). No behavior change; it is the migration's progress bar and turns
   the emitter's three `if` shapes into one (delete two branches). Tests: smoke, self-check.
2. **`try` as a spelling rule.** core/passes/tries.warp (94 lines) → `extensions/sugar/tries.warp`:
   `rewrite try $x else $y => try·(… )`, `try $x catch $e $h`; the parser's `try` branch family (160 lines) shrinks to
   reading a phrase. Proves block holes and positions through a rule. Tests: test/control (test_bare_try,
   test_assert_else), test/errors.
3. **`x is T` as the first typed rule.** core/analysis/type_tests.warp (81 lines) → a rule
   `$x is $T where $T is type => is_type($x, "$T")`, which the engine can already say; then make `is type` read the
   analyzer's facts instead of the program's spelling. Proves the typed stage and the stage-from-pattern choice.
   Tests: test/types.

Then by size of win: arity_overloads (945, T), node_matches + node_cases (648), plain_enums + enum_declarations (306),
definitions (260), the parser's `if`/`for` heads (466).

Open for the user (via warp-interviewer): the two-stage design above, and whether `phrase` becomes a kernel form.
