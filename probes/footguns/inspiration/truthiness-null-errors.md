# Inspiration: truthiness, null, errors

Group `truthiness-null-errors`. Claims marked ✓ were run locally in `probes/footguns/inspiration/scratch/`
(python3 3.x, node, ruby, lua, rustc, swift, go on 2026-09-26).

Cross-cutting tension to resolve first: [wiki/truthiness.md](../../../wiki/truthiness.md) says "close to Python"
(all empty values falsy, even an unbound `x`), while [DESIGN.md → Dangerous implicitness](../../../DESIGN.md#dangerous-implicitness)
forbids "arbitrary truthiness across numbers, text, collections, and user objects". The languages that solved this
family of footguns (Swift, Go, Rust, Kotlin) all chose the DESIGN.md side; the wiki's "unbound x is falsy" also
directly contradicts the Solved entry *Undefined variables*.

### Empty values
Solved elsewhere: **Swift** ✓: `let xs:[Int]=[]; if xs {}` → compile error `cannot convert value of type '[Int]' to expected condition type 'Bool'`; you write `if xs.isEmpty` — conditions must be `Bool` (or an optional binding `if let`). **Go** ✓: `if x {}` with `x := 1` → `non-boolean condition in if statement`. **Ruby** ✓: `!![]`, `!!""`, `!!0`, `!!"0"` → all `true` — only `nil` and `false` are falsy, one rule with no per-type exceptions (the uniform alternative if Warp keeps truthiness at all).
Adopt in Warp: conditions take `Bool` or `T?` only (DESIGN.md); `if xs` on a collection/text is a structured diagnostic with fix-it `if not xs.empty` / `if xs.count > 0`. If backwards compatibility with the wiki is wanted, make it a *normalization hint*: accept `if xs` in the friendly surface, but elaborate it explicitly to `not empty xs` in the semantic IR, so one rule (Python's) applies uniformly and is visible in the resolved view. Either way, drop "unbound `x` is falsy" from the wiki.

### `and`/`or` as ternary
Solved elsewhere: **Python 2.5+** ✓: `0 if 1 else 2` → `0` (vs `1 and 0 or 2` → `2` ✓) — PEP 308 added a real conditional expression precisely because the idiom breaks on falsy `x`. **Rust/Swift** ✓: `if true {0} else {2}` → `0` — `if` is an expression and `&&`/`||` only accept and return `bool`, so the idiom cannot typecheck. **Lua** ✓ still has the bug: `true and false or "y"` → `y` (no fix; shows the cost).
Adopt in Warp: make `and`/`or` Bool-typed (return `Bool`, not an operand), which dissolves the footgun under the strict-condition rule above; until then, lint the shape `a and b or c` with fix-it `if a then b else c`. Value-selecting defaulting belongs to a dedicated `??` / `or else` on `T?` (see Null), not to `or`.

### Null
Solved elsewhere: **Swift** ✓: `let p:P? = nil; p?.name ?? "anon"` → `anon`; `p + 1` on `Int?` → compile error `value of optional type 'Int?' must be unwrapped`. **Kotlin**: `val s:String? = null; s.length` → compile error; `if (s != null) s.length` compiles via flow-sensitive smart casts. **Rust** ✓: `None::<i32>.map_or(0,|x|x+1)` → `0` — no null at all, `Option<T>` is an ordinary enum with exhaustive `match`.
Adopt in Warp: exactly [wiki/null.md](../../../wiki/null.md) + DESIGN.md's `Option(T)`: `T?` in the semantic type, Kotlin-style flow narrowing after `if x` / `if x != ø`, `?.` and `??` as the only implicit paths, and `x=ø; x+1` becoming a spanned diagnostic "`x` is `Int?`, unwrap with `x ?? 0` or check `if x`" instead of a panic. No transparent unwrapping (DESIGN.md).

### Swallowed errors
Solved elsewhere: **Rust** ✓: `#[must_use] fn g()->Result<..>; g();` → `unused Result that must be used` (deny-able to an error); `?` propagates in one character. **Swift** ✓: calling a `throws` function without marking it → compile error `call can throw but is not marked with 'try'`; `(try? f()) ?? -1` → `-1` is the *explicit* swallow, and `Int("12a")` → `nil` ✓ (not `0`). **Zig**: error unions must be handled (`try`, `catch`, or `catch unreachable`); discarding one is a compile error.
Adopt in Warp: `Result<T,E>` as data with Rust's "must use" as a hard error, `?`/`try` for propagation, and any discard spelled explicitly (`try? f()`, `f() or else default`) so it is greppable. Concretely: `int("12a")` must return `Int?`/`Result`, never `0`; `x[3]` out of range returns `T?` or a spanned error; compiler panics (`Cannot extract numeric value …`) become `Diagnostic`s with span + resolved facts + fix-it.

### Undefined variables
Solved elsewhere: **JavaScript strict mode / ES modules** ✓: `"use strict"; undeclared = 1` → `ReferenceError` (sloppy mode silently created a global). **Python** ✓: `a+1` → `NameError: name 'a' is not defined` (at runtime only). **Rust/Go/Swift**: unresolved names are compile errors, with "did you mean `abc`?" suggestions (rustc uses edit distance over names in scope).
Adopt in Warp: keep the compile-time error, but make it a structured diagnostic (span, names in scope) with a Rust-style *suggestion* — the suggestion is a fix-it the user/agent accepts, never an automatic rebinding (DESIGN.md forbids "guessing unresolved identifiers from nearby names"). Remove the contradictory "`x==false` if unbound" rule from wiki/truthiness.md.

### "0" is falsy
Solved elsewhere: **Ruby** ✓: `!!"0"` and `!!0` → `true` — no string or number is falsy. **Python/JS** ✓: `bool("0")` / `!!"0"` → `True`/`true` — only the *empty* string is falsy, the content is never inspected. **Swift/Go/Rust** ✓: `if "0"` is a type error — strings are never conditions.
Adopt in Warp: already solved (`if "0"` → `1`). Under the strict-condition rule it would become a type error with fix-it `if s.empty` / `if int(s) != 0`, which is stronger; note that the same rule also turns Warp's currently-accepted `if 0 {…}` into an error (fix-it `if n != 0`), so pin that decision together with *Empty values*.
