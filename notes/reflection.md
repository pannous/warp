# Reflection: one model for every metadata word

Roadmap card: reflection-words (notes/roadmaps.md). `dir(x)` and the words below answer at compile time when the
target is known, else at run time from the program's own classes or the module's WASM metadata. Lookup order
field → meta → reflection. One warp custom section, `warp.meta`, kept unstripped.

## Words
| word (aliases) | on | answer | how |
|---|---|---|---|
| `dir(x)` | module, object, class, map | module words / fields + methods / keys | compile time for a known instance, class or map literal's variable; any other value (`dir pi`) a map's keys at run time, else no names, never undefined (reflection.rs lower_unknown_dir) |
| `dir(f)`, `type(f)` | user function, `f` or `&f` | `["params" "signature" "body" "effects"]` / `function` | the reflection words take the function itself, never its call |
| `dir(job)` | task (`job = go f()`) | `["await" "stop" "pause" "cancel" "resume" "finishes"]`, no wait for the task | declarations::Tasks::task_dir |
| `x.type`, `type(x)` | any value | its type name (`int`, `P`) | a value known only at run time: field-read fallback (list_ops::FIELD_FALLBACKS) → node_type_name |
| `x.class` | instance | its class, `P` | known instances; else a type test over the program's classes |
| `x.fields` (`attributes`, `members`) | instance, class, map | field names, declared order, inherited first | a map's keys at run time |
| `x.methods`, `methods(x)`, `methods of x` | module, instance, class, value | the functions themselves (`function sq` each) | without constructors and derived methods; a std module's words (introspection.rs), a value's type module; an imported .wasm module's class gives names |
| `x.keys` | map, object | keys | library word |
| `x.meta`, `x.@key` | any binding | meta entries (meta_entries.rs) | `x.meta` under `use comments` |
| `f.params` (`parameters`), `f.signature` | function | parameter names / `(a:int, b:int) -> int` | user and library functions (step 3) |
| `f.name`, `f.return_type`, `type(f)` | function, function value | `gcd` / `int` / `function: (a, b) -> int` | compile time in reflection.rs; on a function value at run time `.name .doc .signature .return_type .dump` are field fallbacks over the closure targets (closures.rs FUNCTION_WORDS); `print f` shows the doc as `#` lines and the definition (function_dump), in a list its name |
| `f.dump` | function | the whole definition, to copy and paste: `area(width:int, height:int) := width*height` | Node::serialize spacing |
| `f.effects` (= `effects of f`) | function | effect set as texts (`"IO"`, `["IO" "ask"]`) | a value anywhere |
| `event.listeners` (= `listeners of e`) | event | handler count/list | a name nothing listens to is a loud error |
| `module.exports` | `use wasm`, foreign module | export names | imported core modules and components |
| `x.unit` | quantity | its unit | not yet: waits for quantities in variables (notes/units_runtime.md) |
| `help(x)` | class, instance, function, module | `class P extends Q` / `fields: …` / `methods: …`; a function's signature and doc | compile time; see "help" below |
| `x.doc` | binding, function | its comment; a function's doc string | `x.doc` = `x.@comment`; a function's leading text statement or the comment above its definition, without `use comments` (doc_strings.rs), `""` without |

## Rule
`x.word` is, in order: x's real field `word` (a class or map field always wins), its meta entry `@word`
(meta_entries.rs), then the reflection word. A program's own definition of the word (`fields(x) := …`) wins as usual
(library_words defined_names). One lowering pass, `src/lowering/data/reflection.rs`, rewrites `x.word` to the
function form (`type(x)`, `effects of f`, `listeners of e`), so each word has one implementation.
Text holes: interpolation is the first source pass, so reflection words work inside `\(…)`.

## Sources
1. Compile time (free): the analyzer knows the static type of x: class layouts, function signatures, effects
   (effects.rs), units (static_units.rs), module words (modules.rs std_module_definitions). The answer is a constant.
