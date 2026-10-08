# Reflection: one model for every metadata word (card reflection, user 2026-10-08)

Decision (notes/decisions.md "Reflection streamlined"): `dir(x)` and the words below answer at compile time when the
target is known, else at run time from the module's WASM metadata. Lookup order field → meta → reflection. One warp
custom section, `wasp.meta`, kept unstripped.

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
| `f.params`, `f.signature` | function | parameter names / `(a:int, b:int) -> int` | "undefined function: params" |
| `f.effects` (= `effects of f`) | function | effect set | `effects of f` works |
| `event.listeners` (= `listeners of e`) | event | handler count/list | `listeners of tick` broken (signal_listeners undefined) |
| `module.exports` | `use wasm`, foreign module | export names | missing (dir-runtime) |
| `x.unit` | quantity | its unit | static units (static_units.rs), no `.unit` word yet |
| `x.doc` | binding, function | its comment (P114, `use comments`) | `x.@comment` exists |

## Rule
`x.word` is, in order: x's real field `word` (a class or map field always wins), its meta entry `@word`
(meta_entries.rs), then the reflection word. A program's own definition of the word (`fields(x) := …`) wins as usual
(library_words defined_names). One lowering pass, `src/lowering/reflection.rs` (absorbing introspection.rs), rewrites
`x.word` to the function form (`type(x)`, `effects of f`, `listeners of e`), so each word has one implementation.

## Sources
1. Compile time (free): the analyzer knows the static type of x: class layouts (fields, methods), function
   signatures, effects (effects.rs), units (static_units.rs), module words (modules.rs std_module_definitions). The
   answer is a constant list/text.
2. Run time, only when the static type is unknown (`any`, a value read from data, a loaded .wasm): ONE host call reads
   the module's `wasp.meta` section (and the name section for exports), cached per module.
3. Warp objects (maps, Nodes) answer from their own keys at run time (`keys`, already there).

## wasp.meta layout
Wasp notation text (parsed by the reader we already have), one map:
`{units: "km:1 h:-1", classes: {P: {fields: [x y], methods: [norm]}}, functions: {f: {params: [a b], signature: "…", effects: [IO]}}}`.
`units` holds today's wasp.units text unchanged (warp-worker/warp-99 switches both ends in one commit once this is fixed).

## Steps (each a small branch)
1. reflection.rs: the dot forms → existing function forms (`x.type`, `f.effects`, `e.listeners`); fix `listeners of e`.
2. Compile-time objects: `x.class`, `x.fields` (+ aliases), `x.methods`, `dir(object|class|map)`. Done:
   `reflection::lower_objects`, a source pass right before class_methods::lower (class bodies still as written; the
   layout from class_methods::class_layouts, instances from instance_classes: constructions, `p:P` annotations and
   typed parameters). Not yet: instances only the analyzer infers (a call's result), `fun P.sum()` extension methods
   in `methods`.
3. Functions: `f.params`, `f.signature`.
4. `x.unit`, `x.doc` (aliases of meta/units), `module.exports` for `use wasm` modules (compile time: their exports).
5. wasp.meta section (absorbing wasp.units) + the run-time host call for `any`-typed values; playground reader too.
