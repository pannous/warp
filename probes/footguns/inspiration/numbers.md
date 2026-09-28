# Footgun inspiration: numbers

Verified locally on 2026-09-26 unless marked *(not run)*; scripts in `probes/footguns/inspiration/scratch/`
(`num_py.py`, `numbers.js`, `NumHs.hs` via `runghc`, `numbers.jl`, `num_rs.rs`, `num_go.go`, `num.swift`, `trap2.swift`).
Pitfall found along the way: a scratch file named `numbers.py` shadows Python's stdlib `numbers` and breaks `import fractions`.
`ghc` cannot link on this machine (the Swift toolchain's clang is on PATH and lacks `inttypes.h`); `runghc` works.

### Decimal fractions
Solved elsewhere: **Haskell**: `1%10 + 2%10 == (3%10 :: Rational)` → `True`; also `0.1 + 0.2 == (0.3 :: Rational)` → `True`: decimal literals are overloaded (`fromRational`), so at type `Rational` they are exact.
**Go**: `0.1+0.2 == 0.3` → `true`: untyped constants are evaluated at arbitrary precision at compile time, and only rounded when they get a concrete type.
**Julia**: `1//10 + 2//10 == 3//10` → `true` (`Rational{Int}`; `//` builds exact rationals). **Python** `Decimal("0.1")+Decimal("0.2")==Decimal("0.3")` → `True`, but only with an opt-in type.
Adopt in Warp: the Haskell/Go model: a literal like `0.1` means the exact value 1/10 and representation is chosen late (as Go does for constants, but for all values); `@f64` is the only place rounding enters. `.1` should parse like `0.1`.

### 🐞 Sum of quotients truncated
Solved elsewhere: **Python**: `Fraction(1,4)+Fraction(1,4)` → `1/2`; **Julia**: `1//4+1//4` → `1//2`; **Haskell** `1%4 + 1%4` → `1 % 2`. The result type of `+` is computed from the operand types (a numeric tower/promotion rule: `Rational + Rational → Rational`), never from a default.
Adopt in Warp: result types of arithmetic come from a promotion table over the operand types (Int ⊂ Rational ⊂ Real), checked in elaboration; a function's return type is never defaulted to Int ([DESIGN.md](../../../DESIGN.md) "unknown names and unsolved overloads are errors, never `Symbol` or `Int` defaults").

### 64 bit overflow
Solved elsewhere: **Python**: `2**63, 2**64, abs(-2**63)` → `9223372036854775808 18446744073709551616 9223372036854775808`: `int` is arbitrary precision (small ints fast path, bignum on demand). **Haskell** `2^63 :: Integer` → `9223372036854775808` (and `Integer` is the default type).
**Swift**: `Int.max + 1` → traps at runtime (verified: process aborts), wrapping only via the explicit `&+` → `-9223372036854775808`, and `addingReportingOverflow`.
**Rust**: `i64::MAX.checked_add(1)` → `None`, `overflowing_add` → `(-9223372036854775808, true)`: the operation names the policy (`checked_/wrapping_/saturating_/overflowing_`).
Adopt in Warp: Python/Haskell semantics (Int is ℤ, i64 is an inferred representation that promotes to bignum on overflow, e.g. via an overflow check on the fast path as in [wiki/int60.md](../../../wiki/int60.md)), plus Swift-style explicit wrapping operators or an `@wrap` / `@i64` representation annotation for code that wants modular arithmetic.

### Big literals silently corrupted
Solved elsewhere: **Rust**: `let _x: i64 = 100000000000000000000;` → `error: literal out of range for 'i64'` (deny-by-default lint).
**Go**: `var x int64 = 100000000000000000000` → `cannot use 100000000000000000000 (untyped int constant) as int64 value in variable declaration (overflows)`, with the exact column.
**JS**: `100000000000000000000n` → `100000000000000000000n`; **Python** `100000000000000000000` → the exact int.
Adopt in Warp: until bignums exist, any integer literal outside i64 is a compile error with the span (Go/Rust); once Int is unbounded, the literal is simply a bignum (Python). Never a NUL string.

### Scientific notation and digit separators
Solved elsewhere: **Python/Rust/Julia/JS/Go**: `1_000_000` → `1000000`, `1e3` → `1000.0` (Python, Julia) / `1000` (JS, Go, Rust prints `1000`). Separators are lexical: `_` between digits is ignored by the tokenizer, and a digit sequence followed by `e[+-]?digits` is one token.
**Python**: `int("1_000")` → `1000`: the same grammar is used for parsing text at runtime.
**Go**: `1e3` is an *untyped* constant usable as an int (`const n int = 1e3; fmt.Println(n)` → `1000`), i.e. the exponent does not force float.
Adopt in Warp: lex both in the number token (the tokenizer, not the list parser); in exact mode `1e3` is the integer 1000 and `1.5e-3` the rational 3/2000 (Go's untyped constants), so exponent notation never implies a float. `1e3` staying the list `1 e3` should become at least a warning.

### NaN and infinity
Solved elsewhere: **Python**: `1/0` → `ZeroDivisionError: division by zero`, `math.sqrt(-1)` → `ValueError`, `cmath.sqrt(-1)` → `1j` (the complex answer is a separate, explicit module).
**JS BigInt / Python Fraction**: `1n/0n` → `RangeError: Division by zero`: exact types have no NaN/Inf, so division by zero is an error.
**Julia**: `isequal(NaN,NaN)` → `true` vs `NaN==NaN` → `false`; **JS** `Object.is(NaN,NaN)` → `true`, `[NaN].includes(NaN)` → `true` but `indexOf` → `-1`: a second, reflexive "same value" relation used for collections and hashing.
Adopt in Warp: exact numbers make `1/0` an `Error` value (Python/BigInt) and `sqrt(-1)` a `Complex` only when the result type allows it (as `cmath`); under `@f64` keep IEEE `==` but use a reflexive total-order equality (Julia `isequal`, IEEE 754 `totalOrder`) for `is`, dict keys, sorting and `law` checks.

### Rounding mode
Solved elsewhere: **Julia**: `round(2.5)` → `2.0`, `round(2.5, RoundNearestTiesAway)` → `3.0`, `round(2.5, RoundUp)` → `3.0`: the mode is a named argument with a documented default.
**Rust**: `(2.5f64).round()` → `3`, `(2.5f64).round_ties_even()` → `2`: two differently named functions, no hidden default.
**Python Decimal**: `Decimal("2.5").quantize(Decimal("1"), rounding=ROUND_HALF_UP)` → `3`; `round(2.5)` → `2`. **Swift** `(2.5).rounded()` → `3.0`, `.rounded(.toNearestOrEven)` → `2.0`.
Adopt in Warp: `round(x)` plus a named mode argument (`round x half:even`, `half:up`, `half:away`, `down`, `up`), with the default spelled in the signature; for exact rationals the rounding is exact (no double-rounding of `2.675`-style decimals).

### Negative modulo
Solved elsewhere: **Haskell**: `(-5) \`mod\` 3` → `1`, `(-5) \`rem\` 3` → `-2`; **Julia** `mod(-5,3)` → `1`, `rem(-5,3)` → `-2`: two named operators, one per convention.
**Rust**: `(-5i64).rem_euclid(3)` → `1` while `-5 % 3` → `-2`. **Python**: `-5 % 3` → `1`, `divmod(-5,3)` → `(-2, 1)` and `math.fmod(-5,3)` → `-2.0`, with the law `a == (a//b)*b + a%b` holding for floored division.
Adopt in Warp: Haskell/Julia naming, `%`/`mod` floored (sign of divisor) and `rem` truncating, and state the division law pairwise (`a == b*floor_div(a,b) + a%b`) as a `law` in the standard library so both pairs are checked.

### Booleans are integers
Solved elsewhere: **Haskell**: `True + True` → `No instance for 'Num Bool'`; **Rust**: `true + true` → `error[E0369]: cannot add 'bool' to 'bool'`; conversion is explicit (`fromEnum True`, `true as i64`).
**JS BigInt**: `1n + true` → `TypeError` (the newer numeric type refused the old coercion). Contrast **Python**: `True+True` → `2`, `isinstance(True,int)` → `True`; **Julia** `true+true` → `2` (Bool <: Integer).
Adopt in Warp: `Bool` is its own kind in semantic IR (it may still be encoded as i32 0/1 in WASM); arithmetic on it is a type error, with explicit `int(b)` / counting via `count(xs, pred)`.

### Fast *and* lawful floats
Solved elsewhere: **Python**: `(0.1+0.2)+0.3 == 0.1+(0.2+0.3)` → `False`, but `math.fsum([0.1,0.2,0.3]) == 0.6` → `True`: exactly-rounded summation (Shewchuk) recovers the lawful answer for the most common case.
**Julia** `sum` uses pairwise summation and `@fastmath` is a *local, explicit* opt-in to reassociation *(not run)*; **Rust** has no global fast-math flag at all: reassociation is only allowed through explicit intrinsics (`fadd_fast`, nightly) *(not run)*.
**Herbie** (tool) rewrites float expressions to more accurate equivalents automatically *(not run)*.
Adopt in Warp: keep IEEE semantics bit-exact under `@f64` (no global fast-math), make reductions like `sum` exactly rounded or compensated by default, and allow reassociation only through a scoped annotation (`@fastmath`) so `law` checks know which laws hold where.

### Exact real numbers
Solved elsewhere: **Mathematica / SymPy**: `Sqrt[2]*Sqrt[2] == 2` → `True`, `sqrt(2)**2` → `2` *(not run)*: algebraic numbers are kept symbolic and simplified.
**Android calculator (Boehm's constructive reals, `CR`/`UnifiedReal`)**: displays `√2·√2` as exactly `2` by combining rationals × known irrationals with lazily evaluated arbitrary precision; equality is decided when it is provable and otherwise reported as "equal to N digits" *(not run)*.
**Haskell**: `0.1 + 0.2 == (0.3 :: Rational)` → `True`: the exact tower stops at ℚ, and `sqrt` is simply not defined on `Rational`, so the type system says where approximation begins.
Adopt in Warp: Boehm's approach: represent values as rational × (known symbolic factor) when possible, fall back to lazily refined constructive reals, and make `==` on such values three-valued (true / false / "equal to N digits"), so the type reports when a result is only approximate.
