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
| radix numbers `0x…`, `0b…` | parser.warp radix_value, digit_value | ~30 | **yes**: hex in core/commands/commands.warp (MACHO_MAGIC …) | stays core |
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

Open before moving: the measurement must confirm "no" for each row (a form inside a text the compiler builds at run
time counts too); the declared-operator row moves only with rewrites.warp's typed-operator rules, which the sugar
needs.

## Rules (user 2026-11-03)
- The compiler's own source may change minimally where it helps a lot (e.g. it stops using a sugar form so the form can leave the core).
- The language is never simplified to shrink the core: every spelling programs may write stays (decision self-source-minimal).
