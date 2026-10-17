# Restructuring: layout by concept

Rule (decision restructuring): every function lives in the file of its concept. A general file (parser core,
expressions.rs, mod.rs) holds no syntax of a particular keyword or construct. Split a file by concept, never for its
length: ~500 lines is fine (type_kinds.rs was split and reverted: its entries belong together). Moves are pure moves
in their own commits (widening visibility to `pub(crate)` is the only edit); true duplicates merge, measured in
net src/ lines. Inventory: data/restructuring/functions.tsv (name, file, line); earlier verdicts
data/restructuring/duplicates_exact.md, similar_names.md. The full function → file tables of the 2026-10-14 moves:
notes/OLD/restructuring_moves.md.

## Tools
- `scripts/move_fn.py`: pure move of functions, consts, statics and type aliases between files (cuts docs and
  attributes, widens what leaves mod.rs or goes deeper). Intermediate folder mod.rs files start with `use super::*;`
  and glob-import their children so `use super::*` chains see them.
- `scripts/move_module.py <module> <folder>`: `git mv` into a concept folder, rewrites every `crate::`/`warp::` use path.
- `probes/cleanup/duplicate_constants.py` finds constants holding the same text or word list;
  `scripts/merge_constant.py [--home <file>] NAME file:LOCAL…` merges them.

### Moves with many users
A module whose path many files (and open branches) use moves in two steps:
1. `scripts/move_module.py <module> <folder> --keep-paths`: uses stay, lib.rs keeps the old path alive with
   `pub use <folder>::<module>;`, no open branch breaks.
2. Once those branches have landed, one fastmod pass in a quiet window right after an Integrator batch (agreed with
   warp-integrator): `crate::<module>` → `crate::<folder>::<module>` (also `warp::` in tests), then drop the re-export.

## Parser (`src/warp_parser/`)
Core stays flat and general: `mod.rs` (WarpParser, ParserOptions, entry points, Atom), `scanning.rs`, `lookahead.rs`,
`expressions.rs` (the Pratt loop, operator diagnostics), `atoms.rs` (symbols, calls, keyword forms, glued suffixes),
`suffixes.rs`, `declaration_scan.rs`. Everything keyword- or construct-specific is in a concept file:

```
warp_parser/
  blocks.rs          {…}, do…end, colon/indented bodies, trailing blocks
  comments.rs        // # /* */ comments, the // floor-division ambiguity
  commands.rs        print and braceless commands `sleep 1s`
  pipes.rs           |>, f.(xs) broadcast
  math_notation.rs   superscript powers, |x| norm bars
  annotations.rs     @name, @name(value), Ruby @x
  control/           conditions, if_else, loops, switch, guards (try/catch/finally/fallback), jumps (return, await, emit)
  definitions/       functions, lambdas, variables (global/nonlocal/constants/<-), foreign, operators (user operators)
  types/             declarations (class/struct/enum/sum types), expressions (type applications, list<T>, T[])
  collections/       lists, subscripts, ranges
  literals/          mod.rs (parse_value), numbers, units, text, shaders, constants, paths (paths, URLs, versions), data
  markup/            xml, css
```

## Root (`src/`)
Stay at the root: lib.rs, main.rs, pipeline.rs, util.rs, extensions.rs + extensions/, words.rs, type_kinds.rs,
law.rs + law/, host.rs + host/, modules.rs + modules/. Folders: cli/ (main.rs's flags, outputs, executables, display,
serve, console), compiler/ (compile context and mode, analysis memo, locals, functions, meta, dead functions,
run_time_blocks), errors/ (diagnostic/, fixits, crash_card, run_time), normalize/ (hints toward canonical syntax),
operators/, numbers/, time/, text/, formats/, modules/ (wasm_modules/, components, cargo_crates/, package tools, std
docs), host/ (tasks/, channels, sandbox, database, fetch), run/, web/, graphics/, sound/, gc_traits/, ffi/.
Each root file or mod.rs re-exports its children, so most `crate::…` paths stayed.

## Lowering (`src/lowering/`)
Only folding, nodes and pipes stay flat: they serve every folder. Folders: control/, definitions/, types/
(class_methods/, traits/, inline_unions), collections/ (list_mutations, filled_lists), words/ (library_words/ with
function_methods, guards, similarity, templates), welcoming/, web/, signals/, data/ (database_tables/, reflection,
meta_entries), foreign/ (gpu_maps/). Each folder mod.rs does `use super::*`; the crate root re-exports moved modules.

## Analyzer and emitter
Analyzer (`src/analyzer/`): by concept (annotations, booleans, counting, imports, inference, list_views, main_globals,
upcast_fields, user_functions/, variables, checks/, effects/); keyword-specific checks leave mod.rs/inference.rs.
Emitter (`src/wasm_emitter/`): core files flat (mod.rs, module_finish, type_manager, import_manager, globals, layout,
config, declared_values, constructors, values, diagnostics, function_builder, string_table) plus numbers/, text/,
lists/, maps/, time/, control/, functions/, host/, reflection/; folder modules are glob re-exported. mod.rs keeps the
module driver and the Node list encoding; `kind_with_article`/`with_article` wait for a words/ home.

## Same names, different jobs (don't merge)
`aliased_method`/`method_aliases` (a class's own method vs. another language's name), `keyword_definition`
(definition vs. head), `map_key` (Lean model vs. emitter), `start_of` (parser cursor vs. node position). The
`is_type_word` sets are named by set: `is_builtin_type_word`, `is_kinded_type_word`, `is_kinded_type_or_any`,
`is_kinded_or_testable_type`.

## Constants
Language words live in src/words.rs (`<NAME>_WORD`, `IMPLICIT_PARAMETER`, ANY_TYPE). Distinct despite equal text,
kept with their names: a role differing from the spelling (`VALUE_FIELD` vs. `VALUE_WORD`, `·` as UNIT_PRODUCT vs.
NAME_SEPARATOR, `"wasm"` as runtime vs. extension, the two MODULE_EXTENSIONS), SWIFT_IN, KERNEL_COUNT, AWAIT_ALL_WORD,
MINIMUM_KEYWORD, STATE_KEYWORD, LET_TOPIC, field and parameter names that spell a word (NUMBER_FIELD, ERROR_FIELD,
LIST_PARAMETER, MAP_PARAMETER, LIST_ANSWER, TEXT_HOLE, ANY_STEP), EXTREMA (pairs words with operators).
Left to merge, each into one home with `--home`: UNION_JOINER, MODULE_FILE ×3, JSON_TYPE/TEXT_TYPE, STACK_EXHAUSTED,
MISSING_TARGET_SIGN, WARP_CALL, FIRED_PREFIX, STYLE ×6, EXTREMA/EXTREMUM_WORDS, CHANGE_WORDS ×2
(`probes/cleanup/duplicate_constants.py` shows which are still there).

