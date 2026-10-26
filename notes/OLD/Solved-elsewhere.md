# Solved elsewhere

Comparisons showing how other languages avoid the footguns catalogued in [Footguns.md](Footguns.md).
Each recommendation remains paired with the original Footguns heading.

## Solved

### Loose equality

Solved elsewhere: **Python/Ruby**: `1 == "1"` → `False` ✓ — no cross-type coercion, `==` of unrelated types is simply false. **Haskell/Rust**: `1 == "1"` → compile-time type error — `Eq`/`PartialEq` only for the same type. **Scheme** separates numeric `=` (`(= 1 1.0)` → #t) from structural `equal?`.
Adopt in Warp: keep `1=="1"` → false today, but aim for the Haskell/Rust answer: comparing unrelated types is a diagnostic (almost always a bug), while numeric kinds compare by exact value across representations (`3 == 3.0` true, consistent with wiki/equality.md "compatible" and exact rationals).

### Automatic semicolon insertion

Solved elsewhere: **Go**: `x := 1⏎-1` → *compile error: -1 (untyped int constant) is not used* — the lexer inserts `;` after a line ending in an identifier/literal/`)`/`}`/`return`/`++`, a purely lexical, documented rule, and a dangling expression statement is an error.
**Python**: newline ends a statement unless inside brackets or after `\`, so `return⏎{…}` returns `None` *and* the dict line is a separate (dead) statement.
**JavaScript** (the footgun, verified): `let x=1⏎-1` → `x == 0`.
Adopt in Warp: already solved (newline ends the statement); add Go's second half — a bare value statement whose result is discarded (`-1` on its own line mid-block) is a diagnostic, and continuation only inside open brackets or after a trailing binary operator.

### Parameter shadowing

Solved elsewhere: **Swift**: `let x=1; func g(x:Int)->Int{x*2}; g(x:5)+x` → `11` (verified) — lexical scoping, parameter wins, outer untouched.
**Haskell (`-Wname-shadowing`), Go (`go vet -vettool shadow`), Kotlin ("Name shadowed" warning)** — shadowing is legal but the compiler reports it, catching the "meant the outer one" bug.
**Rust**: `let x = x + 1` shadowing is idiomatic, but an unused outer binding triggers `unused_variables`.
Adopt in Warp: keep lexical parameter shadowing (already Solved) and add a lint-level diagnostic with span when a parameter or local shadows an outer binding that is then used in the same function, with a fix-it rename; never an error, since data formats reuse names.

### "0" is falsy

Solved elsewhere: **Ruby** ✓: `!!"0"` and `!!0` → `true` — no string or number is falsy. **Python/JS** ✓: `bool("0")` / `!!"0"` → `True`/`true` — only the *empty* string is falsy, the content is never inspected. **Swift/Go/Rust** ✓: `if "0"` is a type error — strings are never conditions.
Adopt in Warp: already solved (`if "0"` → `1`). Under the strict-condition rule it would become a type error with fix-it `if s.empty` / `if int(s) != 0`, which is stronger; note that the same rule also turns Warp's currently-accepted `if 0 {…}` into an error (fix-it `if n != 0`), so pin that decision together with *Empty values*.

### Undefined variables

Solved elsewhere: **JavaScript strict mode / ES modules** ✓: `"use strict"; undeclared = 1` → `ReferenceError` (sloppy mode silently created a global). **Python** ✓: `a+1` → `NameError: name 'a' is not defined` (at runtime only). **Rust/Go/Swift**: unresolved names are compile errors, with "did you mean `abc`?" suggestions (rustc uses edit distance over names in scope).
Adopt in Warp: keep the compile-time error, but make it a structured diagnostic (span, names in scope) with a Rust-style *suggestion* — the suggestion is a fix-it the user/agent accepts, never an automatic rebinding (DESIGN.md forbids "guessing unresolved identifiers from nearby names"). Remove the contradictory "`x==false` if unbound" rule from wiki/truthiness.md.

### Integer overflow

Solved elsewhere: **Python**: `2**63, 2**64, abs(-2**63)` → `9223372036854775808 18446744073709551616 9223372036854775808`: `int` is arbitrary precision (small ints fast path, bignum on demand). **Haskell** `2^63 :: Integer` → `9223372036854775808` (and `Integer` is the default type).
**Swift**: `Int.max + 1` → traps at runtime (verified: process aborts), wrapping only via the explicit `&+` → `-9223372036854775808`, and `addingReportingOverflow`.
**Rust**: `i64::MAX.checked_add(1)` → `None`, `overflowing_add` → `(-9223372036854775808, true)`: the operation names the policy (`checked_/wrapping_/saturating_/overflowing_`).
Adopt in Warp: Python/Haskell semantics (Int is ℤ, i64 is an inferred representation that promotes to bignum on overflow, e.g. via an overflow check on the fast path as in [wiki/int60.md](wiki/int60.md)), plus Swift-style explicit wrapping operators or an `@wrap` / `@i64` representation annotation for code that wants modular arithmetic.

### Big literals

Solved elsewhere: **Rust**: `let _x: i64 = 100000000000000000000;` → `error: literal out of range for 'i64'` (deny-by-default lint).
**Go**: `var x int64 = 100000000000000000000` → `cannot use 100000000000000000000 (untyped int constant) as int64 value in variable declaration (overflows)`, with the exact column.
**JS**: `100000000000000000000n` → `100000000000000000000n`; **Python** `100000000000000000000` → the exact int.
Adopt in Warp: until bignums exist, any integer literal outside i64 is a compile error with the span (Go/Rust); once Int is unbounded, the literal is simply a bignum (Python). Never a NUL string.

### Date guessing in data

Solved elsewhere: **TOML 1.0**: `d = 2001-12-14` → `date(2001,12,14)`, while `gene = "SEPT2"` → `'SEPT2'`. Dates are a separate literal grammar (RFC 3339 only) and are never guessed from strings.
**pandas**: `read_csv(..., dtype=str)` → `{'gene':'SEPT2','zip':'01234'}`. Date parsing is off unless `parse_dates=` is passed. Without `dtype`, the zip still turns into `1234`.
Counter-example worth citing: **JS**: `new Date("SEPT 2")` → *Sun Sep 02 2001* (V8 fills in the year 2001 on its own); only ISO `2001-12-14` is specified.
Adopt in Warp: keep today's rule (`SEPT2` stays a symbol). If date literals are added, accept only an unambiguous RFC 3339 / ISO 8601 form, as TOML does, and parse anything else as a date only when a schema expects one.

### Memory-safety classics: use-after-free, double free, dangling pointers

Solved elsewhere: **Rust**: `let v=vec![1]; drop(v); v` → *compile error E0382 borrow of moved value* (verified) — ownership + borrow checker, no GC.
**Java / Go / JS / WASM GC**: tracing GC, no `free`, no pointer arithmetic, bounds-checked arrays. **Swift**: ARC + exclusivity checks, `Unsafe*Pointer` quarantined by name.
Adopt in Warp: already safe by construction via WASM GC; keep any linear-memory or FFI access behind the `Unsafe` effect (DESIGN.md effect set) so it is visible in signatures, and make ownership-pass optimisations (stack/unique allocation) never able to produce a dangling reference: fall back to GC when escape analysis is unsure.

### Data races

Solved elsewhere: **Rust**: `let mut n=0; thread::spawn(|| n+=1); n+=1;` → *compile error E0373 / E0503* (verified) — `Send`/`Sync` traits plus borrow rules: shared XOR mutable across threads.
**Swift 6 strict concurrency**: actors and `Sendable` checking make cross-actor mutable sharing a compile error. **Erlang / Elixir, Pony**: share-nothing processes with message passing (Pony's reference capabilities prove race freedom statically).
Adopt in Warp: stay share-nothing: parallelism only over values proved pure (DESIGN.md Cautions), threads/components communicate by copying or moving values (WASM components already share no memory), and a future shared-memory mode requires an explicit `Sendable`-like capability checked by the ownership pass.

## NOT YET

### Numbers

#### Decimal fractions

Solved elsewhere: **Haskell**: `1%10 + 2%10 == (3%10 :: Rational)` → `True`; also `0.1 + 0.2 == (0.3 :: Rational)` → `True`: decimal literals are overloaded (`fromRational`), so at type `Rational` they are exact.
**Go**: `0.1+0.2 == 0.3` → `true`: untyped constants are evaluated at arbitrary precision at compile time, and only rounded when they get a concrete type.
**Julia**: `1//10 + 2//10 == 3//10` → `true` (`Rational{Int}`; `//` builds exact rationals). **Python** `Decimal("0.1")+Decimal("0.2")==Decimal("0.3")` → `True`, but only with an opt-in type.
Adopt in Warp: the Haskell/Go model: a literal like `0.1` means the exact value 1/10 and representation is chosen late (as Go does for constants, but for all values); `@f64` is the only place rounding enters. `.1` should parse like `0.1`.

#### 🐞 Sum of quotients truncated

Solved elsewhere: **Python**: `Fraction(1,4)+Fraction(1,4)` → `1/2`; **Julia**: `1//4+1//4` → `1//2`; **Haskell** `1%4 + 1%4` → `1 % 2`. The result type of `+` is computed from the operand types (a numeric tower/promotion rule: `Rational + Rational → Rational`), never from a default.
Adopt in Warp: result types of arithmetic come from a promotion table over the operand types (Int ⊂ Rational ⊂ Real), checked in elaboration; a function's return type is never defaulted to Int ([DESIGN.md](DESIGN.md) "unknown names and unsolved overloads are errors, never `Symbol` or `Int` defaults").

#### Scientific notation and digit separators

Solved elsewhere: **Python/Rust/Julia/JS/Go**: `1_000_000` → `1000000`, `1e3` → `1000.0` (Python, Julia) / `1000` (JS, Go, Rust prints `1000`). Separators are lexical: `_` between digits is ignored by the tokenizer, and a digit sequence followed by `e[+-]?digits` is one token.
**Python**: `int("1_000")` → `1000`: the same grammar is used for parsing text at runtime.
**Go**: `1e3` is an *untyped* constant usable as an int (`const n int = 1e3; fmt.Println(n)` → `1000`), i.e. the exponent does not force float.
Adopt in Warp: lex both in the number token (the tokenizer, not the list parser); in exact mode `1e3` is the integer 1000 and `1.5e-3` the rational 3/2000 (Go's untyped constants), so exponent notation never implies a float. `1e3` staying the list `1 e3` should become at least a warning.

#### NaN and infinity

Solved elsewhere: **Python**: `1/0` → `ZeroDivisionError: division by zero`, `math.sqrt(-1)` → `ValueError`, `cmath.sqrt(-1)` → `1j` (the complex answer is a separate, explicit module).
**JS BigInt / Python Fraction**: `1n/0n` → `RangeError: Division by zero`: exact types have no NaN/Inf, so division by zero is an error.
**Julia**: `isequal(NaN,NaN)` → `true` vs `NaN==NaN` → `false`; **JS** `Object.is(NaN,NaN)` → `true`, `[NaN].includes(NaN)` → `true` but `indexOf` → `-1`: a second, reflexive "same value" relation used for collections and hashing.
Adopt in Warp: exact numbers make `1/0` an `Error` value (Python/BigInt) and `sqrt(-1)` a `Complex` only when the result type allows it (as `cmath`); under `@f64` keep IEEE `==` but use a reflexive total-order equality (Julia `isequal`, IEEE 754 `totalOrder`) for `is`, dict keys, sorting and `law` checks.

#### Rounding mode

Solved elsewhere: **Julia**: `round(2.5)` → `2.0`, `round(2.5, RoundNearestTiesAway)` → `3.0`, `round(2.5, RoundUp)` → `3.0`: the mode is a named argument with a documented default.
**Rust**: `(2.5f64).round()` → `3`, `(2.5f64).round_ties_even()` → `2`: two differently named functions, no hidden default.
**Python Decimal**: `Decimal("2.5").quantize(Decimal("1"), rounding=ROUND_HALF_UP)` → `3`; `round(2.5)` → `2`. **Swift** `(2.5).rounded()` → `3.0`, `.rounded(.toNearestOrEven)` → `2.0`.
Adopt in Warp: `round(x)` plus a named mode argument (`round x half:even`, `half:up`, `half:away`, `down`, `up`), with the default spelled in the signature; for exact rationals the rounding is exact (no double-rounding of `2.675`-style decimals).

#### Negative modulo

Solved elsewhere: **Haskell**: `(-5) \`mod\` 3` → `1`, `(-5) \`rem\` 3` → `-2`; **Julia** `mod(-5,3)` → `1`, `rem(-5,3)` → `-2`: two named operators, one per convention.
**Rust**: `(-5i64).rem_euclid(3)` → `1` while `-5 % 3` → `-2`. **Python**: `-5 % 3` → `1`, `divmod(-5,3)` → `(-2, 1)` and `math.fmod(-5,3)` → `-2.0`, with the law `a == (a//b)*b + a%b` holding for floored division.
Adopt in Warp: Haskell/Julia naming, `%`/`mod` floored (sign of divisor) and `rem` truncating, and state the division law pairwise (`a == b*floor_div(a,b) + a%b`) as a `law` in the standard library so both pairs are checked.

#### Booleans are integers

Solved elsewhere: **Haskell**: `True + True` → `No instance for 'Num Bool'`; **Rust**: `true + true` → `error[E0369]: cannot add 'bool' to 'bool'`; conversion is explicit (`fromEnum True`, `true as i64`).
**JS BigInt**: `1n + true` → `TypeError` (the newer numeric type refused the old coercion). Contrast **Python**: `True+True` → `2`, `isinstance(True,int)` → `True`; **Julia** `true+true` → `2` (Bool <: Integer).
Adopt in Warp: `Bool` is its own kind in semantic IR (it may still be encoded as i32 0/1 in WASM); arithmetic on it is a type error, with explicit `int(b)` / counting via `count(xs, pred)`.

### Implicit conversions

#### String + number

Solved elsewhere: **Rust**: `"5" + 3` → compile error E0369 `cannot add {integer} to &str` ✓ — `Add` is only implemented for matching types, no coercion trait exists. **Python**: `"5"+3` → `TypeError: can only concatenate str (not "int") to str` ✓, while `f"{5}{3}"` → `"53"` ✓ — conversion is explicit, interpolation is the sanctioned route. **Julia**: `"5"*3` → `MethodError` ✓ — multiple dispatch has no `(String, Int)` method.
Adopt in Warp: `+` dispatches on resolved semantic types in elaboration; `Text + Int` has no method and yields a diagnostic with the two fix-its (`"5" + str 3`, `int "5" + 3`); codepoint arithmetic exists only for the `char` type, so `"5"+3 → 56` disappears because a one-char string literal is `Text`, not `char`.

#### Parsing numbers from text

Solved elsewhere: **Rust**: `"12a".parse::<i64>()` → `Err(ParseIntError { kind: InvalidDigit })` ✓ — returns `Result`, the whole string must match. **Swift**: `Int("12a")` → `nil` ✓ — failable initializer returns `Optional`. **Go**: `strconv.Atoi("12a")` → `0, invalid syntax` ✓ — error value alongside, lint-enforced checking. (Counter-example: JS `parseInt("12a")` → `12` ✓, prefix parsing.) Also **Haskell** `readMaybe "12a" :: Maybe Int` → `Nothing` ✓.
Adopt in Warp: `int "12a"` returns `Result Int ParseError` (DESIGN.md → Effects: `Error` as `Result`), requiring the full string to match; no transparent unwrapping, so using it as a number without handling the error is a type error. A prefix-parsing variant, if ever wanted, must be named as such (`int_prefix`).

#### Type annotations not enforced loudly

Solved elsewhere: **Swift**: `let x = 5; x = 6` → `error: cannot assign to value: 'x' is a 'let' constant` ✓ — constness is part of the binding, checked before codegen. **Julia**: `x::Int = 5; x = "five"` → `MethodError` (convert String→Int) ✓ — typed globals insert a checked `convert` on every assignment. **Rust/TypeScript**: `let x: i64 = 5; x = "five"` → `mismatched types` with span and `help:` fix-it — diagnostics carry spans and suggestions.
Adopt in Warp: bindings in semantic IR carry `{type, mutability}`; elaboration checks every assignment against them and emits a spanned diagnostic (with fix-it) instead of reaching the emitter; `const` and `::=` produce immutable bindings, so reassignment is rejected rather than ignored.

#### Lists and arithmetic

Solved elsewhere: **Julia**: `[1,2,3] .* 2` → `[2,4,6]` ✓, `[1,2,3] + 1` → `MethodError` ✓, `vcat([1,2],[3])` → `[1,2,3]` ✓ — broadcasting is a separate, explicit dot syntax; plain operators keep their algebraic meaning. **APL/J/NumPy** broadcast implicitly (`[1,2,3]*2` → `[2,4,6]`) but under fixed shape rules; **Python** `[1,2]+[3]` → `[1,2,3]` ✓ (concatenation) yet `[1,2,3]*2` → repetition ✓, i.e. `*` has an unrelated meaning.
Adopt in Warp: `+` on lists is concatenation (the list monoid, lawful: associative with `[]` as identity), `list * number` is a type error; element-wise arithmetic uses an explicit lifting form (Julia-style `.+`/`.*` or a `map`), which is exactly the "type-directed, law-governed lifting rule" DESIGN.md → Dangerous implicitness asks for. Never sum a list implicitly.

### Equality and identity

#### 🐞 String comparison

Solved elsewhere: **Rust**: `String::from("abc") == "abc"` → `true` ✓ — `==` is `PartialEq`, always by value; identity needs explicit `std::ptr::eq`. **Swift**: `==` is value equality for `String`, identity `===` exists only for class instances. **JavaScript**: `Object.is(NaN,NaN)` → `true` ✓ separates SameValue from `===`, showing the cost of having several equalities.
Adopt in Warp: `==` and `is` are structural value equality for all data (Text compared by content), elaborated per type rather than via "extract numeric value"; no user-visible identity operator. Mixed-type comparisons (`0==""`, `null==false`) are type errors, not panics and not coercions.

#### Unicode normalization

Solved elsewhere: **Swift**: `"\u{e9}" == "e\u{301}"` → `true` ✓ — `String ==` uses Unicode canonical equivalence on grapheme clusters. **Python/JS** make it explicit: `unicodedata.normalize("NFC", s)` ✓ / `s.normalize("NFC")` ✓ (plain `==` → `false` ✓). Rust `==` → `false` ✓ (bytes; `unicode-normalization` crate needed).
Adopt in Warp: normalize text literals and text read by the parser to NFC once (so storage is canonical and `==` stays a cheap byte compare); text entering at runtime from IO is normalized at the boundary, with an explicit `bytes`/raw type for when exact code points must be preserved.

#### Duplicate keys

Solved elsewhere: **Go**: `map[string]int{"a":1,"a":2}` → compile error `duplicate key "a" in map literal` ✓. **TOML 1.0**: `a=1\na=2` → `TOMLDecodeError: Cannot overwrite a value` ✓ — the spec forbids redefinition. **Python json** silently keeps the last (`{'a': 2}` ✓) but `object_pairs_hook` can reject it ✓; **Nix** `a // { a = 2; }` / JS `{...o, a:2}` show the explicit-override form.
Adopt in Warp: a literal with a repeated key is a parse diagnostic in data (TOML/Go rule); overriding is spelled explicitly in code (an update/merge form like `o with {a:2}` or Nix `//`), so "last wins" is never accidental.

### Strings

#### Bytes vs characters vs graphemes

Solved elsewhere: **Swift**: `"👍🏽".count` → `1` (and `.utf8.count` 8, `.utf16.count` 4, `.unicodeScalars.count` 2). The default `Character` is an extended grapheme cluster, the other units are separate named *views*, and `String.Index` is opaque, so `s[2]` does not compile.
**Rust**: `"👍🏽".len()` → `8` bytes, `.chars().count()` → `2`. Slicing at a non-char boundary panics (`is_char_boundary(1)` → false), it never returns half a code point. Graphemes need a crate (`unicode-segmentation`).
**JS (Intl.Segmenter)**: `[...new Intl.Segmenter().segment("👍🏽")].length` → `1`. It came late and is opt-in, while `.length` stays 4.
Adopt in Warp: use Swift's model. Text has named views `bytes`, `codepoints`, `graphemes`, and the default `#`, `for c in text` and `count` work on graphemes. Byte access is only possible through the `bytes` view (or `[]` if Warp keeps that split). An index that is not on a boundary is an error value, never a partial character like `'Ã'`.

### Truthiness and null

#### Empty values

Solved elsewhere: **Swift** ✓: `let xs:[Int]=[]; if xs {}` → compile error `cannot convert value of type '[Int]' to expected condition type 'Bool'`; you write `if xs.isEmpty` — conditions must be `Bool` (or an optional binding `if let`). **Go** ✓: `if x {}` with `x := 1` → `non-boolean condition in if statement`. **Ruby** ✓: `!![]`, `!!""`, `!!0`, `!!"0"` → all `true` — only `nil` and `false` are falsy, one rule with no per-type exceptions (the uniform alternative if Warp keeps truthiness at all).
Adopt in Warp: conditions take `Bool` or `T?` only (DESIGN.md); `if xs` on a collection/text is a structured diagnostic with fix-it `if not xs.empty` / `if xs.count > 0`. If backwards compatibility with the wiki is wanted, make it a *normalization hint*: accept `if xs` in the friendly surface, but elaborate it explicitly to `not empty xs` in the semantic IR, so one rule (Python's) applies uniformly and is visible in the resolved view. Either way, drop "unbound `x` is falsy" from the wiki.

#### `and`/`or` as ternary

Solved elsewhere: **Python 2.5+** ✓: `0 if 1 else 2` → `0` (vs `1 and 0 or 2` → `2` ✓) — PEP 308 added a real conditional expression precisely because the idiom breaks on falsy `x`. **Rust/Swift** ✓: `if true {0} else {2}` → `0` — `if` is an expression and `&&`/`||` only accept and return `bool`, so the idiom cannot typecheck. **Lua** ✓ still has the bug: `true and false or "y"` → `y` (no fix; shows the cost).
Adopt in Warp: make `and`/`or` Bool-typed (return `Bool`, not an operand), which dissolves the footgun under the strict-condition rule above; until then, lint the shape `a and b or c` with fix-it `if a then b else c`. Value-selecting defaulting belongs to a dedicated `??` / `or else` on `T?` (see Null), not to `or`.

#### Null

Solved elsewhere: **Swift** ✓: `let p:P? = nil; p?.name ?? "anon"` → `anon`; `p + 1` on `Int?` → compile error `value of optional type 'Int?' must be unwrapped`. **Kotlin**: `val s:String? = null; s.length` → compile error; `if (s != null) s.length` compiles via flow-sensitive smart casts. **Rust** ✓: `None::<i32>.map_or(0,|x|x+1)` → `0` — no null at all, `Option<T>` is an ordinary enum with exhaustive `match`.
Adopt in Warp: exactly [wiki/null.md](wiki/null.md) + DESIGN.md's `Option(T)`: `T?` in the semantic type, Kotlin-style flow narrowing after `if x` / `if x != ø`, `?.` and `??` as the only implicit paths, and `x=ø; x+1` becoming a spanned diagnostic "`x` is `Int?`, unwrap with `x ?? 0` or check `if x`" instead of a panic. No transparent unwrapping (DESIGN.md).

### Syntax and precedence

#### Unary minus and power

Solved elsewhere: **Python / Julia / Haskell**: `-2**2` / `-2^2` → `-4` — `**`/`^` binds tighter than prefix minus, as in mathematics.
**JavaScript**: `-2**2` → *SyntaxError: Unary operator used immediately before exponentiation expression* — the ambiguous form is simply rejected; `(-2)**2` → `4`.
**Rust**: `-2i32.pow(2)` → `-4` — method call binds tighter than unary minus.
Adopt in Warp: give `^` (and `²`, `³`) higher precedence than prefix `-`, so `-2^2` → `-4` like Python/Julia; a literal `-2` token is still negation of `2^…`, never a signed base. Optionally emit a normalizer hint recommending `-(2^2)` / `(-2)^2` in the canonical view.

#### `not` and bitwise operators vs comparison

Solved elsewhere: **Python**: `not 1==2` → `True`, `3 & 4 == 4` → `False` — `not` binds weaker than comparisons; `&`,`|`,`^` bind *tighter* than comparisons (fixing C's historical order).
**Go / Rust**: `3&4 == 4` → `false` — same fix: bitwise ops at multiplicative/additive level, above `==`.
**Carbon** *(not run)*: precedence is a *partial order*; `a & b == c` is a compile error demanding parentheses.
Adopt in Warp: `not`/`and`/`or` below comparisons (Python), and either Python's "bitwise above comparison" or Carbon's partial order (mixing bitwise with comparison without parentheses is a diagnostic with a fix-it). The partial order fits DESIGN.md's "never guess" best; `|` staying pipe means bitwise or needs its own spelling (`bitor`, `∨`?) anyway.

#### Chained comparison

Solved elsewhere: **Python / Julia**: `3>2>1` → `True`, `1<3>2` → `True` — `a<b<c` desugars to `a<b and b<c` with `b` evaluated once.
**Rust**: `1 < 2 < 3` → *error: comparison operators cannot be chained; help: split the comparison into two* — refuses rather than guesses.
**Haskell**: `3 > 2 > 1` → *Precedence parsing error: cannot mix ‘>’ [infix 4] and ‘>’ [infix 4]* — comparisons are declared non-associative (`infix 4`).
Adopt in Warp: Python/Julia chaining for same-direction chains (`a<b<c`, `a≤b<c`, `a==b==c`) desugared in elaboration with the middle operand bound once; mixed-direction chains like `1<3>2` (legal but confusing in Python) are a diagnostic.

#### Assignment in a condition

Solved elsewhere: **Python**: `if x = 2:` → *SyntaxError: invalid syntax. Maybe you meant '==' or ':=' instead of '='?* — `=` is a statement; binding inside an expression needs the distinct walrus `:=`.
**Swift**: `if i = 2 {}` → *error: use of '=' in a boolean context, did you mean '=='?* ; **Rust**: `if x = 2 {}` → *mismatched types, help: you might have meant to compare for equality* — assignment has type `()`/`Void`, never Bool.
**Kotlin** *(not run)*: "Assignments are not expressions, and only expressions are allowed in this context".
Adopt in Warp: assignment evaluates to unit, so in a Bool-expected position (condition of `if`/`while`) `=` is elaborated as comparison (as [wiki/Bad.md](wiki/Bad.md) wants) *or* rejected with a fix-it — pick one and persist it; `if 1=2` must never be truthy. Bidirectional typing (Bool expected) makes this an elaboration rule, not a parser heuristic.

#### 🐞 Increment

Solved elsewhere: **Swift** (removed ++ in Swift 3, SE-0004): `i++` → *error: cannot find operator '++' in scope; did you mean '+= 1'?*
**Rust**: `i++` → *error: Rust has no postfix increment operator; help: use `+= 1` instead*.
**Go**: `j := i++` → *syntax error: unexpected ++* — `i++` exists but only as a statement, never an expression, so `i++ + i++` cannot be written.
Anti-example **Python**: `i=1; ++i` → `1` silently (`+(+i)`), a footgun of its own.
Adopt in Warp: Go's rule — `x++`/`++x` is a statement meaning `x += 1` (both spellings identical, per wiki/equality.md), its value is unit so it can't be used inside an expression; `++` applied to a non-place is a diagnostic, never `+(+x)`. First fix the bug that `x=1;x++;x` → `1`.

#### Braceless calls

Solved elsewhere: **Ruby**: `f 3-1` → `20` (argument is the whole expression) and `1 + f 3` → *SyntaxError* — a braceless call is only allowed where it is unambiguous, otherwise loud; `f -1` → warning *ambiguous first argument*.
**Haskell**: `f 3-1` → `29`, `1 + f 3` → `31` — the opposite, but *one* fixed rule (application binds tightest) plus `$` for "rest of line": `f $ 3-1` → `20`.
**Julia**: `2x` → `6` — juxtaposition restricted to numeric-literal coefficients, where it cannot mislead.
Adopt in Warp: keep Warp's chosen rule (braceless call takes the whole remaining argument expression, like Ruby/`$`) and apply it uniformly also in operand position, so `1 + f 3` → `1 + f(3)` → `31`; `fib it-1 + fib it-2` then needs the known arity of `fib` (1) to stop the argument at the next `+`-level call — resolve that with the declared signature in elaboration, and where arity is unknown emit a diagnostic instead of a parse.

### Mutation and scope

#### Aliasing

Solved elsewhere: **Swift**: `var a=[1]; var b=a; b[0]=9; print(a)` → `[1]` (verified) — Array/String/Dictionary are value types with copy-on-write: the buffer is shared until a write, `isKnownUniquelyReferenced` decides whether to copy.
**Rust**: `let mut b = a; b[0]=9; a` → *compile error E0382, use of moved value* — assignment moves ownership; sharing needs an explicit `.clone()` or `&`/`Rc`.
**Clojure / Immutable.js**: `(let [a [1] b (assoc a 0 9)] a)` → `[1]` — persistent data structures with structural sharing, mutation returns a new value.
Adopt in Warp: Swift's model is exactly DESIGN.md's ownership rule 5: lists/text/records are values, `b=a` shares the GC ref, and a write through `b#1=` copies unless the ownership pass proves `b` unique (then mutate in place). This keeps `=` cheap and the observable semantics identical whichever plan is chosen.

#### Mutable default arguments

Solved elsewhere: **Swift**: `func f(_ xs:[Int]=[]) {var ys=xs; ys.append(1); return ys}; f(); f()` → `[1] [1]` (verified) — default expressions are re-evaluated at every call site, and arrays are values anyway.
**Kotlin / C++ / JS**: `fun f(a: MutableList<Int> = mutableListOf())` → fresh list per call — defaults are call-site expressions, not objects stored on the function.
(**Python** workaround `def f(a=None): a = [] if a is None else a` → `[1] [1]` (verified) shows the cost of the wrong default.)
Adopt in Warp: define a default as an expression elaborated at the call site (inserted into the argument list in semantic IR), never a value stored once with the function; with value semantics the footgun is doubly impossible. The current panic on `def f(a=())` must become a type/elaboration error with span if it is not fixed.

#### Closures capturing loop variables

Solved elsewhere: **Go ≥ 1.22**: `for i:=0;i<3;i++ { fs=append(fs, func()int{return i}) }` → `0 1 2` (verified) — each iteration gets a fresh `i` (loopvar semantics change).
**JS `let`**: `for (let i=0;i<3;i++) fs.push(()=>i)` → `[0,1,2]` (verified) — per-iteration binding, unlike `var`.
**Rust / Swift**: `(0..3).map(|i| move || i)` → `[0,1,2]` (verified) — loop variables are immutable per-iteration bindings; `move` captures by value, capturing a mutable by reference that outlives it is a borrow error.
Adopt in Warp: loop variables are fresh immutable bindings per iteration and closures capture by value (a reassignable outer local captured by a closure is either copied at capture time or rejected with a diagnostic, never shared by reference silently). This fits the "immutable local bindings" core and makes closures pure by default.

### Bounds

#### Index out of range / negative index

Solved elsewhere: **Rust**: `vec![1,2,3].get(3)` → `None`, `.last()` → `Some(3)` (verified); `a[3]` panics with the index and length — checked indexing is the default, the unchecked form is `unsafe get_unchecked`.
**Go**: `a[3]` → `runtime error: index out of range [3] with length 3` (verified) — always bounds checked, message carries index and length.
**JS `Array.at`**: `[1,2,3].at(-1)` → `3` (verified) — negative indexing only through an explicit method; plain `a[-1]` / `a[3]` stay `undefined` (the footgun). **Swift**: `arr.indices.contains(3)` / `arr.last` → optional, `arr[3]` traps.
Adopt in Warp: two spellings, both checked: `x#i` traps/returns an error value carrying span, index and length (1-based, so `#0` is always an error), and a total variant (e.g. `x#?i` or `x.get i`) returns `Option<T>` for code that wants to branch. Negative counting only via an explicit `last`/`from end` form, never by wrap-around of `#-1`.

### Data formats

#### The Norway problem

Solved elsewhere: **TOML 1.0**: `country = NO` → *parse error*, so it has to be written `"NO"`. The only booleans are lowercase `true`/`false`, bare words are not values, and strings must be quoted.
**YAML 1.2 core schema / StrictYAML**: 1.2 cut the boolean set down to `true|false` (in any case); StrictYAML treats every scalar as a string until a schema says otherwise. (PyYAML and Ruby Psych still use 1.1: `NO` → `false`, verified.)
**JSON**: `true`/`false` only; a bare `NO` is a syntax error.
Adopt in Warp: data literals should accept only `true`/`false` (plus `✔`/`✖` if they are wanted) as booleans. `yes`/`no`/`on`/`off` should be symbols in data. In code, a symbol reaches `Bool` only through an expected-type coercion that the elaborator records explicitly. The value then depends on the schema, not on the spelling.

#### Numbers that are not numbers

Solved elsewhere: **TOML 1.0**: `zip = 01234` → *parse error*, because leading zeros are forbidden. That makes `"01234"` the only way to write it, so a leading zero can never be silently dropped.
**Python json**: `json.loads('{"v":1.10}', parse_float=Decimal)` → `Decimal('1.10')`. The decimal keeps its trailing zero because the parser takes a hook for number construction.
**JS (Node 26, JSON.parse source text access)**: `JSON.parse('{"v":1.10}', (k,v,ctx)=>ctx.source ?? v)` → `{v:'1.10'}`. The reviver sees the original lexeme; `JSON.rawJSON` does the same when serializing.
Adopt in Warp: every numeric literal in `Meta` keeps its source lexeme, so serialization round-trips `1.10` and `01234` byte for byte. When no numeric type is expected, a lexeme with a leading zero is a symbol/text with a diagnostic, as in TOML, not an octal or truncated number. This matches DESIGN.md's rule that `Node` preserves "data literals exactly enough for round-trip serialization".

#### Data that executes

Solved elsewhere: **PyYAML**: `yaml.safe_load("!!python/object/apply:os.system ['echo pwned']")` → `ConstructorError`. The safe loader has no constructor for language-object tags, so a tag cannot produce code (plain `yaml.load` without `SafeLoader` used to run it).
**JSON / Rust serde**: `serde_json::from_str::<Config>(s)` produces only the declared type. The format has no tags that could name executable things, and the target type is fixed by the caller.
**Deno**: `deno run script.ts` with no `--allow-*` flags means the code cannot read files, use the network or read env. Capabilities are deny-by-default and granted per run.
Adopt in Warp: add a `warp read file.wasp` / `load(text)` entry point that stops at `Node` and cannot evaluate, and make it the default for data files. Evaluating foreign code should run as a WASM component with an empty import set, plus only the capabilities the caller grants explicitly (the WIT imports backstop in DESIGN.md → Effects as enforced capabilities).

### Errors

#### Swallowed errors

Solved elsewhere: **Rust** ✓: `#[must_use] fn g()->Result<..>; g();` → `unused Result that must be used` (deny-able to an error); `?` propagates in one character. **Swift** ✓: calling a `throws` function without marking it → compile error `call can throw but is not marked with 'try'`; `(try? f()) ?? -1` → `-1` is the *explicit* swallow, and `Int("12a")` → `nil` ✓ (not `0`). **Zig**: error unions must be handled (`try`, `catch`, or `catch unreachable`); discarding one is a compile error.
Adopt in Warp: `Result<T,E>` as data with Rust's "must use" as a hard error, `?`/`try` for propagation, and any discard spelled explicitly (`try? f()`, `f() or else default`) so it is greppable. Concretely: `int("12a")` must return `Int?`/`Result`, never `0`; `x[3]` out of range returns `T?` or a spanned error; compiler panics (`Cannot extract numeric value …`) become `Diagnostic`s with span + resolved facts + fix-it.

### Variance

Solved elsewhere: **Kotlin / Scala / C#**: `List<out E>` read-only is covariant, `MutableList<E>` is invariant — declaration-site variance checked by the compiler, so Java's `Object[] a = new String[1]; a[0]=1` (verified: `ArrayStoreException` at runtime) cannot type check.
**Rust**: `&T` covariant, `&mut T` invariant, inferred from usage: pushing a short-lived `&str` into a `Vec<&'static str>` via `&mut` → `E0597 s does not live long enough` (verified).
**Java generics** (use-site `List<? extends Number>`) show the verbose alternative.
Adopt in Warp: infer variance from use like Rust instead of annotating it: immutable values (the default) are covariant, a parameter or place that is written through (a mutation effect on it) is invariant. Since collections are values with copy-on-write, most code never meets invariance at all.

## "Impossible"

### Exact real numbers

Solved elsewhere: **Mathematica / SymPy**: `Sqrt[2]*Sqrt[2] == 2` → `True`, `sqrt(2)**2` → `2` *(not run)*: algebraic numbers are kept symbolic and simplified.
**Android calculator (Boehm's constructive reals, `CR`/`UnifiedReal`)**: displays `√2·√2` as exactly `2` by combining rationals × known irrationals with lazily evaluated arbitrary precision; equality is decided when it is provable and otherwise reported as "equal to N digits" *(not run)*.
**Haskell**: `0.1 + 0.2 == (0.3 :: Rational)` → `True`: the exact tower stops at ℚ, and `sqrt` is simply not defined on `Rational`, so the type system says where approximation begins.
Adopt in Warp: Boehm's approach: represent values as rational × (known symbolic factor) when possible, fall back to lazily refined constructive reals, and make `==` on such values three-valued (true / false / "equal to N digits"), so the type reports when a result is only approximate.

### Fast *and* lawful floats

Solved elsewhere: **Python**: `(0.1+0.2)+0.3 == 0.1+(0.2+0.3)` → `False`, but `math.fsum([0.1,0.2,0.3]) == 0.6` → `True`: exactly-rounded summation (Shewchuk) recovers the lawful answer for the most common case.
**Julia** `sum` uses pairwise summation and `@fastmath` is a *local, explicit* opt-in to reassociation *(not run)*; **Rust** has no global fast-math flag at all: reassociation is only allowed through explicit intrinsics (`fadd_fast`, nightly) *(not run)*.
**Herbie** (tool) rewrites float expressions to more accurate equivalents automatically *(not run)*.
Adopt in Warp: keep IEEE semantics bit-exact under `@f64` (no global fast-math), make reductions like `sum` exactly rounded or compensated by default, and allow reassociation only through a scoped annotation (`@fastmath`) so `law` checks know which laws hold where.

### "The" length of a string

Solved elsewhere: **Swift**: `s.count` / `s.utf8.count` / `s.utf16.count` / `s.unicodeScalars.count` → `1/8/4/2` for `"👍🏽"`. Every unit has a name, and only the grapheme count gets the short name.
**JS Intl / ICU**: `"i".toLocaleUpperCase("tr")` → `İ`, while `"i".toUpperCase()` → `I`. Locale-dependent case mapping takes the locale as an explicit argument; the default is locale-independent.
**Python**: `unicodedata.unidata_version` exposes the Unicode version, so the segmentation rules can at least be seen and pinned.
Adopt in Warp: a string has no `length`. It has `bytes.count`, `codepoints.count` and `graphemes.count` (`count` means graphemes), and case mapping takes an explicit locale or defaults to the invariant locale. The Unicode version used for segmentation should be recorded in the semantic artifact so results are reproducible across builds.

### Guessing intent

Solved elsewhere: **Unison** *(not run)*: code is stored as a content-addressed AST; names are resolved once at `add`/`update` time and the hash, not the text, is what later builds consume — exactly "resolve once, persist".
**Python** (`:=` walrus) / **Pascal/Ada** (`:=` vs `=`): assignment, definition and comparison get *distinct tokens*, so no guess is needed.
**Rust/Swift/Python diagnostics**: `did you mean '=='?` — the compiler proposes, the programmer accepts (machine-applicable fix-it, `cargo fix`).
Adopt in Warp: keep the DESIGN.md plan (content-addressed resolution sidecar, Unison-style); canonical view uses distinct spellings (`:=` define, `=` assign, `==` compare) so an accepted resolution can be written back into source, and fix-its are machine-applicable so agents/`warp fix` persist the choice.
