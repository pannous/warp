# A WebAssembly module's own warp words: the `warp.words` section

A core module a program uses (`use x.wasm`, `use cargo strsim`; extensions/c_imports/wasm_modules.warp) links its
exports as typed imports: numbers only (i32, i64, f32, f64). Texts cross through warp words the module carries itself.

## The section
- A custom section named `warp.words`, UTF-8 warp source: one definition a line, `//` lines are comments.
- The using program compiles those definitions with itself, as if written at its `use` line, so they call the module's
  exports by name (each export is declared as a foreign import from the module's path).
- An export or a word whose name holds `__` is the module's own: dir(m) and m.functions list the others, and
  `m.word(…)` calls any of them.

## Texts across, by convention (scripts/warp-crate wasm writes it; a hand-written module may follow it)
For a module prefix `<p>` (warp-crate: the manifest's library, `rs_strsim`):
- `<p>__alloc(length: i32) -> i32` room for a text in the module's memory, `<p>__store(at, byte)`, `<p>__load(at) -> byte`
- a text parameter is two i32s, its bytes' place and length; a text result is one i64, place << 32 | length
- the words copy byte by byte through store and load:

      rs_strsim__text_in(t: text): int := { bytes = t.bytes; at = rs_strsim__alloc(#bytes); for i in 0 ..< #bytes { rs_strsim__store(at + i, bytes[i]) }; at }
      levenshtein(a: text, b: text): int := rs_strsim__levenshtein(rs_strsim__text_in(a), byte_size(a), rs_strsim__text_in(b), byte_size(b))

Only i32/i64 calls cross, so the same module works in the native warp (wasmtime linking) and in the page (JS
instantiation) with no host change and no second memory in the program.

## Crates: `scripts/warp-crate wasm lib/crates/<crate>.warp`
The crate manifest's foreign lines (notes/crate_adapter.md), built for wasm32-unknown-unknown with the section above
(Rust `#[link_section = "warp.words"]`), written to lib/crates/<crate>.wasm, tracked, which `use cargo <crate>` links.
Free functions on numbers, bool and text only; objects (regex.Regex, semver.Version) wait for a handle design
(decision samples-modules-wasm).

## Left
- Bulk copying: a byte a call is fine for short texts; a long one wants memory.copy through an imported memory
  (emitter work), card wasm-text-bulk-copy.
- The texts a module allocates are never freed (a `<p>__free` and a call after each use).
