# Inspiration: conversions and equality

Claims marked ✓ were run locally in `probes/footguns/inspiration/scratch/ce/` (python3 3.x, node, rustc, swift, julia, go, runghc, 2026-09-26).

### String + number
Solved elsewhere: **Rust**: `"5" + 3` → compile error E0369 `cannot add {integer} to &str` ✓ — `Add` is only implemented for matching types, no coercion trait exists. **Python**: `"5"+3` → `TypeError: can only concatenate str (not "int") to str` ✓, while `f"{5}{3}"` → `"53"` ✓ — conversion is explicit, interpolation is the sanctioned route. **Julia**: `"5"*3` → `MethodError` ✓ — multiple dispatch has no `(String, Int)` method.
Adopt in Warp: `+` dispatches on resolved semantic types in elaboration; `Text + Int` has no method and yields a diagnostic with the two fix-its (`"5" + str 3`, `int "5" + 3`); codepoint arithmetic exists only for the `char` type, so `"5"+3 → 56` disappears because a one-char string literal is `Text`, not `char`.

### Parsing numbers from text
Solved elsewhere: **Rust**: `"12a".parse::<i64>()` → `Err(ParseIntError { kind: InvalidDigit })` ✓ — returns `Result`, the whole string must match. **Swift**: `Int("12a")` → `nil` ✓ — failable initializer returns `Optional`. **Go**: `strconv.Atoi("12a")` → `0, invalid syntax` ✓ — error value alongside, lint-enforced checking. (Counter-example: JS `parseInt("12a")` → `12` ✓, prefix parsing.) Also **Haskell** `readMaybe "12a" :: Maybe Int` → `Nothing` ✓.
Adopt in Warp: `int "12a"` returns `Result Int ParseError` (DESIGN.md → Effects: `Error` as `Result`), requiring the full string to match; no transparent unwrapping, so using it as a number without handling the error is a type error. A prefix-parsing variant, if ever wanted, must be named as such (`int_prefix`).

### Type annotations not enforced loudly
Solved elsewhere: **Swift**: `let x = 5; x = 6` → `error: cannot assign to value: 'x' is a 'let' constant` ✓ — constness is part of the binding, checked before codegen. **Julia**: `x::Int = 5; x = "five"` → `MethodError` (convert String→Int) ✓ — typed globals insert a checked `convert` on every assignment. **Rust/TypeScript**: `let x: i64 = 5; x = "five"` → `mismatched types` with span and `help:` fix-it — diagnostics carry spans and suggestions.
Adopt in Warp: bindings in semantic IR carry `{type, mutability}`; elaboration checks every assignment against them and emits a spanned diagnostic (with fix-it) instead of reaching the emitter; `const` and `::=` produce immutable bindings, so reassignment is rejected rather than ignored.

### Lists and arithmetic
Solved elsewhere: **Julia**: `[1,2,3] .* 2` → `[2,4,6]` ✓, `[1,2,3] + 1` → `MethodError` ✓, `vcat([1,2],[3])` → `[1,2,3]` ✓ — broadcasting is a separate, explicit dot syntax; plain operators keep their algebraic meaning. **APL/J/NumPy** broadcast implicitly (`[1,2,3]*2` → `[2,4,6]`) but under fixed shape rules; **Python** `[1,2]+[3]` → `[1,2,3]` ✓ (concatenation) yet `[1,2,3]*2` → repetition ✓, i.e. `*` has an unrelated meaning.
Adopt in Warp: `+` on lists is concatenation (the list monoid, lawful: associative with `[]` as identity), `list * number` is a type error; element-wise arithmetic uses an explicit lifting form (Julia-style `.+`/`.*` or a `map`), which is exactly the "type-directed, law-governed lifting rule" DESIGN.md → Dangerous implicitness asks for. Never sum a list implicitly.

### 🐞 String comparison
Solved elsewhere: **Rust**: `String::from("abc") == "abc"` → `true` ✓ — `==` is `PartialEq`, always by value; identity needs explicit `std::ptr::eq`. **Swift**: `==` is value equality for `String`, identity `===` exists only for class instances. **JavaScript**: `Object.is(NaN,NaN)` → `true` ✓ separates SameValue from `===`, showing the cost of having several equalities.
Adopt in Warp: `==` and `is` are structural value equality for all data (Text compared by content), elaborated per type rather than via "extract numeric value"; no user-visible identity operator. Mixed-type comparisons (`0==""`, `null==false`) are type errors, not panics and not coercions.

### Unicode normalization
Solved elsewhere: **Swift**: `"\u{e9}" == "e\u{301}"` → `true` ✓ — `String ==` uses Unicode canonical equivalence on grapheme clusters. **Python/JS** make it explicit: `unicodedata.normalize("NFC", s)` ✓ / `s.normalize("NFC")` ✓ (plain `==` → `false` ✓). Rust `==` → `false` ✓ (bytes; `unicode-normalization` crate needed).
Adopt in Warp: normalize text literals and text read by the parser to NFC once (so storage is canonical and `==` stays a cheap byte compare); text entering at runtime from IO is normalized at the boundary, with an explicit `bytes`/raw type for when exact code points must be preserved.

### Duplicate keys
Solved elsewhere: **Go**: `map[string]int{"a":1,"a":2}` → compile error `duplicate key "a" in map literal` ✓. **TOML 1.0**: `a=1\na=2` → `TOMLDecodeError: Cannot overwrite a value` ✓ — the spec forbids redefinition. **Python json** silently keeps the last (`{'a': 2}` ✓) but `object_pairs_hook` can reject it ✓; **Nix** `a // { a = 2; }` / JS `{...o, a:2}` show the explicit-override form.
Adopt in Warp: a literal with a repeated key is a parse diagnostic in data (TOML/Go rule); overriding is spelled explicitly in code (an update/merge form like `o with {a:2}` or Nix `//`), so "last wins" is never accidental.

### Loose equality
Solved elsewhere: **Python/Ruby**: `1 == "1"` → `False` ✓ — no cross-type coercion, `==` of unrelated types is simply false. **Haskell/Rust**: `1 == "1"` → compile-time type error — `Eq`/`PartialEq` only for the same type. **Scheme** separates numeric `=` (`(= 1 1.0)` → #t) from structural `equal?`.
Adopt in Warp: keep `1=="1"` → false today, but aim for the Haskell/Rust answer: comparing unrelated types is a diagnostic (almost always a bug), while numeric kinds compare by exact value across representations (`3 == 3.0` true, consistent with wiki/equality.md "compatible" and exact rationals).
