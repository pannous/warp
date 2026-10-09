# Reflection: one model for every metadata word (card reflection, user 2026-10-08)

Decision (notes/decisions.md "Reflection streamlined"): `dir(x)` and the words below answer at compile time when the
target is known, else at run time from the module's WASM metadata. Lookup order field → meta → reflection. One warp
custom section, `warp.meta`, kept unstripped.

## Words
| word (aliases) | on | answer | today (2026-10-08, probed) |
|---|---|---|---|
| `dir(x)` | module, object, class, map | names: module words / fields + methods / keys | step 2: compile time for a known instance, class or map literal's variable; else undefined |
| `x.type`, `type(x)` | any value | its type name (`int`, `P`) | works; `p.type` of a known instance since step 2 |
| `x.class` | instance | its class, `P` | step 2: known instances (`type(p)`) |
| `x.fields` (`attributes`, `members`) | instance, class, map | field names, in declared order | step 2: inherited fields first; a map's keys at run time |
| `x.methods` | instance, class | method names | step 2: without constructors and derived methods |
| `x.keys` | map, object | keys | works (library word) |
| `x.meta`, `x.@key` | any binding | meta entries (meta_entries.rs) | works, `x.meta` under `use comments` |
| `f.params` (`parameters`), `f.signature` | function | parameter names / `(a:int, b:int) -> int` | step 3: user functions |
| `f.effects` (= `effects of f`) | function | effect set as texts (`"IO"`, `["IO" "ask"]`) | a value anywhere (card effects-value); a program whose top level does nothing answers at compile time |
| `event.listeners` (= `listeners of e`) | event | handler count/list | `listeners of tick` broken (signal_listeners undefined) |
| `module.exports` | `use wasm`, foreign module | export names | step 4: imported core modules; components natively |
| `x.unit` | quantity | its unit | static units (static_units.rs), no `.unit` word yet |
| `x.doc` | binding, function | its comment (P114, `use comments`) | step 4: `x.doc` = `x.@comment`, of functions too |

## Rule
`x.word` is, in order: x's real field `word` (a class or map field always wins), its meta entry `@word`
(meta_entries.rs), then the reflection word. A program's own definition of the word (`fields(x) := …`) wins as usual
(library_words defined_names). One lowering pass, `src/lowering/reflection.rs` (absorbing introspection.rs), rewrites
`x.word` to the function form (`type(x)`, `effects of f`, `listeners of e`), so each word has one implementation.

## Sources
1. Compile time (free): the analyzer knows the static type of x: class layouts (fields, methods), function
   signatures, effects (effects.rs), units (static_units.rs), module words (modules.rs std_module_definitions). The
   answer is a constant list/text.
2. Run time, only when the static type is unknown: a value of one of the program's own classes (`f(o) := o.fields`,
   `dir(xs#1)`) needs no host call, every class is known at compile time: a type test over them picks the names
   (`if o is P then ["x" "y"] else …`, reflection.rs Objects::dispatched; a class's own member of that name is read as
   written; the last else is the field read as written, for `dir` the keys). Values from outside the program (a
   loaded .wasm, data of another program): ONE host call reads the module's `warp.meta` section, cached per module.
3. Warp objects (maps, Nodes) answer from their own keys at run time (`keys`, already there).

## warp.meta layout
Warp notation text (parsed by the reader we already have), one map:
`{units: "km:1 h:-1", classes: {P: {fields: [x y], methods: [norm]}}, functions: {f: {params: [a b], signature: "…", effects: [IO]}}}`.
`units` holds today's warp.units text unchanged (warp-worker/warp-99 switches both ends in one commit once this is fixed).

## Steps (each a small branch)
1. reflection.rs: the dot forms → existing function forms (`x.type`, `f.effects`, `e.listeners`); fix `listeners of e`.
2. Compile-time objects: `x.class`, `x.fields` (+ aliases), `x.methods`, `dir(object|class|map)`. Done:
   `reflection::lower_objects`, a source pass right before class_methods::lower (class bodies still as written; the
   layout from class_methods::class_layouts, instances from instance_classes: constructions, `p:P` annotations and
   typed parameters). Not yet: instances only the analyzer infers (a call's result), `fun P.sum()` extension methods
   in `methods`.
3. Functions: `f.params`, `f.signature`. Done in reflection::lower: constants from the user function's definition;
   a parameter's type is its annotation or the kind the body demands, the result the inferred kind (`def g(x){x*2}`
   is `(x) -> int`). Not yet: library and host words (`sqrt.signature`).
4. `x.unit`, `x.doc` (aliases of meta/units), `module.exports` for `use wasm` modules (compile time: their exports).
   `x.doc` done (meta_entries.rs; a function's comment survived no lowering before: declarations::lower_c_functions
   and nonlocal_cells rebuilt the definition without its Meta, now `Node::with_meta_of`). `x.unit` waits for quantities
   in variables (units runtime, warp-worker's area). `m.exports` and `dir(m)` of an imported core module (`import
   lib/fourty_two`) done: reflection::lower_module_words after modules::resolve, the sorted names from
   wasm_modules::exports. Components (`use wasm "x.wasm" as lib`, card reflection-components): `lib.exports` and `dir(lib)` natively from
   the compiled component (components::export_names, functions and resource types of it and its exported interfaces,
   snake_case), reflection::lower_component_words right before foreign_modules. Not yet in the page: the compiler
   there cannot read the component (components.js loads it by name at its first call).
5. warp.meta section (absorbing warp.units) + the run-time host call for `any`-typed values; playground reader too.
   Done first: run-time names of the program's own classes by type dispatch (Sources 2). Agreed with warp-worker
   (2026-10-08): web switches static_units with_result_units / module_units and the playground reader to the `units`
   entry, text byte for byte; warp-worker owns `x.unit` once quantities live in variables.
   Done (card reflection-foreign-meta): pipeline::compile_program writes the `functions` and `classes` entries
   (reflection::meta_entries: the lowered program's functions minus class methods, the classes' layouts from the
   source) next to `units` in one meta_section::with_entries. An importing program (`import "adder.wasm"`) reads them
   at compile time, the module's path is known then, so no host call: `adder.exports` are the module's own functions
   (not warp's runtime exports), `adder.add.params` and `.signature` come from the entry (Objects::module_function_word;
   wasm_modules::qualify leaves `m.f.word` of an exported function unqualified for it). The run-time host call stays
   for `any`-typed values only. Not yet: the page reader.
   Done (card reflection-classes, on that entry): `m.P.fields`, `m.P.methods`, `dir(m.P)` of an imported module's class
   (Objects::module_member_word, which also answers `m.f.params`). The entries hold names as symbols (`[x y]`), a
   one-letter text reads back as a character; `classes` lists inherited fields first. wasm_emitter emit_names also
   writes the parameters' local names, so named arguments reach a warp module's function (`twice(a: 21)`). Fixture
   tests/fixtures/wasm/shapes.wasm from probes/reflection_classes/shapes.warp. Not yet: constructing or reading the
   module's instances (`shapes.P(1, 2)`, exports with reference types; card import-compiled).
   Size (web::test_bundle_budget, 23 KB for a hello-world site, a few dozen bytes of headroom): the `functions` entry
   holds only the functions the source defines, not the html_*/prelude ones lowering brings in; a hello world writes
   no section. The same rule holds for run-time tables: list_text carries the operators' texts only when the program
   makes a Key with an operator beyond `:` (a Need::KeyOperator, wasm_emitter/mod.rs).

Text holes: interpolation is the first source pass (card interpolation-source), so `"\(f.params)"` and every other
reflection word work inside `\(…)` like plain statements.
