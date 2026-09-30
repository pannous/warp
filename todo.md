# TODO

- DONE: Unbounded Int promotes to bignum on overflow, and `law::lean` exports Warp Int as Lean's unbounded `Int`; `square(3037000500)` is `9223372037000250000` and `law square(x) >= 0` is provable. Explicit `as i64` values still wrap and need a future `BitVec 64` proof model. See notes/laws.md.
- `i<n {…}` without spaces lexes `<n` as a tag (`while i<n {i++}` → garbage); with spaces it works (seen writing lib/uniscript.wasp).
- `global g = read("f")` → "undefined variable: read"; `global g = "abc"` used in a function → "cannot extract a numeric value"; `const` works.
- A variable first assigned a one-character literal (`s="x"`) is a codepoint variable; `s = s + "ab"` is then a type error.
- Uniscript: two suffix controls on one character (`<:mirror red A>`) are not expressible; nested tags are not supported.
- tests/test_wasm.rs `test_string_concat_wasm` passes now but is still `#[ignore]`.
