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
# Dropped in the port: fringe forms a warp twin leaves out
The port keeps the clean core, not every Rust case (user 2026-10-17). A twin may skip a fringe form when it is
recorded here with how to bring it back; the harness (tests/common/twins.rs pass_differential, `dropped`) skips the
named sources and refuses a dropped source this file does not name. Passes that look like bloat as a whole are listed
to the Supervisor instead of ported.
Format: `## <part>` (lib/compiler/<part>.warp), then per form: the form, the sources showing it, why it is left out,
how to bring it back.
