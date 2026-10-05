# Uniscript web page

https://pannous.com/uniscript/ — `web/uniscript/`: a textarea whose uniscript is converted live by the uniscript.wasp of the uniscript package (github.com/pannous/uniscript)
compiled to WASM GC by warp (no JS reimplementation), and a reverse box (`unicode_to_uniscript`).

## Build and deploy

```
web/uniscript/build.sh           # warp compile → uniscript.wasm, copies packages/uniscript/data/entities.idx, fonts → slices
web/uniscript/build.sh deploy    # … and rsync to pannous.com:/var/www/pannous/uniscript/
cd web/uniscript && python3 -m http.server 8765   # local
```

https://pannous.com/uniscript/rust/ is the uniscript repository's second page (its `docs/demo.html`, the Rust crate compiled
to WebAssembly, deployed by its `docs/make_demo.sh deploy`). The two pages link to each other, and that page loads its fonts
from this page's `fonts/`. No deploy script into /var/www/pannous uses `rsync --delete`: one wiped the other's files.

Committed: `index.html`, `uniscript.js`, `uniscript.wasm`, `build.sh`. Not committed (`.gitignore`): `data/` (a copy of the
3.4 MB index) and `fonts/` (slice_fonts.py: UniscriptSans whole, UniscriptCJK and NewGardinerOmni2d4 sliced, all OFL; from
the uniscript repository's `fonts/dist` (`UNISCRIPT_FONTS`) or `~/Library/Fonts`). The Monaco/Menlo mirror fonts are Apple fonts and are never copied.

## How the JS host works (`uniscript.js`)

- The module's `main` is its init: it calls `host.read("packages/uniscript/data/entities.idx")` synchronously. Host calls are
  synchronous, so the page fetches the index before instantiating; any other path falls back to a synchronous XHR
  (charset `x-user-defined` keeps the bytes).
- Imports come from `WebAssembly.Module.imports()`: `host.read`, `host.fetch`, `host.fetch_within`, `host.warn` are real
  (`warning(message)` in wasp lists under the output of the conversion that raised it, cleared on the next one),
  `host.run` and anything unknown warn in the page and return 0 / -1.
- Bytes go into memory by the rule of `src/host.rs write_bytes_to_caller` (the exported `text_heap` bump pointer, fresh
  pages when it is 0 or full), then `new_text(ptr, len)`.
- Results: JS cannot read WASM GC struct fields, so warp now exports `get_text_ptr` / `get_text_len` next to `get_kind`
  (the `$String` of a Text, Symbol or Error, 0 otherwise). `tests/text/test_text_getters.rs` does the same in wasmtime.
- After each conversion `text_heap` is reset to its value after init: `out += …` allocates a fresh text per append, and
  the results are copied out immediately, so memory does not grow with typing.

## Status

- Verified headless (agent-browser, Chromium) locally and on the public URL: all examples, typing, the reverse box,
  errors (`<:nosuchthing>` in red), fonts render colors, mirror/turn/left/right, IDS composition (⿰木木 → 林) and
  hieroglyph groups.
- Firefox and Safari not tested (agent-browser drives only Chromium); both ship WASM GC.
- Nested tags (`<:above 木 <:beside 木 木>>`) are not supported by uniscript.wasp: a tag ends at the first `>`.
- Greek blocks join their words (spaces separate operands): one word per `<:greek> … <:/greek>`; no final sigma (θεοσ).
- A repeated warning reports the operand's byte, not the character's: `<:greek> philosophia` warns twice "no greek form of h at byte 8".

## Sliced fonts (slice_fonts.py, sequence_fonts.js)

The whole CJK font is 10.6 MB as woff2, Gardiner 1.5 MB. `slice_fonts.py` (fonttools, wordfreq) writes `fonts/fonts.css`
with ~100 unicode-range slices per font (frequency-ordered zh/ja/ko first, then code point order) and `fonts/fonts.json`.
- Chrome shapes each unicode-range face on its own and maps only the characters its range declares (tested:
  a face whose cmap has 犭 but whose range does not, shows notdef). It tries every face whose range intersects the text,
  last declared first, and downloads each one it tries. So pure CSS cannot keep a sequence in one face: declaring the
  IDS components in the ⿰ face downloads it for plain text, and declaring TAG in every slice downloads all 91 slices.
- Sequences therefore get faces outside fonts.css, assigned by `sequence_fonts.js` (`watchSequences(elements)`: wraps each
  sequence in a span whose first font is one unranged FontFace, re-wrapping on DOM changes):
  - IDS: faces of 50 ligatures per operator, sorted by operands (1112 faces, median 21 KB). Few ligatures per face matter:
    a face holds every ligature its components can form, so 400 per face gave 2.6 MB faces. Nested IDS: the shortest
    ligature is the arity; JS parses operands recursively. Aliases: a component glyph has several code points
    (犭 U+72AD, ⺨ U+2EA8); faces contain all, JS maps operands to the first.
  - hieroglyph runs containing a format control (13430–1345F, mirror 13440 too): one face with every sign (1.4 MB).
  - base + TAG (你 mirror/color): the base's plain slice file as an unranged face (cached); slices carry TAG undeclared.
- Verified with the uniscript repository's probes/font_slices/shaping.html: sliced = whole font for all cases.
- Cold load of pannous.com/uniscript/rust/: 12.8 MB → 2.3 MB. The main page with every example clicked: fonts 12.6 → 2.2 MB.
- pannous.com gzips application/json since 2026-09-30 (fonts.json 55 → 13 KB).
- Not handled: text in textareas (no spans), and a TAG effect on a composed IDS character.