2. Run time, only when the static type is unknown: a value of one of the program's own classes (`f(o) := o.fields`,
   `dir(xs#1)`) needs no host call: a type test over the classes picks the names (Objects::dispatched; a class's own
   member of that name is read as written; the last else is the field read, for `dir` the keys). Values from outside
   the program would need ONE host call reading the module's `warp.meta` section, cached per module: not in either
   host yet.
3. Warp objects (maps, Nodes) answer from their own keys at run time.

Instances come from instance_classes (constructions, `p:P` annotations, typed parameters); extension methods
(`fun P.twice()`, `extension P {…}`) are listed in `methods` and `dir` (declarations::extension_methods).

## Functions (step 3)
Constants from the user function's definition: a parameter's type is its annotation or the kind the body demands,
the result the inferred kind (`def g(x){x*2}` is `(x) -> int`). Library words (`sqrt.signature`, `first.params`,
`dir(sqrt)`, `help(upper)`) answer from lowering/foreign/library_signatures.rs: a hand table names and types the
operator, rounding and text/list words (result `number` where it depends on the argument), a prelude word shows its
lib/prelude.warp head, libm, text builtins and runtime words get positional names (`(a:float, b:float) -> float`).
A function's comment survives lowering through `Node::with_meta_of`.

## Modules
`m.exports` and `dir(m)` of an imported core module (`import lib/fourty_two`): reflection::lower_module_words after
modules::resolve, sorted names from wasm_modules::exports. Components (`use wasm "x.wasm" as lib`): from the compiled
component (components::export_names, functions and resource types of it and its exported interfaces, snake_case),
reflection::lower_component_words before foreign_modules. In the page, wasm_modules::component_export_names reads
them with wasmparser's validator (no wasmtime). Sample samples/wasm/component_reflection.warp.

A compiled warp module's `warp.meta` carries `functions` and `classes` (below). An importing program (`import
"adder.wasm"`) reads them at compile time: `adder.exports` are the module's own functions, `adder.add.params`,
`.signature`, `m.P.fields`, `m.P.methods`, `dir(m.P)` (Objects::module_function_word / module_member_word). The
page's compiler reads them the same way (web::read_bytes). emit_names writes the parameters' local names, so named
arguments reach a warp module's function (`twice(a: 21)`). Fixture tests/fixtures/wasm/shapes.wasm from
probes/reflection_classes/shapes.warp. Not yet: constructing or reading the module's instances (`shapes.P(1, 2)`,
card import-compiled).

## warp.meta layout
Warp notation text (parsed by the reader we already have), one map:
`{units: "km:1 h:-1", classes: {P: {fields: [x y], methods: [norm]}}, functions: {f: {params: [a b], signature: "…", effects: [IO]}}}`.
pipeline::compile_program writes it (reflection::meta_entries: the lowered program's functions minus class methods,
class layouts from the source, inherited fields first) in one meta_section::with_entries. Names are symbols
(`[x y]`); a one-letter text reads back as a character. `functions` holds only the functions the source defines, not
the html_*/prelude ones lowering brings in; a hello world writes no section (web::test_bundle_budget leaves a few
dozen bytes of headroom). Likewise list_text carries the operators' texts only when the program makes a Key with an
operator beyond `:` (Need::KeyOperator).

## help
`help(sound)` is the module's index (std_docs::module_help, as `warp help sound`), `help(midi)` the word's head and
comment, answered before reflection.rs (src/lowering/data/word_help.rs). After modules::resolve, `help(x)` and
`function(x).dump` of a name known only at run time look x up in a map literal of the used modules' words plus the
program's functions. `function trick` is the function reference (P82); where trick is a variable, its value is the
name. std_docs indexes `global x = …` and `def f(x) {…}` too; a prelude word's help stays its head.
`help(x)` on its own line (not the value of a block or program) prints: it is `print help(x)`
(word_help::printing_help_statements), in a loop's body the last statement too. `t = help(x)` stays silent.
