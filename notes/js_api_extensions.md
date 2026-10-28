# WebAssembly JS API extensions and warp (researched 2026-10-28)

What the JS-side WebAssembly proposals are, which engines ship them, and what each would give warp: the browser
playground (web/playground), the DOM, async host calls, texts.
Sources at the end, fetched 2026-10-28. "?" marks a value that is not verified.

## Where warp stands (grounding)
- Texts are `$Text = (array (mut i8))`, UTF-8 (AGENTS.md "Serialization"). Linear memory is used only at the
  host/FFI edge: `text_address` copies a text there, `print_line` streams through one page (notes/self_hosting.md).
- emitter.warp imports WASI `fd_write` and the "host" module's words (core/emitter/emitter.warp `imported_words`, about
  line 530). There are no `wasm:` builtin imports and no externref values.
- Errors: inside a `try`, a runtime error throws the tag `warp_error(i32)`, which a `try_table` catches (final
  exnref EH). Outside a try it traps (core/emitter/emitter.warp about line 88).
- The playground still runs the Rust-era compiler: web/playground/warp.wasm is built by build.sh from src/web.
  emitter.warp modules do not run in the browser yet (notes/self_hosting.md; card `playground-compiles`).
- The playground already uses JSPI: `WebAssembly.Suspending`/`promising` (host.js:405, host-foreign.js:272,
  host-compiler.js:42). Its comment there says "Safari has none", which Safari 27 has made out of date.
- JS cannot read GC structs or arrays, so reader.js reads results through the module's `reflect_*` exports. It reads a
  text one byte per call (`readTextBytes` → `reflect_text_byte`), so a text of n bytes costs n boundary crossings.

## Proposal status

Phases are from WebAssembly/proposals. "Finished" means merged into the spec.

| Proposal | CG phase | Chrome / V8 | Firefox | Safari | Node | Deno | wasmtime / native sibling |
|---|---|---|---|---|---|---|---|
| JS String Builtins (`wasm:js-string`, `importedStringConstants`) | finished (WG 2025-07-23) | 130 (some sources say 131) | 134 | 26.2 (streaming compile broken until 26.6, WebKit bug 308136) | behind `--experimental-wasm-imported-strings` | 2.1 | none built in. Builtin imports are ordinary imports to a non-JS host, so a host can define them as externref functions (no host does this yet) |
| JS Text Encoding Builtins (`wasm:text-encoding`, UTF-8 ↔ JS string on `(array (mut i8))`) | 1 | – | – | – | – | – | – |
| JS Primitive Builtins | 2 | ? | ? | ? | ? | ? | – (contents not read) |
| Reference-Typed Strings (stringref) | 1 | (V8 had a flag earlier, ?) | – | – | – | – | – |
| JSPI (`Suspending`, `promising`) | 5 (standardized, not merged) | 137 | 153 (intent to ship) | 27.0 | 26.0 | 2.3.2 | async host functions on fibers (`func_wrap_async`); stack switching below |
| Stack Switching (`cont.new`, `resume`, `suspend`, `switch`) | 3 | behind a flag | – | – | – | – | under way: tracking issue #10248, GC support PR #14140 (2026-08); on by default? |
| ESM Integration (`import … from "x.wasm"`, `import source`) | 3 | – (source-phase imports in V8 M131) | – | – | flag dropped in 24.5 / 22.19, still experimental | 2.1 | – (components instead) |
| Type Reflection (`WebAssembly.Function`, `.type()`) | 1 in the active table (it used to be 3) | behind a flag | Nightly only | 18.2: only `Module.imports/exports` types | behind a flag | behind a flag | wasmtime's Rust API has full type reflection |
| GC JS API (opaque structs, null prototype) | finished, part of GC | 119 | 120 | 18.2 | 22 | 1.38 | 47 |
| Custom Descriptors and JS Interop (JS prototypes and methods on GC structs) | 3 | origin trial / `--experimental-wasm-custom-descriptors`, JS part `--experimental-wasm-js-interop` | – | – | – | – | – |
| Memory64 (JS: `address: "i64"`, BigInt indices) | finished | 133 | 134 | Technology Preview 251 only | 24 | 2.2 | 30 |
| `Memory.toResizableBuffer` / `toFixedLengthBuffer` | JS API spec | 144 | 145 | 26.2 | 26 | ? | n/a |
| Exception handling, exnref (`WebAssembly.Tag`, `WebAssembly.Exception`, `getArg`, `is`) | finished | 137 | 131 | 18.4 | 24.15 | 2.3.2 | 47 |
| Wide Arithmetic (`i64.add128`, `i64.mul_wide_s/u`) | 4 | behind a flag | Nightly | behind a flag | behind a flag | behind a flag | `--wasm=wide-arithmetic` |
| Compact Import Section | 4 | – | – | – | – | – | – |
| Custom Page Sizes | 3 | – | – | – | – | – | `--wasm=custom-page-sizes` |
| Component Model / WASI p3 (async) | 1 (CG) | via jco (transpiled to JS) | jco | jco | jco | jco | primary: strings are copied by the canonical ABI; p3 async in recent releases (?) |

