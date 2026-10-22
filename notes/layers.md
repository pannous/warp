# Language layers: notation, kernel, sugar (card language-layers, survey 2026-10-22)

The user asked whether to separate the core parser from the core language and the sugar on top. Proposal
(Supervisor), surveyed against lib/ (Rust is frozen, not surveyed):

1. **notation**: the reader only, the same one warp data uses: atoms, keys (pairs, tags), groups `() [] {}`,
   comments, operators from one precedence table (extensible by `operator` declarations), juxtaposition; no keywords.
2. **kernel**: the forms the analyzer and emitter must know. Test: a form is kernel only if it cannot be a rewrite
   into other kernel forms.
3. **sugar**: everything expressible as `rewrite` rules (lib/compiler/rewrites.warp) into the kernel, one module per
   topic in lib/sugar/*.warp loaded by the prelude. New syntax extends the notation generally (a table row, an
   operator or alias declaration), never a parser keyword branch.

Measured: parser.warp is today ~50 % notation (2102 lines), ~16 % kernel (686), ~34 % sugar (1417); 26 of the 62
other compiler files (2448 of their 10841 lines) are sugar passes. Sugar in all: ~3900 lines.

## The kernel (user decision 2026-10-22: this list plus `match`, "match stays in the kernel 100%")

| # | form | why it can't be a rewrite |
|---|------|---------------------------|
| 1 | `name(params) := body`: definition (functions, constants), parameters with type and default | binds names |
| 2 | `place = value`: assignment to a name, `x.f`, `x#i` | the effect itself |
| 3 | `f(args)` call, `x.f` member, `x#i` element | the primitive operations |
| 4 | `params => body`: function literal | closure conversion needs free-variable capture (lambdas.warp) |
| 5 | `if c then a else b` (also the value form) | the one branch |
| 6 | `while c do body`, `break`, `continue`, `return` | the one loop and its jumps |
| 7 | `class Name {fields}` with variants: the one type declaration | declares a type |
| 8 | `use module` | loads code |
| 9 | `try x else y`: the error guard | wasm try_table, emitted specially |
| 10 | `global` | scope, not a value |
| 11 | `data x`: quote | stops evaluation; the code-as-data door |
| 12 | `rewrite p => t`, `operator …` declarations | the meta forms that build layer 3 and extend layer 1 |
| 13 | `match subject { pattern => value … }` | user decision; its lowering (node_matches, node_cases) is emitter work, never sugar |

Not kernel (evidence: a pass already lowers them away before the emitter): `for` (for_loops.warp → while), `enum` (plain_enums.warp → class with variants), `unless`/`until`, `?:`, `??`,
`x is T` (type_tests.warp → is_type call), `as` (conversion calls), comprehensions, ranges, `n times`. Operators are
notation; their meaning is a call (arithmetic ones are emitter intrinsics, as any builtin).

Open within the decision: `global` could be an attribute instead (`@global`); `try` could be an effect handler
(notes/effect_handlers.md) later; `|x|`/`x =>`/`fun(x)` are three spellings of #4 (sugar).

## Rewrite engine: what it has, what sugar still needs

Has: holes `$a`, rest holes `$xs...`, typed holes `$x: text|symbol|list|block|int|number`, phrase runs (`$n beat`
inside a longer phrase), statement templates `($a; $b)`, origin conditions `where $c = channel()`, module rules that
skip words the program binds, bottom-up, each node once.

Features named by the sugar below (2026-10-22: F1–F4, F6, F7 built, spec and examples in notes/rewrites.md;
F5 unparked with high priority by the user 2026-10-22 evening, card rewrite-templates in Now; F8 open, no card):
- **F1 positions/context** (built): match only as a statement head, in a condition, in a class body, as a key or a value.
- **F2 where-predicates** (built) beyond `= shape`: "$t is a type word", "$x unbound", "$n < 0 literal", "f's first
  parameter is typed".
- **F3 fresh names + evaluate once** (built): hygienic temporaries (`$·tmp`, notes/compiler_temporaries.md `·` names), so
  `a ?: b` computes a once.
- **F4 per-item templates**: apply a sub-template to each item a rest hole bound (`$xs...` mapped). Built as `T for $x in $xs...`; zip over two rest holes
  (`for ($a, $b) in …`, multiple_assignments) is not.
- **F5 name building** (unparked, high priority, user 2026-10-22; card rewrite-templates; decision name-glue): `·`
  glues a hole or symbol into a name inside templates and quoted code (`$i·end`, `square·$a` → square_a_number);
  numbers beside `·` multiply, plain `x·y` between values stays multiplication.
- **F6 several statements** (built, `rewrite ($a; $b) => …`): match a run of statements (`fib(0) := 0; fib(1) := 1` → one definition).
- **F7 re-rewrite** (built, loop limit 64): a result rewritten again until fixpoint (`a of b of c`, for variants onto the one for).
- **F8 text holes** (open, not started; idea: a text pattern `"…$x…"` binding holes from its
  interpolation parts, the template's text holes spliced the same way): rules over a template's holes (interpolation, `sql "…$x"`); stays a pass otherwise.

## lib/compiler/parser.warp by form (4237 lines)

Notation (2102):
| lines | form |
|------:|------|
| 316 | items, separators, grouping by separator strength, indentation blocks, line continuation |
| 176 | `continued`: the precedence-climbing loop (its keyword branches are counted below) |
| 257 | operator declarations, precedence declarations, user operators (`infix operator ⊕ := f`) |
| 86 | operator aliases and circumfix (`operator × := *`, `‖x‖`), lib/math.warp's read once |
| 71 | prefix operator words and circumfix (`sqrt x` is `√x`) |
| 66 | operator tables (SYMBOL_OPERATORS, WORD_OPERATORS), `down to` |
| 41 | binding powers |
| 201 | atoms: dates, number suffixes `0.1f`, `.5`, holes `$a`, `@1`, `°C`, `∅`, graphemes, free symbols, ✔ |
| 117 | texts, escapes, templates `"a ${x}"` |
| 67 | numbers, radix |
| 94 | `named`: names, calls `f(x)`, `Point{x: 1}`, `x°`, turbofish, `int[]` (the last three are foreign sugar) |
| 124 | type application `list<int>`, optional `int?`, typed-field lookahead |
| 86 | vulgar fractions `½`, glued factor `2π`, superscripts `x²` |
| 71 | uniscript entities `\:alpha` (a pre-pass on the text) |
| 52 | paths `~/notes`, URLs |
| 45 | attributes `@x` |
| 45 | juxtaposition: implicit application `print x`, labelled arguments `move 1 from: 10` |
| 53 | where a header ends, arguments |
| 31 | bang words `is!` |
| 17+15+63+8 | key words `{if: 4}`, KEYWORD_FORMS, the `prefixed` dispatch, exact constants π τ ∞ |

Kernel (686):
| lines | form |
|------:|------|
| 201 | type declarations (class/struct/type, fields, variants, sum types) |
| 128 | try/assert/check with catch/except/finally (kernel try; catch/except/finally/assert are sugar on it) |
| 84 | if/while heads (+ unless/until, sugar) |
| 73 | parameters (typed, rest, defaults) |
| 53 | else branches (+ elif/elsif/elseif aliases) |
| 48 | global/export |
| 25 | match-arm and if binding fixes |
| 21 | return |
| 19 | data quote |
| 17 | `wasm i32.add` inline instruction (escape hatch) |
| 17 | `&`/`|` bitwise vs logical: a type question for the analyzer, not the parser |

Sugar (1417), each with the rule it would become:
| lines | form | rule, or the missing feature |
|------:|------|------|
| 236 | for variants: C-style `for (i=0;i<n;i++)`, `for each`, named loops, `for x in xs:`, loop variables | `rewrite for $i in $a to $b $body => ($i = $a; $i·end = $b; while $i <= $i·end { $body; $i++ })`: needs F3/F5; C-style needs F1 (a header of three statements) |
| 187 | English definitions `To square a number: …`, phrase slots | F2 (type words in the head) + F5 (phrase name); hardest, last |
| 153 | symbol prefixes: Rust `&x`, `&mut x`, `*r`; `\|x\| body`; `> 100 =>` relational arm; rest/spread `*xs` | `rewrite &$x => $x`, `rewrite \|$p...\| $b => ($p...) => $b`; spread is notation |
| 110 | `3 days` unit phrases (time literals `10:30` stay notation) | `rewrite $n: number days => days($n)` phrase rule, works today (lib/units.warp style) |
| 103 | Ruby/Lua `do … end`, `then … end`, `fi` | not a rewrite: grouping. Becomes notation: declared word brackets (`circumfix operator do end`) |
| 99 | `shader {…}` / `wgsl {…}` raw-text blocks | not a rewrite: lexical. Notation: a declared raw-block word list |
| 98 | multi-word type names `class full name {…}`, `unsigned int` | lexical (needs a pre-scan); notation table of compound names, or drop |
| 95+39 | `enum Color { Red, Green }` with values, Swift `case` rows, enum bodies | `rewrite enum $n $cases => class $n @variants($cases) {}`: needs F4 for values |
| 68 | slices `xs[:3]`, `x.sort!` mutating suffix | `rewrite $x.$f!() => $x = $x.$f()` (if `!` stays a node); slices: F2 on literal bounds |
| 58 | trailing guards `x = 2 if c`, `x++ while c` | `rewrite $s if $c => if $c then $s`: needs F1 (statement position) |
| 36 | `is not`, `isn't`, `not in`, `nand`, `contains`/`has`/`includes` | operator alias + `rewrite $a isn't $b => not ($a is $b)`, works today once the alias spells an operator |
| 28 | `/!`, `div`, `÷`, `/!=` | already a math.warp alias (`operator ÷ := /!`): the parser branch duplicates it; `operator div := /!` |
| 23 | emit words `emit stop{…}` (send, fire, trigger, signal, ask) | `rewrite emit $e... => emit($e...)`, aliases for the others |
| 22 | test words `x empty`, `x is missing`, `x failed` | `rewrite $x is empty => count($x) == 0` phrase rule, works today |
| 21 | `n times {…}` | no rewrite: a plain library function `infix times(n: number, b: block)` (user 2026-10-22, decision language-layers) |
| 15 | `a ?: b` | `rewrite $a ?: $b => if $a then $a else $b`: needs F3 (a once) |
| 14 | `function f` reference | `rewrite function $f => &$f` |
| 12 | `if no xs` | `rewrite no $x => not $x`: needs F1 (condition position) |

## The other lib/compiler/*.warp files (62 files, 10841 lines)

Machinery, no layer: emitter 2651, analyzer 991, lexer 461 (notation), rewrites 371 (the sugar engine), walk 367,
prelude_words 251, scope 215, uses 193 (kernel `use`), serializer 138, builtin_signatures 113, words 107, diagnostics
101, host_words 74, compiler 48, passes 88, shapes 52, signatures 41, node 39, runtime 33, kinds 20, positions 10,
types 10, byte_codes 9. Checks, no layer: kind_changes 187, source_checks 140, main_globals 94, lints 69.
time_arithmetic 158 and injection 217 are library (runtime words, typed templates), not syntax.

Kernel passes (analysis a rewrite can't carry): node_matches 214 and node_cases 178 (match, kernel by decision), lambdas 283 (closures), arity_overloads 388 (overloads by count),
number_keys 152 and data_items 150 (map key typing, writes through references), tries 91 (try lowering),
enum_declarations 63 (reads declared types).

Sugar passes, with the rule each would become:
| lines | file | rule, or the missing feature |
|------:|------|------|
| 197 | plain_enums | `Color.Red` → the case: F2 (declared enum cases) |
| 186 | definitions | `rewrite def $d => $d`, `rewrite $f($p...) = $b => $f($p...) := $b`: works today, minus `xs(1) = …` (F2) |
| 163 | of_lookups | `rewrite $a of $b => $b.$a`: phrase rule works today; type phrases `list of int` need F2 |
| 137 | for_loops | see the parser's for row: F3, F5, F7 |
| 121 | welcome_forms | match-arm spellings onto one: rows of rules, F4 for arms |
| 121 | foreign_forms | `rewrite public $d... => $d...`, `rewrite new $c => $c`: F1 (statement head); `attr_accessor` F1 (class body) |
| 116 | min_max | `max(a, b)` → library function; the comparison form needs F3 |
| 97 | library_calls | `rewrite reverse $x => reverse($x)` per typed prelude word: F2 (typed signature), unbound already handled |
| 91 | clauses | `fib(0) := 0; fib(1) := 1` → one kernel match: F6 |
| 88 | comprehensions | `rewrite [$e for $x in $xs] => (r = []; for $x in $xs { r += [$e] }; r)`: F3 |
| 86 | prefix_comparisons | `sq 2 == 4` → `sq(2) == 4`: F2 (parameter typed as a value) |
| 65 | number_words | `rewrite one => 1` per word, works today (module rules skip bound words); keys/members stay: F1 |
| 65 | parse_words | `rewrite eval($t) => interpret(parse($t))` works today; folding a constant text is a pass |
| 64 | multiple_assignments | `a, b = b, a + b`: F3 + F4 |
| 61 | list_methods | `rewrite $xs.push($v) => $xs += [$v]` (add, append alike): works today |
| 44 | interpolation | F8 |
| 42 | type_tests | `rewrite $x is $t => is_type($x, "$t")`: F2 (t a type word no name binds) |
| 36 | ranges | range value → loop: F3 |
| 35 | filled_lists | done: lib/sugar/lists.warp (card sugar-filled-lists) |
| 32 | broadcasting | `rewrite $xs .* $y => $xs.map(e => e * $y)`: F3 (hygienic e) |
| 30 | preposition_heads | `play(tone, for duration)` → drop the preposition per parameter: F4 |
| 24 | grouped_receivers | `rewrite ($p).$a => $p.$a`: F2 (p bound to a value) |
| 18 | phrase_blocks | not sugar: `{print "big"}` as one statement is the notation's grouping; belongs in the reader |
| 15 | coalesce | done: lib/sugar/fallbacks.warp (card sugar-coalesce) |
| 140 | sugar.warp | `rewrite $xs[$i] => $xs#($i + 1)`: works for plain indexes; negative and slice bounds need F2; typed class fields are kernel (types) |

## Migration order, by gain

1. **Word aliases into tables, no rewrite needed** (~100 parser lines): elif/elsif/elseif, null/nil/none/…, yes/true,
   func/function, contains/has/includes, `div`/`÷` (math.warp already has `÷`: delete the parser branch), `isn't`,
   `nand`, `not in`. KEYWORD_FORMS keeps only kernel words.
2. **Rules that work today** into lib/sugar/*.warp (~550 lines of passes and parser branches → ~30 rule lines):
   list_methods, of_lookups (keep the type-phrase guard), definitions' `def`/`fun`, number_words, test words, emit
   words, unit phrases, `n times`, parse_words' eval. Each is a cloud-free warp change (decision
   warp-only-acceptance) and its twin pass goes.
   Status (card sugar-migration-2, branch layers): done are list_methods' mutation methods and membership
   (lib/sugar/methods.warp, applied by the `sugar` pass where list_methods ran, rules chosen by the program's own
   bindings) and floor division (`/!`, `/!=` table operators, `div` a word alias, lib/sugar/arithmetic.warp). The
   rest is not "works today": test words, emit words, unit phrases and `n times` are parser grouping (notation, item
   6); number_words needs F1 (keys and members); parse_words' eval needs scoped shadowing; `def`/`fun` is ~10 of
   definitions' 186 lines; of_lookups and number_words are twinned stages that Rust drives in the
   WARP_SELF_HOSTED=1 lane, where lib/sugar rules don't run. `n times {…}` becomes a plain prelude function
   `infix times(n: number, b: block)` (user), then the parser's times branch goes.
3. **F3 fresh names + evaluate once** (one engine feature, then ~550 lines): coalesce, `?:`, comprehensions, ranges,
   for_loops, min_max, multiple_assignments, broadcasting. Largest gain per engine line.
4. **F2 where-predicates** (~450 lines): type_tests, filled_lists, grouped_receivers, library_calls,
   prefix_comparisons, plain_enums, subscripts with negative bounds, Java `int[]`.
5. **F1 positions** (~250 lines): trailing guards, `if no xs`, foreign modifiers, C-style for heads.
6. **Notation generalizations** replacing parser branches (~300 lines): word brackets (`do … end`, `fi`), raw-text
   block words (shader, wgsl), compound names. Each is one table, never a keyword branch.
7. **F4–F7, last**: clauses, preposition heads, English `To …:` definitions (~310 lines). These may stay
   passes written in warp; they are sugar by meaning, passes by mechanism.

Rule of thumb for new syntax from now on: a table row or a declaration (notation), else a `rewrite` line in
lib/sugar/ (sugar); a new parser branch or pass only with the missing engine feature named in the commit.

Related: notes/self_hosting.md "Parser restructuring" (parser-keyword-table, parser-desugar-pass), notes/passes.md
(inventory; the keyword-rewrite table was parked for speed, not shape), notes/compiler_temporaries.md (F3's `·` names),
notes/quasi_quote.md (patterns and templates).
