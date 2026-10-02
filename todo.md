# TODO

- DONE: Unbounded Int promotes to bignum on overflow, and `law::lean` exports Warp Int as Lean's unbounded `Int`; `square(3037000500)` is `9223372037000250000` and `law square(x) >= 0` is provable. Explicit `as i64` values still wrap and need a future `BitVec 64` proof model. See notes/laws.md.
- `i<n {…}` without spaces lexes `<n` as a tag (`while i<n {i++}` → garbage); with spaces it works (seen writing lib/uniscript.wasp).
- `global g = read("f")` → "undefined variable: read"; `global g = "abc"` used in a function → "cannot extract a numeric value"; `const` works.
- A variable first assigned a one-character literal (`s="x"`) is a codepoint variable; `s = s + "ab"` is then a type error.
- DONE: Uniscript: two suffix controls on one character (`<:mirror red A>`) are not expressible; nested tags are not supported. (effect words stack now; nested tags still open)
- tests/test_wasm.rs `test_string_concat_wasm` passes now but is still `#[ignore]`.
- `use x from 1.10` / `use x >= 1.10` read 1.10 as the float 1.1 (only `version 1.10` and literals with two dots keep their text): write `from 1.10.0` or `from version 1.10`.
- DONE: uniscript has no git tag v0.2.0 yet (only the stale v0.1.0): `use uniscript version 0.2.0` works through the default branch's declared version only. (released as v1.0.0 instead, tagged)

- DONE: tests/test_method_words.rs test_library_words_refuse_what_they_cannot_do expects upper("é") to fail with "non ascii text", but the merged text-runtime branch (src/wasm_emitter/text_unicode.rs) now maps Latin/Greek/Cyrillic case, so upper("é") = "É". Decide: drop that expectation or keep the ASCII-only refusal. (2026-10-01 branch consolidation)
