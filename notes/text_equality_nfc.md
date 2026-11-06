# Text equality by NFC (card text-equality-nfc, decisions nfc-tables, nfc-slot)

`"é" == "e◌́"` is yes: text `==` / `!=` compare NFC forms. Plan as the user approved it, with what the code says:

## Where
- Run-time equality: core/runtime/equality.warp node_equal (its text branch: text_order == 0) and textual_equal
  (same_bytes); typed `a == b` of texts: core/emitter/operators.warp (`runtime_call("text_order", …)` for every
  comparison, equalities included); textual Nodes in node_operation (node_text + text_order).
- Keep same_bytes and text_order pure: names, symbols, case spellings and map keys compare by bytes (a map lookup
  normalising each non-ASCII miss would be slow). Only `==`/`!=` of texts take a new runtime word `equal_texts`.

## Fast path
- Equal bytes: yes. Bytes differ: normalise only when a byte ≥ 0xCC is in either text. A text of code points below
  U+0300 only (lead bytes C2–CB) is its own NFC (nothing there has a combining class or is NFC_QC No/Maybe), so two
  such texts that differ differ. Sound and narrower than "any non-ASCII byte": German/French precomposed text never
  leaves the fast path.

## The slot
- A host word `canonical_text(t: Node): Node` (core/analysis/host_words.warp, declared `from runtime` in
  core/runtime/text.warp as wasi.warp declares real_path). Not a funcref global: the host answers it, which is what
  makes loading lazy.
- Native driver (core/native/driver.warp): CANONICAL_TEXT_WORD; on its first call instance_defined the module
  `warp/unicode` (modules/unicode.wasm, its imports linked as any module's) and call its export `canonical_text`,
  passing the wasmtime_val (a GC ref) through; later calls reuse the instance. The linker, engine and linked cell of
  the current run must be reachable from the callback (eval_code nests runs: keep them per run).
- Playground (web/playground/host.js): the same word through linkedInstance("warp/unicode", …) on first call.
- lib/unicode.warp exports `canonical_text(t: any): any := nfc(t)`; std_module_names (core/passes/rewrites.warp)
  gains unicode_data and unicode so `warp rebuild` writes modules/unicode.wasm.
- No unicode.wasm: instance_defined already ends the run naming the missing file (loud, never a byte compare).

## Order of landing (bootstrap)
1. Driver + playground answer canonical_text, unicode becomes a prebuilt module. Harmless alone.
2. After the installed binary has (1) (after-push.sh installs it): the runtime's equal_texts and the emitter's
   equalities use it. Landing (2) first would make a compiler built from it trap under an older driver the first
   time it compares two differing texts with a byte ≥ 0xCC (the compiler compares `→`, `≤`, `÷` …).
- (2) needed one more step first: the runtime calling a host word of nodes made its prebuild export the node builders
  and fail ("index out of range", core/emitter/host.warp host_call). That fix (1be0715b8e) has to be in the seed
  before (2) builds, since the seed's step compiles the runtime with its own emitter: landed, then a seed refresh.
- Measure the compiler's self-build time before/after (2): it compares operator spellings with bytes ≥ 0xCC.
- Un-hold the test/welcoming line (test_footguns «'é'=='é'», Grok C holds it) when (2) lands.