Also new and relevant only at the margin: Shared-Everything Threads (1), Frozen Values (1), More Array Constructors (1),
Type Imports (1), Compilation Hints (2), Multibyte Array Access (2), FP16 (2).

## What each would give warp

| Extension | Concrete gain | How warp would connect | Cost |
|---|---|---|---|
| JS String Builtins | JS strings as `externref` values with `length`, `charCodeAt`, `concat`, `equals`, `compare`, `substring`, with no glue calls: DOM values (`innerText`, input values, URLs) could flow through warp untouched. It is **not** zero-copy for warp's own texts: its array functions are UTF-16 (`fromCharCodeArray` takes `(array (mut i16))`), and warp's texts are UTF-8 | The emitter imports the functions from `"wasm:js-string"` (ordinary import entries, so `imported_words` needs no new mechanism), and the playground compiles with `{builtins: ["js-string"]}`. A JS text would be a new value kind, or a Text node whose data is an externref. wasmtime would need a polyfill under that import name | new value kind: a core change, through the Interviewer |
| JS Text Encoding Builtins | the exact fit: `decodeStringFromUTF8Array($Text, 0, len)` gives a JS string straight from warp's own array, and `encodeStringIntoUTF8Array` goes the other way | the same as above, under module `"wasm:text-encoding"` | Phase 1, so it ships nowhere yet; it is the one to follow |
| JSPI | warp code stays synchronous while fetch, timers and foreign promises resolve | already used by the playground. With JSPI in all three browsers since Safari 27, the non-JSPI fallback paths can be retired or reduced to a warning | glue only |
| Stack Switching | generators, async and green threads in warp itself, native and in the browser, with no host involvement | emitter instructions (cont.new / resume / suspend), plus a native harness flag | waits for engines to ship it |
| ESM Integration | `import {main} from "./prog.wasm"` in Node/Deno; a published warp program as an npm-style module | the module's imports ("host", WASI) would have to resolve as ES modules: an ESM-shaped host.js | glue only, but little demand |
| Type Reflection | JS could make typed `WebAssembly.Function`s for callbacks and inspect export signatures. The playground does not need it: it knows the signatures | – | skip |
| GC JS API today | none beyond passing references: structs are opaque. That is why the `reflect_*` exports and reader.js exist | – | – |
| Custom Descriptors and JS Interop | warp Nodes and classes seen from JS as objects with prototypes and methods (`node.kind`, `person.name`): reader.js and the reflection exports could disappear, and JS could call warp class methods directly | descriptors in the emitter's type section, plus `struct.new_desc` for classes | Phase 3, flags only; the big long-term one |
| Memory64 | nothing: warp keeps one page of memory at the edge | – | skip |
| Resizable buffers | views that survive `memory.grow`. host.js re-reads `memory.buffer` on every call already | – | skip |
| Exception handling JS API | an exported `warp_error` tag lets JS catch a warp error as `WebAssembly.Exception` and read its payload with `getArg(tag, 0)`, and lets host JS throw a warp error that a warp `try` catches (`new WebAssembly.Exception(tag, [..])`). Native: wasmtime 47 has exceptions too (embedder API ?) | an export entry of kind tag (4) for `warp_error`. Throwing instead of trapping outside a `try` would be a separate decision | small core change |
| Wide Arithmetic | faster unbounded-int arithmetic and overflow checks (`mul_wide`, `add128`) | the emitter's int runtime words | waits for browsers to ship it unflagged |
| Component Model | the native, typed alternative to JS glue (WIT), which warp already has a path for | – | – |

