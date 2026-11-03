# Core extensions roadmap (decision core-is-self, card core-extensions-roadmap)

The core is what the compiler needs to compile itself; every other feature becomes an extension, baked in and
registered in one static hook table. Each section lists one area's candidates. A section is a static reading of the
sources (what the compiler's own code never writes), until the measurement of what compiling compiler.warp reaches
confirms it.

## parsing and operators (warp-operators, static reading 2026-11-03)

Method: for each feature group, its functions' lines in core/ (span from one definition to the next), and whether any
code line of core/ outside core/parsing/ writes the form (comments and texts not counted). "Compiler uses: no" makes
it a candidate; core/extensions/sugar counts as an extension already.

| feature | where | lines | compiler uses | hook it needs |
|---|---|---|---|---|
| time literals and counted units: `5ms`, `2h`, `12:30`, `3 times`-style counts | parser.warp glued_time_at … time_unit_at, duration_text, counted_unit, counts_units | 114 | no | **atom suffix**: after a number, an extension may read a glued suffix into a literal (bare_atom) |
| number forms: `½`, `x²`, `0.(3)`, overlined digits, `×10^`, `5%`, exact `π` | parser.warp vulgar_fraction, with_glued_factor, superscript_power, reciprocal_if, raised_terms, overlined_digits, repeating_decimal, ten_to_the, percent_ends, exact_constant | 135 | no (`π` only in comments and texts) | atom suffix (above) and **atom glyph**: an atom starting with a registered glyph |
| radix numbers `0x…`, `0b…` | parser.warp radix_value, digit_value | ~30 | only 3 constants in core/commands/commands.warp (MACHO_MAGIC, SEGMENT_64, PAGE_BYTES) | atom glyph; minimal source change (decision self-source-minimal): those 3 constants written in decimal with the hex in a comment, then the form leaves the core |
| shader blocks, inline wasm blocks | parser.warp shader_block, closing_brace, dedented, indent_width, without_trailing_blanks, shader_text, wasm_instruction_after | 119 | no | **raw block**: a registered head word whose block is read as text |
| URL and path literals `https://…`, `./x` | parser.warp url_text, starts_path(_at), path_literal, is_operand_end, ends_path, starts_path_name | 43 | no | atom glyph (above) |
| circumfix operators `\|x\|`, `‖x‖` and math aliases | parser.warp circumfix_*, math_circumfix_aliases, math_operator_aliases; core/node.warp keeps Op.Abs … only for the seed (card remove-unused) | 60 | no | **operator table**: lib/aliases.warp already holds the spellings; the parser asks the table |
| HTML entities `&amp;` | parser.warp entities, expanded_entities, entity_name_at, text_between, entity_read (table entities.tsv) | 67 | no | **text pass**: a registered rewrite of source text before parsing |
| declared operators: `infix operator ⊕`, fixity, precedence, glyph operators beyond ASCII, operator methods | parser.warp fixity_level … operator_method_definition, binding_powers' declared part | 195 | only core/extensions/sugar (calls.warp, enums.warp: `infix operator \|`, `as`) | operator table, plus **declaration**: a statement head the parser hands to the extension. Moves together with the typed-operator rules of rewrites.warp (operator_overload, operator_rules) |
| spacing diagnostics: `1 +1`, `a *b` | parser.warp signed_item_at, spaced_sign_marked, unevenly_spaced, written_before; source_checks.warp spaced_sign, uneven_spacing; warnings.data 2 lines | ~50 | no (a lint) | **parse marks**: the parser already marks with attributed(…); a lint extension registers its mark and its check |
| other lints: `"3" + 3`, bare `sleep(1)`, charged assignments, discarded copies | source_checks.warp (192 in all), lints.warp (88), warnings.data | ~250 | no | **checks**: source_checks' list of checks becomes the registered checks |
| loop forms: C-style `for (;;)`, `n times {…}`, named loops, `down to` | parser.warp c_style_for, at_c_style_for_head, named_loop, times_binds, repeated, down_to_at; sugar loops.warp:4 | 104 | no | **statement head**: a registered head word starts a statement form |
| natural-language definitions (phrase slots, phrase redefinition) | parser.warp phrase_slots, in_slot, known_type, slot_name, phrase_redefinition, phrase_result_type, shaped | 110 | no | **definition head**: a definition whose head is a phrase |
| Ruby bars `{ \|x\| … }` | parser.warp bar_lambda, pipe_parameters | 22 | no | atom glyph inside a block start |
| other languages' declarations: `new Point(1, 2)`, `attr_accessor :x`, `enum class`, modifier runs | core/parsing/foreign_forms.warp (whole file) | 118 | no | declaration (above) |
| typed templates `sql "…"`, `sh "…"` | core/parsing/injection.warp (whole file) | 231 | no | **text prefix**: a registered word before a text literal |
| English number words `one plus two` | core/parsing/number_words.warp (a pass) | 65 | no | **pass list**: registered passes in pure_passes |
| text times a count, `"ab" * 3`, and yes/no as int in operators | core/emitter/operators.warp repeated, is_count, bools_as_ints | ~15 | check: the compiler may repeat texts | stays core until measured |
| marks surviving arithmetic, `30° * 2` | core/emitter/operators.warp operation_code, marked, marks_kind | ~15 | no (units' marks) | moves with units: **operation hook** after the core's arithmetic |
| postfix / phrase calls: `print #xs`, `f a > b` read by f's parameter (juxtaposition) | core/passes/prefix_calls.warp (135), data_blocks.warp (65), library_calls.warp (50) | 250 | likely yes (`print #xs`-style calls in core/) | measure first |

Candidates by static reading: about 1,600 lines (parser.warp about 1,050 of its 5,060; the rest foreign_forms,
injection, number_words, source_checks and lints). The hooks they share are few: atom suffix, atom glyph, raw block,
statement head, declaration and definition head, text prefix, text pass, operator table, parse marks with checks,
and the pass list. These ten table entries would carry them all.

Against the target shape (Lessons from wasp: operators as rows, keywords through a name → handler table):
- The rows exist half already: lib/aliases.warp is one entry per operator (operator, prefix-operator, symbol Op,
  aliases, infix). What is still code: binding_powers (parser.warp, a 40-line if-chain of precedence and
  associativity), DECLARED_ONLY_OPERATORS, fixity_level and the declared-operator scan (195 lines). Adding
  `power: [left, right]` and `fixity: prefix | infix | suffix | circumfix` to the rows turns all of it into a table
  read, and the user's `infix operator ⊕` becomes a row added at parse time.
- Suffix and circumfix rows then cover `x²`, `x⁻¹`, `5‰`, `5%`, `|x|`, `‖x‖`, `√x`, `∛x`: superscript_power,
  percent_ends and the circumfix functions above become rows (an extension adds a row, not code).
- Number-form glyphs are no operators: `½`, `π`, `0x…`, `0.(3)`, glued units `5ms` want an atom table instead,
  glyph or suffix → the function that reads the literal (the atom glyph and atom suffix hooks above).
- Keyword handlers in this area: c_style_for, `times`, named loops, `down to`, foreign_forms' modifiers and `new`,
  injection's `sql`/`sh` text prefix, shader and wasm raw blocks, the guard words (GUARD_MARKERS: try, assert,
  check). Each is a name → handler entry; the statement head, declaration, raw block and text prefix hooks above
  are that table's kinds.
- Not now (card apply-lessons, Later): moving binding_powers and the keywords out of parser.warp, fusing the
  passes' walks.

Every spelling stays in the language (decision self-source-minimal): a row leaves the core into an extension, never
out of warp. Where the compiler's own source is the only reason a form stays core, a minimal change to that source
(radix constants above; the postfix/phrase row once measured) lets it go.

Open before moving: the measurement must confirm "no" for each row (a form inside a text the compiler builds at run
time counts too); the declared-operator row moves only with rewrites.warp's typed-operator rules, which the sugar
needs.

## units and ± (warp-units, static reading 2026-11-03)

Units and ± are libraries already: lib/units.warp (273 lines; `5km` is the number times the unit `km`, implicit
multiplication, a core form every number glued to a name uses), lib/unit_names.warp (99), lib/uncertain.warp (134;
`6 ± 1` is its operator_±, a glyph operator as any declared one). What core/ holds for them is small and scattered:

| piece | where | lines | compiler uses | hook it needs |
|---|---|---|---|---|
| the degree sign: `30°`, `°C`, `°F` read as a unit name, `x°` as `x * °` | parser.warp atom's DEGREE branch, the name° case after a word, DEGREE | ~14 | no (`°` only in the constant) | **atom glyph** (parsing row above): `°` a registered glyph whose atom units reads |
| ± as an operator: the node, its spelling, its binding power, the `+-` alias | node.warp Op.PlusMinus; node_cases.warp "PlusMinus"; parser.warp ALIAS_TARGET_NAMES `+-`, binding power 140/141 beside + and -; runtime/types.warp and runtime/nodes.warp operator name lists | ~6 entries | no | **operator table** (parsing row above): ± a declared glyph operator of uncertain, its precedence that of + and -; Op.PlusMinus stays for the seed until remove-unused |
| ± as a value kind | analyzer.warp:192 `op == "±"` gives ValueKind.Data | 1 | no | goes with the operator table entry: the declared operator's result type, not a core special case |
| which module's ± wins: units' overloads uncertain's (`5m ± 1cm` brings units) | prelude_words.warp:705 definer order | ~5 | no | stays: the general rule of module definers, units only its example |
| `use units` linked prebuilt; a unit's expected type names its module (`sleep(duration: Time)` → units) | compiler.warp:157, prelude_words.warp 838-879 | — | — | stays: general module machinery, units the example in its comments |
| `c` kept from units' speed-of-light rule | rewrites.warp:57 | 1 | — | stays: a general guard of a name the program binds |
| calendar time units in literals (`2 weeks`, `1 month`) and Time values | parser.warp TIME_UNITS, time_unit_named, with_time_unit (in the time-literals row above); emitter/times.warp (147, Time kind, unit_nanos) | 147 + row above | check: the compiler's `timed` reads the clock, never a time literal | moves with the time-literals row: **atom suffix** plus the Time kind's emitter part as one extension |

Not units, though named so: index_positions.warp (179, index vs position, decision index-units) and UNIT_WORDS /
counts_units (bytes, chars …: what a count counts) are the core's ints; operators.warp marks (`marked`, `marks_kind`)
carry bool and int-type info for every program, so they stay.

Plan once the hooks exist: the degree glyph and the ± operator entry move behind the atom-glyph and operator-table
hooks (about 20 lines leave the core, no spelling lost); lib/units and lib/uncertain stay libraries; the module-order
and expected-type rules stay core as general rules. Time literals with their emitter part are the one larger unit-like
piece (~250 lines), owned by the parsing row's time entry. Nothing here needs the compiler's own source changed.

## Rules (user 2026-11-03)
- The compiler's own source may change minimally where it helps a lot (e.g. it stops using a sugar form so the form can leave the core).
- The language is never simplified to shrink the core: every spelling programs may write stays (decision self-source-minimal).

## Lessons from wasp (C++, ~/dev/apps/wasp/source), 2026-11-03
Sizes: parser 1.8k (Wasp.cpp + literals) vs warp 6k; analyzer 3.8k (Angle, Keywords, Operator/FunctionAnalyzer,
Context, NodeTypes) vs warp passes + analysis 12.6k; emitter 6.3k vs warp emitter + runtime 10.5k.
- The parser knew only structure (the data format); operators and keywords were grouped later. warp's parser.warp
  (5k) holds keywords (34 quoted ones) and binding powers: candidates to leave the parser for tables.
- Operators were rows, not code: prefix/suffix/infix lists, right-associatives, precedence (², ⁻¹, ‰, √ …).
  A row per operator is what an extension should add.
- One analyze() dispatcher: keyword → group* handler (if, while, for, class, import …), ~120 lines. As a
  name → handler table it is the static hook table core-is-self wants.
- Avoid its awkward parts: string compares spread through the if-chain, global mutable state, a hash memo against
  re-analysis, html/js special cases inline, order-sensitive groupTypes → Declarations → Calls → Operators.
- warp has 35 passes, 32 walking the tree on their own; wasp did one recursive walk. Fusing passes into one
  dispatch walk shrinks code and time.
