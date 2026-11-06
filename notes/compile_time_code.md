# Compile-time code in extensions/rules (measurement and proposal, 2026-11-06)

Commit 463dfdefa0 moved the compile-time rule code from lib/ to extensions/rules/. As a result the compiler carries
extensions/rules/rule_words.warp (1017 lines) and extensions/rules/wgsl.warp (864 lines). Nothing has been built yet;
this note only measures and proposes.

## (a) wgsl.warp: the translation can run at run time
The file has two parts:
- **Lines 1–774: the warp → WGSL translator.** paint/shader_of reach it through two rule words, wgsl_shader and
  wgsl_painting, called from extensions/rules/modules/prelude.warp:11–12. The translator turns a function and its
  callees into WGSL text, and the values it reads into the uniform map. Nothing in it rewrites the program.
  - Decision shaders-in-lib (notes/decisions.md:650) already asks for this layout: "the WGSL translator is
    lib/shaders.warp; `paint(f, w, h)` hands f's quoted definition, callees and globals (uniforms) to it". Today's
    compile-time version departs from that decision.
- **Lines 775–864: GPU lists** (gpu_bound, gpu_packed …). These rewrite how the program makes a list, so they stay
  compile-time. About 90 lines, which could move to extensions/gpu_lists/ where the rest of that feature lives.

What the run-time version needs:
1. **One generic rule word.** `definitions_reached($f, $program)` returns `data {…}`: f's definition, the
   definitions of the functions it calls (transitively), and the names of the program values it reads.
   - The rule becomes `paint($f, $w, $h) => paint_shader(definitions_reached($f, $program), $w, $h, $f)`.
   - The word is about 20 lines in extensions and is ordinary reflection. help, doc and full_definition could use it
     too, as could sending a function to a worker.
   - This is not core: quoting (`data …`) and run-time Node walking already exist (test/node/test_parsed_node_walks).
2. **lib/shaders.warp**, which is wgsl.warp ported to run-time node words.
   - It calls the compiler's view words: function_definition, body_of, definition_parameters, level_statements,
     plain_node(parsed(…)) and is_big_number_view. Over a quoted `data {…}` these become plain node walks, and the
     result is text, so no parse is needed.
   - The value widths no longer need inferring: the translator reads the actual uniform values at run time.
   - The translation runs once per painted function (cached by name) on the first paint. It costs milliseconds against
     a frame's GPU work.
3. **Costs.** A painting program links the translator module, roughly 770 lines of warp, but only when it paints. The
   browser playground runs the same lib code, so host-gpu.js does not change.
4. **Core:** none. The shader_values, shader_values_of and paint_size rule words (12 lines) fold into the same rule
   or into lib.

Result: the compiler sheds about 800 of wgsl.warp's 864 lines. The rest are the GPU lists plus one reflection word.

## (b) rule_words.warp: 63 registered words, 135 definitions
**Dead:** none to speak of. Every registered word is called by a rule, and every definition is referenced. One table
line, `path_parameters: path_parameters`, is dead: no rule names it directly, and route_bindings calls the word
itself. That saves 1 line.

Lines by domain (counting non-blank lines):

| part | lines | rule words | callers | fate |
|---|---|---|---|---|
| reflection, help | 263 | help/doc_of/signature_text/dir_words/body_of/definition_text/class_help/module_help/effects_named/return_kind/misnamed/… | sugar/reflection.warp | language core of reflection: **stays**; later partly run time over `definitions_reached` |
| signals, events, pages | 185 + 33 | whenever/once/change_listens, signaled, polled_word, page_valued | lib/signals, rules/modules/signals, timers, sugar/events | rewrites the program (notifications at writes): compile-time by nature, **domain** |
| typed JSON | 102 | typed_json | rules/modules/json | **run time**: lib/json reads `T.field_types` (a generic reflection word: fields, types, defaults) |
| units | 97 | unit_bound/led/regrouped, compound/leading/converted_unit, units_shown | lib/units, rules/modules/units | **domain** |
| lints, small sugar | 79 | then_not_pipe, then_without_if, hyphen_difference, shorthand_closure, counted_question, bare_fields_read, declared_local, module_named | sugar/* | generic language: **stays** |
| table + polled_word | 73 | — | — | infrastructure: **stays** (shrinks with the words) |
| storage | 45 | tables_kept | rules/modules/storage | **domain** |
| laws | 35 | law_check | rules/modules/laws | **run time**: a generic rule word `free_names($e)` turns the law into a lambda, and lib/laws tries it (`law_holds(names, (x) => …)`) |
| tasks, frames | 30 | tasked, task_steps, awaited_operand, end_refused, framed, exited | lib/tasks, rules/modules/tasks, frames | **domain** |
| web routes | 20 | route_verb, route_bindings, served | lib/web, rules/modules/web | **domain** |
| GPU | 12 | gpu_lists_packed, gpu_list_made, shader_values(_of), paint_size | rules/modules/prelude | goes with (a) |

- **Must stay (~415 lines):** reflection, lints and small sugar, and the table. These are the language itself. A
  generic rule language only grows if they leave.
- **Can go to run time with two generic rule words (~140 lines):** typed JSON via `T.field_types`, and laws via
  `free_names`.
- **Domain words (~410 lines):** units, signals/pages, storage, tasks/frames and web. Their logic does arithmetic and
  text work on code: splitting a path at `:name`, a unit's prefix and power, the variables a block writes.
  - Small generic rule features (text words in `where`) would cover only web (20 lines) and part of units.
  - The real lever is **module rule words**: a lib module defines its own rule words (`lib/units.warp`:
    `rule unit_led(…) := …`), compiled once into a small wasm of its own. When a rule calls such a word, the compiler
    asks the host to run it, and nodes cross in the parse-code encoding that extensions/parse_words.warp already writes
    and core/native/driver.warp already reads.
  - That is **core**: computed() in core/passes/rewrites.warp, a host word in the driver, and its twin in the
    playground host. In return it ends the growth for good, because every future domain brings its own compile-time
    code.

## Recommendation
1. **(a) now, no core:** add `definitions_reached` as a rule word, port the translator to lib/shaders.warp as decision
   shaders-in-lib says, and move the GPU-lists tail to extensions/gpu_lists/. Saves about 800 lines.
2. **(b) now, no core:** typed JSON and laws move to run time via `T.field_types` and `free_names`, saving about
   140 lines. Drop the dead table line.
3. **(b) later, core, needs the user's OK through the Interviewer:** module rule words, to take the ~410 domain lines
   out to their libraries. Until then they stay, grouped by domain, possibly split into extensions/rules/<domain>.warp
   files so each one's size is visible to scripts/vitals.py.
- In total, the compiler's 1880 lines become about 540: 415 that stay, about 30 for the three generic words, and the
  90-line GPU-lists tail. Without step 3, about 950.
