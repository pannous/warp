# Uniscript web page

https://pannous.com/uniscript/ — `web/uniscript/`: a textarea whose uniscript is converted live by `lib/uniscript.wasp`
compiled to WASM GC by warp (no JS reimplementation), and a reverse box (`unicode_to_uniscript`).

## Build and deploy

```
web/uniscript/build.sh           # warp compile → uniscript.wasm, copies data/uniscript/entities.idx, fonts → woff2
web/uniscript/build.sh deploy    # … and rsync to pannous.com:/var/www/pannous/uniscript/
cd web/uniscript && python3 -m http.server 8765   # local
```

Committed: `index.html`, `uniscript.js`, `uniscript.wasm`, `build.sh`. Not committed (`.gitignore`): `data/` (a copy of the
3.4 MB index) and `fonts/` (woff2 of UniscriptSans, UniscriptCJK, NewGardinerOmni2d4, all OFL; from `fonts/dist` or
`~/Library/Fonts`). The Monaco/Menlo mirror fonts are Apple fonts and are never copied.

## How the JS host works (`uniscript.js`)

- The module's `main` is its init: it calls `host.read("data/uniscript/entities.idx")` synchronously. Host calls are
  synchronous, so the page fetches the index before instantiating; any other path falls back to a synchronous XHR
  (charset `x-user-defined` keeps the bytes).
- Imports come from `WebAssembly.Module.imports()`: `host.read`, `host.fetch`, `host.fetch_within`, `host.warn` are real
  (`warning(message)` in wasp lists under the output of the conversion that raised it, cleared on the next one),
  `host.run` and anything unknown warn in the page and return 0 / -1.
- Bytes go into memory by the rule of `src/host.rs write_bytes_to_caller` (the exported `text_heap` bump pointer, fresh
  pages when it is 0 or full), then `new_text(ptr, len)`.
- Results: JS cannot read WASM GC struct fields, so warp now exports `get_text_ptr` / `get_text_len` next to `get_kind`
  (the `$String` of a Text, Symbol or Error, 0 otherwise). `tests/test_text_getters.rs` does the same in wasmtime.
- After each conversion `text_heap` is reset to its value after init: `out += …` allocates a fresh text per append, and
  the results are copied out immediately, so memory does not grow with typing.

## Status

- Verified headless (agent-browser, Chromium) locally and on the public URL: all examples, typing, the reverse box,
  errors (`<:nosuchthing>` in red), fonts render colors, mirror/turn/left/right, IDS composition (⿰木木 → 林) and
  hieroglyph groups.
- Firefox and Safari not tested (agent-browser drives only Chromium); both ship WASM GC.
- Nested tags (`<:above 木 <:beside 木 木>>`) are not supported by lib/uniscript.wasp: a tag ends at the first `>`.
- Greek blocks join their words (spaces separate operands): one word per `<:greek> … <:/greek>`; no final sigma (θεοσ).
- A repeated warning reports the operand's byte, not the character's: `<:greek> philosophia` warns twice "no greek form of h at byte 8".
- The CJK font is 10.8 MB as woff2; it loads only when CJK or IDS characters appear (`unicode-range`).