## Recommendations (cheapest high value first)
1. **Make the browser host of emitter.warp modules text-efficient from the start** (part of `playground-compiles`).
   Cross a text in one call: the module copies it into linear memory (`text_address`, which exists) and JS runs
   `TextDecoder` over `memory.buffer`. Do not use a `reflect_text_byte` call per byte. Plan the text crossing as one
   function of the host, so it can later become `wasm:text-encoding` without touching callers.
2. **Retire the "no JSPI" assumption.** JSPI ships in Chrome 137, Firefox 153 and Safari 27, so the comment at
   host.js:405 is out of date and the non-JSPI fallback paths can shrink to a loud "browser too old" message
   (card `jspi-everywhere`, Later).
3. **Export the `warp_error` tag.** It makes a warp error visible to JS as `WebAssembly.Exception`, and lets the
   playground's JS host throw errors that warp's `try` catches. One export entry in core (card
   `export-error-tag`, Later; the core change goes through the Interviewer).
4. **JS strings as a value kind (externref + `wasm:js-string`)** only together with DOM work, when warp code handles
   many DOM strings it never inspects. Design it together with custom descriptors, not before.
5. **Follow, don't build:** JS Text Encoding Builtins (the exact fit for UTF-8 `$Text`), Custom Descriptors and JS
   Interop (would replace reader.js), Stack Switching in wasmtime (native generators and async), Wide Arithmetic.
   Skip Memory64, resizable buffers, type reflection and ESM integration for now.

## Sources (fetched 2026-10-28)
- Proposal phases: https://github.com/WebAssembly/proposals ;
  finished: https://github.com/WebAssembly/proposals/blob/main/finished-proposals.md
- Engine support matrix: https://github.com/WebAssembly/website/blob/main/features.json (the data behind
  https://webassembly.org/features/)
- JS String Builtins: https://github.com/WebAssembly/js-string-builtins ; MDN `builtins` / `importedStringConstants`:
  https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/compile_static ; Safari 26.2
  streaming bug and the 26.6 fix: https://webkit.org/blog/18178/webkit-features-for-safari-26-6/
- JS Text Encoding Builtins:
  https://github.com/WebAssembly/js-text-encoding-builtins/blob/main/proposals/js-text-encoding-builtins/Overview.md
- JSPI in Safari 27: https://webkit.org/blog/18325/webkit-features-for-safari-27-0/ ; Firefox intent to ship (Fx153):
  http://www.mail-archive.com/dev-platform@mozilla.org/msg01810.html
- ESM integration in Node: https://nodejs.org/api/esm.html , https://github.com/nodejs/node/pull/57038
- Custom descriptors: https://github.com/WebAssembly/custom-descriptors/blob/main/proposals/custom-descriptors/Overview.md ,
  V8 https://issues.chromium.org/issues/403372470 , origin trial
  http://www.mail-archive.com/blink-dev@chromium.org/msg16076.html (2025-08)
- wasmtime stack switching: https://github.com/bytecodealliance/wasmtime/issues/10248 ,
  https://github.com/bytecodealliance/wasmtime/pull/14140 (2026-08-14)
- Resizable buffers: https://caniuse.com/mdn-webassembly_api_memory_toresizablebuffer ,
  https://github.com/mdn/browser-compat-data/pull/30541
