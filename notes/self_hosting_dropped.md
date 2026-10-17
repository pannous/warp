# Forms the warp-written parser drops

lib/compiler/parser.warp is the cleanest, most minimal parser, not a copy of src/warp_parser (decisions
self-hosting-minimal and self-parser-drops, 2026-10-17). The forms below the Rust parser reads and parser.warp does not.
The sources that need them are listed in tests/parser/self_parser_dropped.txt (path, tab, form); the survey
`parser_differences` (tests/parser/test_self_parser_survey.rs) skips them, so its count measures the kept forms.
`probes/self_hosting/parser/sort_differences.py` sorts the remaining differences into kinds of forms.

## Comes back as a library data DSL
Code and data are one: once the parser reads plain warp data correctly, these need no syntax of their own.

| Form | Example | What it takes |
|------|---------|---------------|
| Music `go {…}` blocks | `go { c4 e4 g4 }` | a sound library function taking data, `tune d:data := play d`, so `tune { c4 e4 g4 }` plays plain data; note names are constants of the music/sound library (card music-data-dsl) |
| CSS in style blocks | `style { td#x { color: red } }` | `style` reads the block as data: selectors as keys, declarations as pairs (card style-data-dsl) |

## Parser forms to restore later
| Form | Example | Why dropped | What it would take |
|------|---------|-------------|--------------------|
| Other languages' syntax | `println!("{}", x)`, `(x for x in xs)`, `yield from g()`, `xs.each { \|x\| … }` | compatibility reading, not warp | port warp_parser/rust_macros.rs, the Python generator and Ruby block forms (definitions/foreign.rs) |
| Phrase definitions | `to get value of coin in cents: …` | experimental natural-language heads | port the phrase head reading (phrase_definitions in warp_parser) |
| Shader holes in WGSL text | `shader "…{x}…"` with `shader_holes` | experimental GPU text | port the hole scan for shader texts |
| `$a` pattern names | `match n { data($a + 0) … }` | edge form of node patterns | a `$` name atom |
| Error nodes for malformed input | `y = [1 2` → "`]` is missing" | come back with the diagnostics port | the parser's error nodes and their texts |

# Pass twins: fringe forms a warp twin leaves out
The port keeps the clean core, not every Rust case (user 2026-10-17). A twin may skip a fringe form when it is
recorded here with how to bring it back; the harness (tests/common/twins.rs pass_differential, `dropped`) skips the
named sources and refuses a dropped source this file does not name. Passes that look like bloat as a whole are listed
to the Supervisor instead of ported.

Format: `## <part>` (lib/compiler/<part>.warp), then per form: the form, the sources showing it, why it is left out,
how to bring it back.

## passes (SUGAR_PASSES, card sugar-twins; decision sugar-twins-drops)
- sql and sh templates: `sql "SELECT … $name"`, `exec sh "ls"` as parameterized calls (src/text/injection.rs
  lower_templates, run first by interpolation.rs lower_program). Sources: probes/docs_sync/sandbox_exec.warp,
  probes/sandbox/exec.warp, samples/system/crontab.warp. Left out for now: a library feature, not text building.
  Back: port lower_templates in front of lib/compiler/interpolation.warp.
- Rust Option forms: `if let Some(v) = o { A } else { B }`, `match o { Some(v) => A, None => B }`, `o.unwrap_or(d)`,
  `o.expect(m)`, `o.is_some()`. Sources: probes/rust_reading/option.warp. Left out: warp's own forms say it
  (`if o != ø`, `o ?? d`, `o ?? raise(m)`). Back: port src/lowering/welcoming/rust_options.rs as a pass before the match
  arms.
- `set x to 5`, `set width of p to 12` (decision setter-calls: `p.width = 12` with a style hint). Sources:
  probes/english_setter.warp. Left out as a compiler pass: comes back as a library code template, not a pass (card
  set-set). Back meanwhile: port welcome_forms.rs lower_set_phrases after `a of b`.

# Deviations: where a warp twin keeps the clean result instead of Rust's
Rule: notes/self_hosting.md (matching Rust is a check, not the goal). Each entry: the difference, an example, the card
or question for the Rust side.

## emitter (lib/compiler/emitter.warp)
- Value layout: typed values inside and a boxed $Node only at the edges; texts as GC byte arrays, lists as vectors
  (notes/self_hosting.md "Value layout"). Rust boxes nearly every value as a $Node, keeps texts in linear memory as
  (ptr, len) $String and lists as cons cells. Example: `xs#500` walks 500 cells in Rust's module, one array.get in
  warp's. Same program results. Rust side: card rust-emitter.
