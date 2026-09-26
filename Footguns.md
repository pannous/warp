# Footguns

Footguns in other programming languages and how they are avoided in Warp 

# Solved
You may not be aware, but in other languages, you might stumble upon these questions with wrong answers. 
.1 + .2 == .3 ?  Warp : yes! how? ...
> Status 2026-09-26: this headline is the goal, not the implementation yet. `.1` does not parse and
> `0.1+0.2==0.3` evaluates to `0`, see [NOT YET → Numbers](#numbers). Intended answer: numeric literals are exact
> rationals ([DESIGN.md → Exact numbers by default](DESIGN.md#exact-numbers-by-default)), `0.1` *is* 1/10,
> so the sum is exactly 3/10; IEEE floats are an inferred or requested representation (`@f64`), never the meaning.
...

How to read the entries: **offending language**: `one-liner` → *its wrong answer*; then what Warp does.
Every Warp answer below was produced by running the real compiler (`target/debug/warp eval …`, WASM GC round trip):
* `probes/footguns/footguns.sh` evaluates all cases in `probes/footguns/cases.warp`, output in `probes/footguns/results.txt`
* `tests/probe_footguns.rs` pins the Solved entries as `is!` tests; its `#[ignore = "next"]` tests are the NOT YET entries with a clear intended answer

Only entries whose Warp answer was verified are listed here; everything else is under NOT YET.

### Integer division
**C, Java, Go, Python 2**: `7/2` → *3*  
Warp: `7/2` → `3.5`. `/` is always division; truncation must be asked for. (`test_division_is_not_truncating`)

### Octal literals
**C, Java, sloppy JS**: `010` → *8*  
Warp: `010` → `10`. A leading zero never switches the base; bases are explicit (`0x10` → `16`). (`test_leading_zero_is_not_octal`)

### Loose equality
**JavaScript, PHP**: `1 == "1"` → *true* (and `"0" == false` → *true*)  
Warp: `1=="1"` → `false`. No implicit string↔number coercion in comparisons. (`test_no_loose_equality`)

Solved elsewhere: **Python/Ruby**: `1 == "1"` → `False` ✓ — no cross-type coercion, `==` of unrelated types is simply false. **Haskell/Rust**: `1 == "1"` → compile-time type error — `Eq`/`PartialEq` only for the same type. **Scheme** separates numeric `=` (`(= 1 1.0)` → #t) from structural `equal?`.
Adopt in Warp: keep `1=="1"` → false today, but aim for the Haskell/Rust answer: comparing unrelated types is a diagnostic (almost always a bug), while numeric kinds compare by exact value across representations (`3 == 3.0` true, consistent with wiki/equality.md "compatible" and exact rationals).

### Integers silently becoming doubles
**JavaScript, JSON parsers**: `9007199254740993` → *9007199254740992*  
Warp: `9007199254740993` → `9007199254740993`; integers are 64 bit, not doubles. (`test_integers_beyond_double_precision`)

### 32 bit overflow
**Java, C#, C (`int`)**: `2147483647 + 1` → *-2147483648*  
Warp: `x=2147483647;x+1` → `2147483648`. (64 bit: see Integer overflow below.) (`test_integers_beyond_double_precision`)

### Power associativity
**Excel, some calculators**: `=2^3^2` → *64*  
Warp: `2^3^2` → `512`, right associative like mathematics. (`test_power_is_right_associative`)

### Automatic semicolon insertion
**JavaScript**: `x = 1⏎-1` → *x == 0* (no semicolon is inserted before a line starting with `-`; likewise `return⏎{a:1}` → *undefined*)  
Warp: `x=1⏎-1⏎x` → `1`. A newline ends the statement. (`test_newline_ends_statement`)

Solved elsewhere: **Go**: `x := 1⏎-1` → *compile error: -1 (untyped int constant) is not used* — the lexer inserts `;` after a line ending in an identifier/literal/`)`/`}`/`return`/`++`, a purely lexical, documented rule, and a dangling expression statement is an error.
**Python**: newline ends a statement unless inside brackets or after `\`, so `return⏎{…}` returns `None` *and* the dict line is a separate (dead) statement.
**JavaScript** (the footgun, verified): `let x=1⏎-1` → `x == 0`.
Adopt in Warp: already solved (newline ends the statement); add Go's second half — a bare value statement whose result is discarded (`-1` on its own line mid-block) is a diagnostic, and continuation only inside open brackets or after a trailing binary operator.

### Braceless call grabbing too little
**Ruby, Haskell-style juxtaposition** ([wiki/Bad.md](wiki/Bad.md) feared it): `fibonacci number-1` read as *(fibonacci number)-1* → infinite recursion  
Warp: `f := it*10; f 3-1` → `20`, the call takes the whole argument expression `f(3-1)`. (`test_braceless_call_takes_whole_argument`; but see NOT YET → Braceless calls)

### Parameter shadowing
**Many languages** accidentally read the outer variable when a parameter has the same name.  
Warp: `x=1;f(x):=x*2;f(5)+x` → `11`, the parameter shadows, the outer `x` is untouched. (`test_parameter_shadows_outer_variable`)

Solved elsewhere: **Swift**: `let x=1; func g(x:Int)->Int{x*2}; g(x:5)+x` → `11` (verified) — lexical scoping, parameter wins, outer untouched.
**Haskell (`-Wname-shadowing`), Go (`go vet -vettool shadow`), Kotlin ("Name shadowed" warning)** — shadowing is legal but the compiler reports it, catching the "meant the outer one" bug.
**Rust**: `let x = x + 1` shadowing is idiomatic, but an unused outer binding triggers `unused_variables`.
Adopt in Warp: keep lexical parameter shadowing (already Solved) and add a lint-level diagnostic with span when a parameter or local shadows an outer binding that is then used in the same function, with a fix-it rename; never an error, since data formats reuse names.

### "0" is falsy
**PHP, Perl**: `if ("0")` → *false*  
Warp: `if "0" {1} else {2}` → `1`; only the number `0` is falsy: `if 0 {1} else {2}` → `2`. (`test_zero_string_is_truthy`)

Solved elsewhere: **Ruby** ✓: `!!"0"` and `!!0` → `true` — no string or number is falsy. **Python/JS** ✓: `bool("0")` / `!!"0"` → `True`/`true` — only the *empty* string is falsy, the content is never inspected. **Swift/Go/Rust** ✓: `if "0"` is a type error — strings are never conditions.
Adopt in Warp: already solved (`if "0"` → `1`). Under the strict-condition rule it would become a type error with fix-it `if s.empty` / `if int(s) != 0`, which is stronger; note that the same rule also turns Warp's currently-accepted `if 0 {…}` into an error (fix-it `if n != 0`), so pin that decision together with *Empty values*.

### Undefined variables
**JavaScript**: `a + 1` → *NaN*; `a = 1` in sloppy mode silently creates a global; **Perl/PHP**: *0*/warning  
Warp: `a+1` → compile error `Undefined variable: a`. (`test_undefined_variable_is_an_error`; the error is a panic, not yet a structured diagnostic)

Solved elsewhere: **JavaScript strict mode / ES modules** ✓: `"use strict"; undeclared = 1` → `ReferenceError` (sloppy mode silently created a global). **Python** ✓: `a+1` → `NameError: name 'a' is not defined` (at runtime only). **Rust/Go/Swift**: unresolved names are compile errors, with "did you mean `abc`?" suggestions (rustc uses edit distance over names in scope).
Adopt in Warp: keep the compile-time error, but make it a structured diagnostic (span, names in scope) with a Rust-style *suggestion* — the suggestion is a fix-it the user/agent accepts, never an automatic rebinding (DESIGN.md forbids "guessing unresolved identifiers from nearby names"). Remove the contradictory "`x==false` if unbound" rule from wiki/truthiness.md.

### Hidden side effects / "pure" functions that are not
**Every mainstream language**: a helper deep in the call tree prints, logs or phones home and nothing says so.  
Warp: `log(x) := puts x⏎square(x) := log(x) ! Pure⏎square(3)` →
`Error('effect violation at 2:1: square is declared ! Pure but performs IO via square → log → puts')`.
Effects are inferred along the call chain and a module without IO calls has no WASI import at all, so it *cannot* do IO
([DESIGN.md → Effects as enforced capabilities](DESIGN.md#effects-as-enforced-capabilities)). (`test_hidden_side_effect_is_rejected`, `tests/test_effects.rs`)

### Integer overflow
**C, Java, Go, Rust --release**: `INT64_MAX + 1` → *INT64_MIN*; `x*x` for `x = 3037000500` → *-9223372036709301616*; **Java**: `Math.abs(Long.MIN_VALUE)` → *negative*  
Warp (since fcbd300b): Int is unbounded, an i64 fast path promotes to BigInt on overflow:
`square(x) := x*x; square(3037000500)` → `9223372037000250000`, `2^64` → `18446744073709551616`, `abs(-2^63)` → `9223372036854775808`,
`2^100` → `1267650600228229401496703205376`. Wrapping is an explicit opt-out: `(2^63) as i64` → `-9223372036854775808`
(🐞 `as i64` inside a function body still panics, `test_explicit_wrap_inside_function`).
`law square(x) >= 0` now holds at runtime; `law` still turns a false property into a loud failure:
`dec(x) := x-1⏎law dec(x) >= 0⏎dec(0)` → `Error('law (dec x)>=0 violated: counterexample x=0')`
([DESIGN.md → Progressive verification](DESIGN.md#progressive-verification-law)).
(`test_integer_overflow_does_not_wrap`, `test_law_holds_because_integers_do_not_wrap`, `test_law_catches_a_false_property`, `tests/test_unbounded_int.rs`)

Solved elsewhere: **Python**: `2**63, 2**64, abs(-2**63)` → `9223372036854775808 18446744073709551616 9223372036854775808`: `int` is arbitrary precision (small ints fast path, bignum on demand). **Haskell** `2^63 :: Integer` → `9223372036854775808` (and `Integer` is the default type).
**Swift**: `Int.max + 1` → traps at runtime (verified: process aborts), wrapping only via the explicit `&+` → `-9223372036854775808`, and `addingReportingOverflow`.
**Rust**: `i64::MAX.checked_add(1)` → `None`, `overflowing_add` → `(-9223372036854775808, true)`: the operation names the policy (`checked_/wrapping_/saturating_/overflowing_`).
Adopt in Warp: Python/Haskell semantics (Int is ℤ, i64 is an inferred representation that promotes to bignum on overflow, e.g. via an overflow check on the fast path as in [wiki/int60.md](wiki/int60.md)), plus Swift-style explicit wrapping operators or an `@wrap` / `@i64` representation annotation for code that wants modular arithmetic.

### Big literals
**JS**: `100000000000000000000` → *1e+20* (a double); Warp before fcbd300b: a string of NUL bytes.  
Warp: `100000000000000000000` → `100000000000000000000`, round-trips exactly.

Solved elsewhere: **Rust**: `let _x: i64 = 100000000000000000000;` → `error: literal out of range for 'i64'` (deny-by-default lint).
**Go**: `var x int64 = 100000000000000000000` → `cannot use 100000000000000000000 (untyped int constant) as int64 value in variable declaration (overflows)`, with the exact column.
**JS**: `100000000000000000000n` → `100000000000000000000n`; **Python** `100000000000000000000` → the exact int.
Adopt in Warp: until bignums exist, any integer literal outside i64 is a compile error with the span (Go/Rust); once Int is unbounded, the literal is simply a bignum (Python). Never a NUL string.

### Proofs about unbounded integers, run on wrapping ones
**Lean/Coq/Dafny exports that model `int` as ℤ**: `law square(x) >= 0` → *Proved*, while the program returns `square(3037000500)` → `-9223372036709301616`.  
Warp before 1507a3b7 had exactly this bug: the Lean export used unbounded `Int` and reported the law as Proved.
Now Int exports as `BitVec 64` with signed order (`BitVec.slt`/`sle`) and `srem`; Warp `/` is not exported (it yields a Float);
`bv_decide` counterexamples become Violated, and property tests start with `i64::MIN`, `i64::MAX` and ±3037000500.
Lesson: a proof counts only when the Lean model matches the backend's machine semantics. Unbounded Int is tracked in `todo.md`,
details in `notes/laws.md`. (`tests/test_law.rs`)
Since fcbd300b the runtime Int is unbounded again, so the `BitVec 64` model is now the wrong one: see NOT YET → Proof model lags the runtime.

### Date guessing in data
**Excel**: typing the gene name `SEPT2` → *2-Sep*  
Warp: `SEPT2` → symbol `SEPT2`; no date or unit guessing when reading data.

Solved elsewhere: **TOML 1.0**: `d = 2001-12-14` → `date(2001,12,14)`, while `gene = "SEPT2"` → `'SEPT2'`. Dates are a separate literal grammar (RFC 3339 only) and are never guessed from strings.
**pandas**: `read_csv(..., dtype=str)` → `{'gene':'SEPT2','zip':'01234'}`. Date parsing is off unless `parse_dates=` is passed. Without `dtype`, the zip still turns into `1234`.
Counter-example worth citing: **JS**: `new Date("SEPT 2")` → *Sun Sep 02 2001* (V8 fills in the year 2001 on its own); only ISO `2001-12-14` is specified.
Adopt in Warp: keep today's rule (`SEPT2` stays a symbol). If date literals are added, accept only an unambiguous RFC 3339 / ISO 8601 form, as TOML does, and parse anything else as a date only when a schema expects one.

### Memory-safety classics: use-after-free, double free, dangling pointers
**C, C++**: `free(p); p->x` → *undefined behaviour*  
Warp: by construction. Nodes are WASM GC structs and the surface language has no pointers, no `free`, no pointer
arithmetic; the WASM sandbox traps any access outside linear memory. (Bounds inside a Warp list are NOT YET, see below.)

Solved elsewhere: **Rust**: `let v=vec![1]; drop(v); v` → *compile error E0382 borrow of moved value* (verified) — ownership + borrow checker, no GC.
**Java / Go / JS / WASM GC**: tracing GC, no `free`, no pointer arithmetic, bounds-checked arrays. **Swift**: ARC + exclusivity checks, `Unsafe*Pointer` quarantined by name.
Adopt in Warp: already safe by construction via WASM GC; keep any linear-memory or FFI access behind the `Unsafe` effect (DESIGN.md effect set) so it is visible in signatures, and make ownership-pass optimisations (stack/unique allocation) never able to produce a dangling reference: fall back to GC when escape analysis is unsure.

### Data races
**C, C++, Go, Java**: two threads incrementing a shared counter → *lost updates*  
Warp: currently vacuous: generated modules are single threaded and share no memory. The plan keeps it that way: parallelism
is only inferred for code proved pure ([DESIGN.md → Cautions](DESIGN.md#cautions)).

Solved elsewhere: **Rust**: `let mut n=0; thread::spawn(|| n+=1); n+=1;` → *compile error E0373 / E0503* (verified) — `Send`/`Sync` traits plus borrow rules: shared XOR mutable across threads.
**Swift 6 strict concurrency**: actors and `Sendable` checking make cross-actor mutable sharing a compile error. **Erlang / Elixir, Pony**: share-nothing processes with message passing (Pony's reference capabilities prove race freedom statically).
Adopt in Warp: stay share-nothing: parallelism only over values proved pure (DESIGN.md Cautions), threads/components communicate by copying or moving values (WASM components already share no memory), and a future shared-memory mode requires an explicit `Sendable`-like capability checked by the ownership pass.

# NOT YET
...

Footguns Warp still has (verified with the probes above: the Warp answer shown is today's output), or where the fix
is designed but not implemented. Each entry names the intended resolution. Entries marked 🐞 are plain bugs, not design questions.

## Numbers

### Decimal fractions
**Python, JS, Java, C, …**: `0.1 + 0.2 == 0.3` → *false* (`0.30000000000000004`)  
Warp today: `0.1+0.2==0.3` → `0`, `1/3*3==1` → `0`, `3 == 3.0000000000000001` → `1`, and `.1` is a parse error.  
Intended: literals are exact rationals, the existing `Quotient` in `src/extensions/numbers.rs` is the starting point;
`@f64` opts into IEEE ([DESIGN.md → Exact numbers by default](DESIGN.md#exact-numbers-by-default)). (`test_exact_decimal_arithmetic`)

Solved elsewhere: **Haskell**: `1%10 + 2%10 == (3%10 :: Rational)` → `True`; also `0.1 + 0.2 == (0.3 :: Rational)` → `True`: decimal literals are overloaded (`fromRational`), so at type `Rational` they are exact.
**Go**: `0.1+0.2 == 0.3` → `true`: untyped constants are evaluated at arbitrary precision at compile time, and only rounded when they get a concrete type.
**Julia**: `1//10 + 2//10 == 3//10` → `true` (`Rational{Int}`; `//` builds exact rationals). **Python** `Decimal("0.1")+Decimal("0.2")==Decimal("0.3")` → `True`, but only with an opt-in type.
Adopt in Warp: the Haskell/Go model: a literal like `0.1` means the exact value 1/10 and representation is chosen late (as Go does for constants, but for all values); `@f64` is the only place rounding enters. `.1` should parse like `0.1`.

### 🐞 Sum of quotients truncated
Warp today: `1/4+1/4` → `0` while `1/4+1/4 == 0.5` → `1` and `1/3` → `0.333…`. The value is right inside the program,
but the result type of the sum is inferred as Int and truncated on return. (`test_sum_of_quotients_is_not_truncated`)

Solved elsewhere: **Python**: `Fraction(1,4)+Fraction(1,4)` → `1/2`; **Julia**: `1//4+1//4` → `1//2`; **Haskell** `1%4 + 1%4` → `1 % 2`. The result type of `+` is computed from the operand types (a numeric tower/promotion rule: `Rational + Rational → Rational`), never from a default.
Adopt in Warp: result types of arithmetic come from a promotion table over the operand types (Int ⊂ Rational ⊂ Real), checked in elaboration; a function's return type is never defaulted to Int ([DESIGN.md](DESIGN.md) "unknown names and unsolved overloads are errors, never `Symbol` or `Int` defaults").

### Proof model lags the runtime
The mirror image of the solved "Proofs about unbounded integers, run on wrapping ones": the Lean export still models Int as wrapping `BitVec 64`,
but since fcbd300b Warp Int is unbounded. `warp verify` on `square(x) := x*x⏎law square(x) >= 0` →
*FAILED lean counterexample x=-4611686018427388111*, while the program computes `square(-4611686018427388111) > 0` → `1`.  
Intended: export Int as Lean's `Int` again (and `as i64` values as `BitVec 64`), so the proof model follows the representation. (`test_proof_model_matches_unbounded_int`)

### Scientific notation and digit separators
**Python/JS/Rust**: `1e3` → `1000.0`, `1_000_000` → `1000000`  
Warp today: `1e3` → the list `1 e3`, `1_000_000` → the list `1 _000_000`: silently parsed as something else.
Intended: both are number literals (`test_scientific_notation`).

Solved elsewhere: **Python/Rust/Julia/JS/Go**: `1_000_000` → `1000000`, `1e3` → `1000.0` (Python, Julia) / `1000` (JS, Go, Rust prints `1000`). Separators are lexical: `_` between digits is ignored by the tokenizer, and a digit sequence followed by `e[+-]?digits` is one token.
**Python**: `int("1_000")` → `1000`: the same grammar is used for parsing text at runtime.
**Go**: `1e3` is an *untyped* constant usable as an int (`const n int = 1e3; fmt.Println(n)` → `1000`), i.e. the exponent does not force float.
Adopt in Warp: lex both in the number token (the tokenizer, not the list parser); in exact mode `1e3` is the integer 1000 and `1.5e-3` the rational 3/2000 (Go's untyped constants), so exponent notation never implies a float. `1e3` staying the list `1 e3` should become at least a warning.

### NaN and infinity
**IEEE 754 everywhere**: `NaN == NaN` → *false*, `1/0` → *Infinity*, `sqrt(-1)` → *NaN*, and NaN poisons all later math quietly.  
Warp today: `x=0.0/0.0; x==x` → `0`, `1/0` → `inf`, `sqrt(-1)` → `NaN`; `nan` is not even a name.  
Intended: exact numbers make `1/0` an error value (or `∞` of the extended reals, [wiki/int60.md](wiki/int60.md) reserves bits for
±∞, NaN and overflow); `√-1` can be `i` since `Complex` exists in `src/extensions/numbers.rs`. NaN only under `@f64`, with a law-visible warning.

Solved elsewhere: **Python**: `1/0` → `ZeroDivisionError: division by zero`, `math.sqrt(-1)` → `ValueError`, `cmath.sqrt(-1)` → `1j` (the complex answer is a separate, explicit module).
**JS BigInt / Python Fraction**: `1n/0n` → `RangeError: Division by zero`: exact types have no NaN/Inf, so division by zero is an error.
**Julia**: `isequal(NaN,NaN)` → `true` vs `NaN==NaN` → `false`; **JS** `Object.is(NaN,NaN)` → `true`, `[NaN].includes(NaN)` → `true` but `indexOf` → `-1`: a second, reflexive "same value" relation used for collections and hashing.
Adopt in Warp: exact numbers make `1/0` an `Error` value (Python/BigInt) and `sqrt(-1)` a `Complex` only when the result type allows it (as `cmath`); under `@f64` keep IEEE `==` but use a reflexive total-order equality (Julia `isequal`, IEEE 754 `totalOrder`) for `is`, dict keys, sorting and `law` checks.

### Rounding mode
**Python 3, .NET** `round(2.5)` → *2* surprises users of **JS/Excel** (`3`) and vice versa.  
Warp today: `round(2.5)` → `2`, `round(0.5)` → `0` (banker's rounding).  
Intended: keep the IEEE default but name it (`round half even`) and offer `round half up`; document it in the signature.

Solved elsewhere: **Julia**: `round(2.5)` → `2.0`, `round(2.5, RoundNearestTiesAway)` → `3.0`, `round(2.5, RoundUp)` → `3.0`: the mode is a named argument with a documented default.
**Rust**: `(2.5f64).round()` → `3`, `(2.5f64).round_ties_even()` → `2`: two differently named functions, no hidden default.
**Python Decimal**: `Decimal("2.5").quantize(Decimal("1"), rounding=ROUND_HALF_UP)` → `3`; `round(2.5)` → `2`. **Swift** `(2.5).rounded()` → `3.0`, `.rounded(.toNearestOrEven)` → `2.0`.
Adopt in Warp: `round(x)` plus a named mode argument (`round x half:even`, `half:up`, `half:away`, `down`, `up`), with the default spelled in the signature; for exact rationals the rounding is exact (no double-rounding of `2.675`-style decimals).

### Negative modulo
**C, JS, Java**: `-5 % 3` → *-2*, **Python**: *1*, both surprise the other camp.  
Warp today: `-5 % 3` → `-2`.  
Intended: `%` is the mathematical modulo (sign of the divisor, as in Python), `rem` the truncating remainder; both named.

Solved elsewhere: **Haskell**: `(-5) \`mod\` 3` → `1`, `(-5) \`rem\` 3` → `-2`; **Julia** `mod(-5,3)` → `1`, `rem(-5,3)` → `-2`: two named operators, one per convention.
**Rust**: `(-5i64).rem_euclid(3)` → `1` while `-5 % 3` → `-2`. **Python**: `-5 % 3` → `1`, `divmod(-5,3)` → `(-2, 1)` and `math.fmod(-5,3)` → `-2.0`, with the law `a == (a//b)*b + a%b` holding for floored division.
Adopt in Warp: Haskell/Julia naming, `%`/`mod` floored (sign of divisor) and `rem` truncating, and state the division law pairwise (`a == b*floor_div(a,b) + a%b`) as a `law` in the standard library so both pairs are checked.

### Booleans are integers
**Python, C, JS**: `True + True` → *2*  
Warp today: `true + true` → `2`, `false == 0` → `1` (booleans are encoded as Int 1/0).  
Intended: a distinct `bool` kind in semantic IR; arithmetic on booleans is a type error unless explicitly converted.

Solved elsewhere: **Haskell**: `True + True` → `No instance for 'Num Bool'`; **Rust**: `true + true` → `error[E0369]: cannot add 'bool' to 'bool'`; conversion is explicit (`fromEnum True`, `true as i64`).
**JS BigInt**: `1n + true` → `TypeError` (the newer numeric type refused the old coercion). Contrast **Python**: `True+True` → `2`, `isinstance(True,int)` → `True`; **Julia** `true+true` → `2` (Bool <: Integer).
Adopt in Warp: `Bool` is its own kind in semantic IR (it may still be encoded as i32 0/1 in WASM); arithmetic on it is a type error, with explicit `int(b)` / counting via `count(xs, pred)`.

## Implicit conversions

### String + number
**JavaScript**: `"5" + 3` → *"53"*, `"5" * 3` → *15*  
Warp today, worse: `"5"+3` → `56`, `"5"*3` → `159`, `"a"+1` → `98`, `3 + "4"` → `55`: one-character strings are
converted to their code point (C's `'5' + 3`).  
Intended ([DESIGN.md → Dangerous implicitness](DESIGN.md#dangerous-implicitness)): no silent coercion. `"5"+3` is a type error
with a fix-it (`"5" + str 3` or `int "5" + 3`); codepoint arithmetic only on values typed `char`.

Solved elsewhere: **Rust**: `"5" + 3` → compile error E0369 `cannot add {integer} to &str` ✓ — `Add` is only implemented for matching types, no coercion trait exists. **Python**: `"5"+3` → `TypeError: can only concatenate str (not "int") to str` ✓, while `f"{5}{3}"` → `"53"` ✓ — conversion is explicit, interpolation is the sanctioned route. **Julia**: `"5"*3` → `MethodError` ✓ — multiple dispatch has no `(String, Int)` method.
Adopt in Warp: `+` dispatches on resolved semantic types in elaboration; `Text + Int` has no method and yields a diagnostic with the two fix-its (`"5" + str 3`, `int "5" + 3`); codepoint arithmetic exists only for the `char` type, so `"5"+3 → 56` disappears because a one-char string literal is `Text`, not `char`.

### Parsing numbers from text
**C `atoi`, PHP**: `atoi("12a")` → *12*, `(int)"abc"` → *0*  
Warp today: `int("12a")` → `0`.  
Intended: `int "12a"` returns an error value (`Result`, [DESIGN.md → Effects](DESIGN.md#effects)), never a plausible number.

Solved elsewhere: **Rust**: `"12a".parse::<i64>()` → `Err(ParseIntError { kind: InvalidDigit })` ✓ — returns `Result`, the whole string must match. **Swift**: `Int("12a")` → `nil` ✓ — failable initializer returns `Optional`. **Go**: `strconv.Atoi("12a")` → `0, invalid syntax` ✓ — error value alongside, lint-enforced checking. (Counter-example: JS `parseInt("12a")` → `12` ✓, prefix parsing.) Also **Haskell** `readMaybe "12a" :: Maybe Int` → `Nothing` ✓.
Adopt in Warp: `int "12a"` returns `Result Int ParseError` (DESIGN.md → Effects: `Error` as `Result`), requiring the full string to match; no transparent unwrapping, so using it as a number without handling the error is a type error. A prefix-parsing variant, if ever wanted, must be named as such (`int_prefix`).

### Type annotations not enforced loudly
Warp today: `x:int=5;x="five";x` → compiler panic `Cannot extract numeric value from 'five'` (rejected, but as a crash);
`const x=5;x=6;x` → `6` (`const` is ignored).  
Intended: a type/constness diagnostic with span and fix-it; `const` and `::=` enforce single assignment.

Solved elsewhere: **Swift**: `let x = 5; x = 6` → `error: cannot assign to value: 'x' is a 'let' constant` ✓ — constness is part of the binding, checked before codegen. **Julia**: `x::Int = 5; x = "five"` → `MethodError` (convert String→Int) ✓ — typed globals insert a checked `convert` on every assignment. **Rust/TypeScript**: `let x: i64 = 5; x = "five"` → `mismatched types` with span and `help:` fix-it — diagnostics carry spans and suggestions.
Adopt in Warp: bindings in semantic IR carry `{type, mutability}`; elaboration checks every assignment against them and emits a spanned diagnostic (with fix-it) instead of reaching the emitter; `const` and `::=` produce immutable bindings, so reassignment is rejected rather than ignored.

### Lists and arithmetic
**Python**: `[1,2,3]*2` → *[1,2,3,1,2,3]*; **NumPy**: *[2,4,6]*; **JS**: `[1,2]+[3]` → *"1,23"*  
Warp today: `[1 2]+[3]` → `5`, `[1 2 3]*2` → `6` (the list is summed first).  
Intended: `+` on lists is concatenation (as `tests/test_lists.rs` already expects); element-wise lifting only through
a law-governed rule ([DESIGN.md → Dangerous implicitness](DESIGN.md#dangerous-implicitness), [wiki/broadcasting.md](wiki/broadcasting.md)).

Solved elsewhere: **Julia**: `[1,2,3] .* 2` → `[2,4,6]` ✓, `[1,2,3] + 1` → `MethodError` ✓, `vcat([1,2],[3])` → `[1,2,3]` ✓ — broadcasting is a separate, explicit dot syntax; plain operators keep their algebraic meaning. **APL/J/NumPy** broadcast implicitly (`[1,2,3]*2` → `[2,4,6]`) but under fixed shape rules; **Python** `[1,2]+[3]` → `[1,2,3]` ✓ (concatenation) yet `[1,2,3]*2` → repetition ✓, i.e. `*` has an unrelated meaning.
Adopt in Warp: `+` on lists is concatenation (the list monoid, lawful: associative with `[]` as identity), `list * number` is a type error; element-wise arithmetic uses an explicit lifting form (Julia-style `.+`/`.*` or a `map`), which is exactly the "type-directed, law-governed lifting rule" DESIGN.md → Dangerous implicitness asks for. Never sum a list implicitly.

## Equality and identity

### 🐞 String comparison
**Java**: `new String("abc") == "abc"` → *false*; **Python**: `a is b` works for `256` but not `257`  
Warp today: `"abc"=="abc"` → compiler panic `Cannot extract numeric value from 'abc'`; `"abc" is "abc"` stays unevaluated;
`0==""` and `null==false` panic.  
Intended ([wiki/equality.md](wiki/equality.md)): `==` and `is` compare values structurally, there is no identity operator in
the surface language. (`test_string_equality_is_by_value`)

Solved elsewhere: **Rust**: `String::from("abc") == "abc"` → `true` ✓ — `==` is `PartialEq`, always by value; identity needs explicit `std::ptr::eq`. **Swift**: `==` is value equality for `String`, identity `===` exists only for class instances. **JavaScript**: `Object.is(NaN,NaN)` → `true` ✓ separates SameValue from `===`, showing the cost of having several equalities.
Adopt in Warp: `==` and `is` are structural value equality for all data (Text compared by content), elaborated per type rather than via "extract numeric value"; no user-visible identity operator. Mixed-type comparisons (`0==""`, `null==false`) are type errors, not panics and not coercions.

### Unicode normalization
**Almost every language**: `"é" == "é"` (NFC U+00E9 vs NFD e+U+0301) → *false*  
Warp today: identical encodings → `1`, NFC vs NFD → compiler panic.  
Intended: text is normalized to NFC when parsed, so equal-looking strings are equal. (`test_unicode_normalization`)

Solved elsewhere: **Swift**: `"\u{e9}" == "e\u{301}"` → `true` ✓ — `String ==` uses Unicode canonical equivalence on grapheme clusters. **Python/JS** make it explicit: `unicodedata.normalize("NFC", s)` ✓ / `s.normalize("NFC")` ✓ (plain `==` → `false` ✓). Rust `==` → `false` ✓ (bytes; `unicode-normalization` crate needed).
Adopt in Warp: normalize text literals and text read by the parser to NFC once (so storage is canonical and `==` stays a cheap byte compare); text entering at runtime from IO is normalized at the boundary, with an explicit `bytes`/raw type for when exact code points must be preserved.

### Duplicate keys
**JSON (RFC 8259 leaves it open), JS, Python `json`**: `{"a":1,"a":2}` → *silently {"a":2}*  
Warp today: `{a:1 a:2}` is accepted without complaint (field access on objects does not evaluate yet: `x={a:1 b:2};x.a` → unevaluated).  
Intended: duplicate keys are a parse diagnostic in data, an explicit override in code.

Solved elsewhere: **Go**: `map[string]int{"a":1,"a":2}` → compile error `duplicate key "a" in map literal` ✓. **TOML 1.0**: `a=1\na=2` → `TOMLDecodeError: Cannot overwrite a value` ✓ — the spec forbids redefinition. **Python json** silently keeps the last (`{'a': 2}` ✓) but `object_pairs_hook` can reject it ✓; **Nix** `a // { a = 2; }` / JS `{...o, a:2}` show the explicit-override form.
Adopt in Warp: a literal with a repeated key is a parse diagnostic in data (TOML/Go rule); overriding is spelled explicitly in code (an update/merge form like `o with {a:2}` or Nix `//`), so "last wins" is never accidental.

## Strings

### Bytes vs characters vs graphemes
**Python 2, C, Go, JS**: `len("👍🏽")` → *8* bytes (Go), *4* UTF-16 units (JS), *2* codepoints (Python 3); users mean *1*  
Warp today: `size "👍🏽"` → `8`, `'héllo'#2` → `'Ã'` (half of the UTF-8 `é`), though [wiki/indexing.md](wiki/indexing.md) promises `#` is character-safe; `'héllo'.length` is not evaluated.  
Intended: `#` and default iteration are by character (grapheme), `[]` by byte; the unit is part of the type
(`for byte in text`, [DESIGN.md → Cautions](DESIGN.md#cautions)). (`test_character_indexing_is_unicode_safe`)

Solved elsewhere: **Swift**: `"👍🏽".count` → `1` (and `.utf8.count` 8, `.utf16.count` 4, `.unicodeScalars.count` 2). The default `Character` is an extended grapheme cluster, the other units are separate named *views*, and `String.Index` is opaque, so `s[2]` does not compile.
**Rust**: `"👍🏽".len()` → `8` bytes, `.chars().count()` → `2`. Slicing at a non-char boundary panics (`is_char_boundary(1)` → false), it never returns half a code point. Graphemes need a crate (`unicode-segmentation`).
**JS (Intl.Segmenter)**: `[...new Intl.Segmenter().segment("👍🏽")].length` → `1`. It came late and is opt-in, while `.length` stays 4.
Adopt in Warp: use Swift's model. Text has named views `bytes`, `codepoints`, `graphemes`, and the default `#`, `for c in text` and `count` work on graphemes. Byte access is only possible through the `bytes` view (or `[]` if Warp keeps that split). An index that is not on a boundary is an error value, never a partial character like `'Ã'`.

## Truthiness and null

### Empty values
**Python/JS disagree**: `[]` is falsy in Python, truthy in JS; `if ("")` / `if ({})` differ too.  
Warp today: `if "" {1} else {2}` and `if [] {1} else {2}` → compiler panic.  
Intended: either the uniform rule of [wiki/truthiness.md](wiki/truthiness.md) (all empty values falsy) or, per
[DESIGN.md → Dangerous implicitness](DESIGN.md#dangerous-implicitness), truthiness only for `bool` and `Option`, with `empty`/`missing` as explicit tests. Needs a decision.

Solved elsewhere: **Swift** ✓: `let xs:[Int]=[]; if xs {}` → compile error `cannot convert value of type '[Int]' to expected condition type 'Bool'`; you write `if xs.isEmpty` — conditions must be `Bool` (or an optional binding `if let`). **Go** ✓: `if x {}` with `x := 1` → `non-boolean condition in if statement`. **Ruby** ✓: `!![]`, `!!""`, `!!0`, `!!"0"` → all `true` — only `nil` and `false` are falsy, one rule with no per-type exceptions (the uniform alternative if Warp keeps truthiness at all).
Adopt in Warp: conditions take `Bool` or `T?` only (DESIGN.md); `if xs` on a collection/text is a structured diagnostic with fix-it `if not xs.empty` / `if xs.count > 0`. If backwards compatibility with the wiki is wanted, make it a *normalization hint*: accept `if xs` in the friendly surface, but elaborate it explicitly to `not empty xs` in the semantic IR, so one rule (Python's) applies uniformly and is visible in the resolved view. Either way, drop "unbound `x` is falsy" from the wiki.

### `and`/`or` as ternary
**Python, Lua**: `cond and x or y` → *y* when `x` is falsy  
Warp today: `1 and 0 or 2` → `2` while `if 1 then 0 else 2` → `0` (documented in [wiki/truthiness.md](wiki/truthiness.md)).  
Intended: lint `a and b or c` with a fix-it to `if a then b else c`.

Solved elsewhere: **Python 2.5+** ✓: `0 if 1 else 2` → `0` (vs `1 and 0 or 2` → `2` ✓) — PEP 308 added a real conditional expression precisely because the idiom breaks on falsy `x`. **Rust/Swift** ✓: `if true {0} else {2}` → `0` — `if` is an expression and `&&`/`||` only accept and return `bool`, so the idiom cannot typecheck. **Lua** ✓ still has the bug: `true and false or "y"` → `y` (no fix; shows the cost).
Adopt in Warp: make `and`/`or` Bool-typed (return `Bool`, not an operand), which dissolves the footgun under the strict-condition rule above; until then, lint the shape `a and b or c` with fix-it `if a then b else c`. Value-selecting defaulting belongs to a dedicated `??` / `or else` on `T?` (see Null), not to `or`.

### Null
**Java, C#, JS**: `obj.field` on null → *NullPointerException* / *TypeError at runtime*  
Warp today: `x=ø; x+1` → compiler panic (loud, but not a diagnostic).  
Intended ([wiki/null.md](wiki/null.md)): typed null, `T?` optional types, flow-sensitive `if x {…}` narrowing; no member access on `T?` without a check.

Solved elsewhere: **Swift** ✓: `let p:P? = nil; p?.name ?? "anon"` → `anon`; `p + 1` on `Int?` → compile error `value of optional type 'Int?' must be unwrapped`. **Kotlin**: `val s:String? = null; s.length` → compile error; `if (s != null) s.length` compiles via flow-sensitive smart casts. **Rust** ✓: `None::<i32>.map_or(0,|x|x+1)` → `0` — no null at all, `Option<T>` is an ordinary enum with exhaustive `match`.
Adopt in Warp: exactly [wiki/null.md](wiki/null.md) + DESIGN.md's `Option(T)`: `T?` in the semantic type, Kotlin-style flow narrowing after `if x` / `if x != ø`, `?.` and `??` as the only implicit paths, and `x=ø; x+1` becoming a spanned diagnostic "`x` is `Int?`, unwrap with `x ?? 0` or check `if x`" instead of a panic. No transparent unwrapping (DESIGN.md).

## Syntax and precedence

### Unary minus and power
**Excel, bash**: `=-2^2` → *4*  
Warp today: `-2^2` → `4`. Mathematics and Python say `-4`. (`test_negative_power_precedence`)

Solved elsewhere: **Python / Julia / Haskell**: `-2**2` / `-2^2` → `-4` — `**`/`^` binds tighter than prefix minus, as in mathematics.
**JavaScript**: `-2**2` → *SyntaxError: Unary operator used immediately before exponentiation expression* — the ambiguous form is simply rejected; `(-2)**2` → `4`.
**Rust**: `-2i32.pow(2)` → `-4` — method call binds tighter than unary minus.
Adopt in Warp: give `^` (and `²`, `³`) higher precedence than prefix `-`, so `-2^2` → `-4` like Python/Julia; a literal `-2` token is still negation of `2^…`, never a signed base. Optionally emit a normalizer hint recommending `-(2^2)` / `(-2)^2` in the canonical view.

### `not` and bitwise operators vs comparison
**C**: `!1 == 2` → *0*, `3 & 4 == 4` → *1* (`&` binds weaker than `==`)  
Warp today: `not 1==2` → `0`, `3 & 4 == 4` → `1`, both C's answers. Python says `True` and `False`. Also `3 | 4` → `3` (`|` is pipe, not bitwise or).  
Intended: `not`, `&`, `|` bind weaker than comparisons; mixing bitwise and comparison without grouping is a diagnostic
([wiki/precedence.md](wiki/precedence.md)). (`test_logic_binds_weaker_than_comparison`)

Solved elsewhere: **Python**: `not 1==2` → `True`, `3 & 4 == 4` → `False` — `not` binds weaker than comparisons; `&`,`|`,`^` bind *tighter* than comparisons (fixing C's historical order).
**Go / Rust**: `3&4 == 4` → `false` — same fix: bitwise ops at multiplicative/additive level, above `==`.
**Carbon** *(not run)*: precedence is a *partial order*; `a & b == c` is a compile error demanding parentheses.
Adopt in Warp: `not`/`and`/`or` below comparisons (Python), and either Python's "bitwise above comparison" or Carbon's partial order (mixing bitwise with comparison without parentheses is a diagnostic with a fix-it). The partial order fits DESIGN.md's "never guess" best; `|` staying pipe means bitwise or needs its own spelling (`bitor`, `∨`?) anyway.

### Chained comparison
**C, JS**: `3 > 2 > 1` → *false* (`true > 1`)  
Warp today: `3>2>1` → `0` (`1<2<3` → `1` only by luck). Intended: mathematical chaining as in Python, `3>2>1` → `true`.

Solved elsewhere: **Python / Julia**: `3>2>1` → `True`, `1<3>2` → `True` — `a<b<c` desugars to `a<b and b<c` with `b` evaluated once.
**Rust**: `1 < 2 < 3` → *error: comparison operators cannot be chained; help: split the comparison into two* — refuses rather than guesses.
**Haskell**: `3 > 2 > 1` → *Precedence parsing error: cannot mix ‘>’ [infix 4] and ‘>’ [infix 4]* — comparisons are declared non-associative (`infix 4`).
Adopt in Warp: Python/Julia chaining for same-direction chains (`a<b<c`, `a≤b<c`, `a==b==c`) desugared in elaboration with the middle operand bound once; mixed-direction chains like `1<3>2` (legal but confusing in Python) are a diagnostic.

### Assignment in a condition
**C, JS**: `if (x = 2)` → *always true, x overwritten*  
Warp today: `x=1;if x=2 {3} else {4}` → `3`; even `if 1=2 {3} else {4}` → `3`.  
Intended ([wiki/Bad.md](wiki/Bad.md) "assignment, declaration, comparison"): `=` in a condition is comparison or a
diagnostic, never assignment; persisted resolution if ambiguous ([DESIGN.md → Content-addressed resolutions](DESIGN.md#content-addressed-resolutions)).

Solved elsewhere: **Python**: `if x = 2:` → *SyntaxError: invalid syntax. Maybe you meant '==' or ':=' instead of '='?* — `=` is a statement; binding inside an expression needs the distinct walrus `:=`.
**Swift**: `if i = 2 {}` → *error: use of '=' in a boolean context, did you mean '=='?* ; **Rust**: `if x = 2 {}` → *mismatched types, help: you might have meant to compare for equality* — assignment has type `()`/`Void`, never Bool.
**Kotlin** *(not run)*: "Assignments are not expressions, and only expressions are allowed in this context".
Adopt in Warp: assignment evaluates to unit, so in a Bool-expected position (condition of `if`/`while`) `=` is elaborated as comparison (as [wiki/Bad.md](wiki/Bad.md) wants) *or* rejected with a fix-it — pick one and persist it; `if 1=2` must never be truthy. Bidirectional typing (Bool expected) makes this an elaboration rule, not a parser heuristic.

### 🐞 Increment
**C**: `i++ + i++` → *undefined behaviour*  
Warp today: `x=1;x++;x` → `1` (the increment is lost), `++i` is a parse error; [wiki/equality.md](wiki/equality.md) says
`++` is immediate, so `i++` and `++i` are the same. (`test_increment_changes_variable`)

Solved elsewhere: **Swift** (removed ++ in Swift 3, SE-0004): `i++` → *error: cannot find operator '++' in scope; did you mean '+= 1'?*
**Rust**: `i++` → *error: Rust has no postfix increment operator; help: use `+= 1` instead*.
**Go**: `j := i++` → *syntax error: unexpected ++* — `i++` exists but only as a statement, never an expression, so `i++ + i++` cannot be written.
Anti-example **Python**: `i=1; ++i` → `1` silently (`+(+i)`), a footgun of its own.
Adopt in Warp: Go's rule — `x++`/`++x` is a statement meaning `x += 1` (both spellings identical, per wiki/equality.md), its value is unit so it can't be used inside an expression; `++` applied to a non-place is a diagnostic, never `+(+x)`. First fix the bug that `x=1;x++;x` → `1`.

### Braceless calls
Warp today: `f := it*10; 1 + f 3` → `3` 🐞 (should be `31`), and the recursive case from [wiki/Bad.md](wiki/Bad.md)
`fib := it<2 ? it : fib it-1 + fib it-2` fails with `Undefined variable: it`. (`test_braceless_call_as_operand`)

Solved elsewhere: **Ruby**: `f 3-1` → `20` (argument is the whole expression) and `1 + f 3` → *SyntaxError* — a braceless call is only allowed where it is unambiguous, otherwise loud; `f -1` → warning *ambiguous first argument*.
**Haskell**: `f 3-1` → `29`, `1 + f 3` → `31` — the opposite, but *one* fixed rule (application binds tightest) plus `$` for "rest of line": `f $ 3-1` → `20`.
**Julia**: `2x` → `6` — juxtaposition restricted to numeric-literal coefficients, where it cannot mislead.
Adopt in Warp: keep Warp's chosen rule (braceless call takes the whole remaining argument expression, like Ruby/`$`) and apply it uniformly also in operand position, so `1 + f 3` → `1 + f(3)` → `31`; `fib it-1 + fib it-2` then needs the known arity of `fib` (1) to stop the argument at the next `+`-level call — resolve that with the declared signature in elaboration, and where arity is unknown emit a diagnostic instead of a parse.

## Mutation and scope

### Aliasing
**Python, JS, Java**: `a=[1]; b=a; b[0]=9; a` → *[9]*  
Warp today: `a=(1 2);b=a;b#1=9;a#1` → `9`, and even strings: `x="ab";y=x;y#1="z";x` → `'zb'`.  
Intended ([DESIGN.md → Ownership](DESIGN.md#ownership-and-resource-inference)): value semantics; mutation needs a unique place,
otherwise copy-on-write. (`test_mutation_through_alias_is_not_visible`)

Solved elsewhere: **Swift**: `var a=[1]; var b=a; b[0]=9; print(a)` → `[1]` (verified) — Array/String/Dictionary are value types with copy-on-write: the buffer is shared until a write, `isKnownUniquelyReferenced` decides whether to copy.
**Rust**: `let mut b = a; b[0]=9; a` → *compile error E0382, use of moved value* — assignment moves ownership; sharing needs an explicit `.clone()` or `&`/`Rc`.
**Clojure / Immutable.js**: `(let [a [1] b (assoc a 0 9)] a)` → `[1]` — persistent data structures with structural sharing, mutation returns a new value.
Adopt in Warp: Swift's model is exactly DESIGN.md's ownership rule 5: lists/text/records are values, `b=a` shares the GC ref, and a write through `b#1=` copies unless the ownership pass proves `b` unique (then mutate in place). This keeps `=` cheap and the observable semantics identical whichever plan is chosen.

### Mutable default arguments
**Python**: `def f(a=[]): a.append(1); return a` → second call returns *[1, 1]*  
Warp today: default arguments work for numbers (`tests/test_functions.rs`), `def f(a=()): a.add(1); f(); f()` panics. Unverified.  
Intended: defaults are values evaluated per call; with value semantics the footgun cannot occur.

Solved elsewhere: **Swift**: `func f(_ xs:[Int]=[]) {var ys=xs; ys.append(1); return ys}; f(); f()` → `[1] [1]` (verified) — default expressions are re-evaluated at every call site, and arrays are values anyway.
**Kotlin / C++ / JS**: `fun f(a: MutableList<Int> = mutableListOf())` → fresh list per call — defaults are call-site expressions, not objects stored on the function.
(**Python** workaround `def f(a=None): a = [] if a is None else a` → `[1] [1]` (verified) shows the cost of the wrong default.)
Adopt in Warp: define a default as an expression elaborated at the call site (inserted into the argument list in semantic IR), never a value stored once with the function; with value semantics the footgun is doubly impossible. The current panic on `def f(a=())` must become a type/elaboration error with span if it is not fixed.

### Closures capturing loop variables
**Python, JS `var`, Go < 1.22**: `[lambda: i for i in range(3)]` → all return *2*  
Warp today: functions do not see outer variables at all: `x=1;f(y):=x+y;f(1)` → `Undefined variable: x`. Unverified.  
Intended: closures capture values (immutable bindings), so each iteration's `i` is its own.

Solved elsewhere: **Go ≥ 1.22**: `for i:=0;i<3;i++ { fs=append(fs, func()int{return i}) }` → `0 1 2` (verified) — each iteration gets a fresh `i` (loopvar semantics change).
**JS `let`**: `for (let i=0;i<3;i++) fs.push(()=>i)` → `[0,1,2]` (verified) — per-iteration binding, unlike `var`.
**Rust / Swift**: `(0..3).map(|i| move || i)` → `[0,1,2]` (verified) — loop variables are immutable per-iteration bindings; `move` captures by value, capturing a mutable by reference that outlives it is a borrow error.
Adopt in Warp: loop variables are fresh immutable bindings per iteration and closures capture by value (a reassignable outer local captured by a closure is either copied at capture time or rejected with a diagnostic, never shared by reference silently). This fits the "immutable local bindings" core and makes closures pure by default.

## Bounds

### Index out of range / negative index
**C**: `a[3]` on 3 elements → *undefined behaviour*; **JS**: *undefined*; **Python**: `a[-1]` wraps silently  
Warp today: `x=[1 2 3]; x[3]` → the unevaluated program text `x=[1 2 3]; x#4`; `x#0` → `1`, `x[-1]` → `1`.  
Intended: out of range is an error value with span; `#0` is an error (1-based), `#-1` means last only if spelled so. (`test_index_out_of_bounds_is_an_error`)

Solved elsewhere: **Rust**: `vec![1,2,3].get(3)` → `None`, `.last()` → `Some(3)` (verified); `a[3]` panics with the index and length — checked indexing is the default, the unchecked form is `unsafe get_unchecked`.
**Go**: `a[3]` → `runtime error: index out of range [3] with length 3` (verified) — always bounds checked, message carries index and length.
**JS `Array.at`**: `[1,2,3].at(-1)` → `3` (verified) — negative indexing only through an explicit method; plain `a[-1]` / `a[3]` stay `undefined` (the footgun). **Swift**: `arr.indices.contains(3)` / `arr.last` → optional, `arr[3]` traps.
Adopt in Warp: two spellings, both checked: `x#i` traps/returns an error value carrying span, index and length (1-based, so `#0` is always an error), and a total variant (e.g. `x#?i` or `x.get i`) returns `Option<T>` for code that wants to branch. Negative counting only via an explicit `last`/`from end` form, never by wrap-around of `#-1`.

## Data formats

### The Norway problem
**YAML 1.1**: `country: NO` → *country: false*  
Warp today: `country: NO` → `country:0`, `yes` → `1`.  
Intended: only `true`/`false` (and `✔`/`✖`?) are boolean literals in data; `NO`, `no`, `yes`, `on` stay symbols. Needs a decision since `yes` is a documented alias.

Solved elsewhere: **TOML 1.0**: `country = NO` → *parse error*, so it has to be written `"NO"`. The only booleans are lowercase `true`/`false`, bare words are not values, and strings must be quoted.
**YAML 1.2 core schema / StrictYAML**: 1.2 cut the boolean set down to `true|false` (in any case); StrictYAML treats every scalar as a string until a schema says otherwise. (PyYAML and Ruby Psych still use 1.1: `NO` → `false`, verified.)
**JSON**: `true`/`false` only; a bare `NO` is a syntax error.
Adopt in Warp: data literals should accept only `true`/`false` (plus `✔`/`✖` if they are wanted) as booleans. `yes`/`no`/`on`/`off` should be symbols in data. In code, a symbol reaches `Bool` only through an expected-type coercion that the elaborator records explicitly. The value then depends on the schema, not on the spelling.

### Numbers that are not numbers
**YAML, Excel, CSV importers**: `version: 1.10` → *1.1*, `zip: 01234` → *1234*  
Warp today: `version: 1.10` → `version:1.1`, `zip: 01234` → `zip:1234`.  
Intended: serialization round-trips the literal (keep the source text in `Meta`), a leading zero or trailing zero after
the point keeps the value text-like unless a numeric type is expected.

Solved elsewhere: **TOML 1.0**: `zip = 01234` → *parse error*, because leading zeros are forbidden. That makes `"01234"` the only way to write it, so a leading zero can never be silently dropped.
**Python json**: `json.loads('{"v":1.10}', parse_float=Decimal)` → `Decimal('1.10')`. The decimal keeps its trailing zero because the parser takes a hook for number construction.
**JS (Node 26, JSON.parse source text access)**: `JSON.parse('{"v":1.10}', (k,v,ctx)=>ctx.source ?? v)` → `{v:'1.10'}`. The reviver sees the original lexeme; `JSON.rawJSON` does the same when serializing.
Adopt in Warp: every numeric literal in `Meta` keeps its source lexeme, so serialization round-trips `1.10` and `01234` byte for byte. When no numeric type is expected, a lexeme with a leading zero is a symbol/text with a diagnostic, as in TOML, not an octal or truncated number. This matches DESIGN.md's rule that `Node` preserves "data literals exactly enough for round-trip serialization".

### Data that executes
**Early JS `eval(json)`, YAML `!!python/object`, pickle**: loading data runs code  
Warp today: `WaspParser::parse` is data only; `eval` of an untrusted `.wasp` file runs it, but only with the capabilities
its imports declare (`tests/test_effects.rs::test_imports_follow_effects`).  
Intended: a CLI/data loading path that parses without evaluating, and evaluation of foreign data only with an empty capability set.

Solved elsewhere: **PyYAML**: `yaml.safe_load("!!python/object/apply:os.system ['echo pwned']")` → `ConstructorError`. The safe loader has no constructor for language-object tags, so a tag cannot produce code (plain `yaml.load` without `SafeLoader` used to run it).
**JSON / Rust serde**: `serde_json::from_str::<Config>(s)` produces only the declared type. The format has no tags that could name executable things, and the target type is fixed by the caller.
**Deno**: `deno run script.ts` with no `--allow-*` flags means the code cannot read files, use the network or read env. Capabilities are deny-by-default and granted per run.
Adopt in Warp: add a `warp read file.wasp` / `load(text)` entry point that stops at `Node` and cannot evaluate, and make it the default for data files. Evaluating foreign code should run as a WASM component with an empty import set, plus only the capabilities the caller grants explicitly (the WIT imports backstop in DESIGN.md → Effects as enforced capabilities).

## Errors

### Swallowed errors
**Go**: `v, _ := f()`; **Java**: `catch (Exception e) {}`; **JS**: unhandled promise rejection → *silent*  
Warp today: law and effect violations come back as `Error` values (verified above), but many failures are compiler panics
(`Cannot extract numeric value …`) or silent wrong values (`int("12a")` → `0`, `x[3]`).  
Intended: `Result<T, E>` as data, every diagnostic with span, resolved facts and fix-it ([DESIGN.md → The compiler is a query interface](DESIGN.md#the-compiler-is-a-query-interface)).

Solved elsewhere: **Rust** ✓: `#[must_use] fn g()->Result<..>; g();` → `unused Result that must be used` (deny-able to an error); `?` propagates in one character. **Swift** ✓: calling a `throws` function without marking it → compile error `call can throw but is not marked with 'try'`; `(try? f()) ?? -1` → `-1` is the *explicit* swallow, and `Int("12a")` → `nil` ✓ (not `0`). **Zig**: error unions must be handled (`try`, `catch`, or `catch unreachable`); discarding one is a compile error.
Adopt in Warp: `Result<T,E>` as data with Rust's "must use" as a hard error, `?`/`try` for propagation, and any discard spelled explicitly (`try? f()`, `f() or else default`) so it is greppable. Concretely: `int("12a")` must return `Int?`/`Result`, never `0`; `x[3]` out of range returns `T?` or a spanned error; compiler panics (`Cannot extract numeric value …`) become `Diagnostic`s with span + resolved facts + fix-it.

## Injection

### SQL and shell injection
**Every language with string building**: `"SELECT * FROM t WHERE name='" + name + "'"` with `name = "x' OR '1'='1"` → *all rows*  
Warp today: no SQL or shell API exists yet; string interpolation `` `${…}` `` produces plain text.  
Intended: interpolation into a query or command is typed (the target language is a type, arguments are parameters, not text); combined with effects, only modules with a `sql`/`process` capability can run them.

## Time

### Dates and time zones
**JS**: `new Date(2024, 1, 31)` → *March 2*; **Java**: `new Date().getYear()` → *124*; everyone: local time jumps at DST  
Warp today: no date/time type.  
Intended: distinct `instant`, `date`, `local time`, `zoned time`; no implicit current zone; months are 1-based; arithmetic on calendar units is explicit about overflow (`Jan 31 + 1 month`).

## Variance
**Java**: `Object[] a = new String[1]; a[0] = 1;` → *ArrayStoreException at runtime*  
Warp today: no generic/subtyping rules exist to be unsound. Intended: immutable collections may be covariant, mutable ones are invariant.

Solved elsewhere: **Kotlin / Scala / C#**: `List<out E>` read-only is covariant, `MutableList<E>` is invariant — declaration-site variance checked by the compiler, so Java's `Object[] a = new String[1]; a[0]=1` (verified: `ArrayStoreException` at runtime) cannot type check.
**Rust**: `&T` covariant, `&mut T` invariant, inferred from usage: pushing a short-lived `&str` into a `Vec<&'static str>` via `&mut` → `E0597 s does not live long enough` (verified).
**Java generics** (use-site `List<? extends Number>`) show the verbose alternative.
Adopt in Warp: infer variance from use like Rust instead of annotating it: immutable values (the default) are covariant, a parameter or place that is written through (a mutation effect on it) is invariant. Since collections are values with copy-on-write, most code never meets invariance at all.

# "Impossible"
... Really?

Footguns no language can remove in general, because the limit is mathematical or physical. Warp's answer is to make the
limit visible instead of pretending it is gone.

### Deciding termination and exact behaviour
**Halting problem, Rice's theorem**: no compiler can tell for every program whether it terminates, which effects it really performs or what it costs.  
Warp: effects are over-approximated (the union of everything reachable, `tests/test_effects.rs`), costs are declared in
signatures and checked by benchmarks, never derived in general ([DESIGN.md → Cautions](DESIGN.md#cautions)).

### Exact real numbers
**Richardson's theorem**: equality of real expressions built from `π`, `exp`, `sin`, … is undecidable, so `√2 * √2 == 2` cannot hold for every real computation.  
Warp: rationals are exact, algebraic numbers could be kept symbolic; beyond that the result is an approximation and its type says so.

Solved elsewhere: **Mathematica / SymPy**: `Sqrt[2]*Sqrt[2] == 2` → `True`, `sqrt(2)**2` → `2` *(not run)*: algebraic numbers are kept symbolic and simplified.
**Android calculator (Boehm's constructive reals, `CR`/`UnifiedReal`)**: displays `√2·√2` as exactly `2` by combining rationals × known irrationals with lazily evaluated arbitrary precision; equality is decided when it is provable and otherwise reported as "equal to N digits" *(not run)*.
**Haskell**: `0.1 + 0.2 == (0.3 :: Rational)` → `True`: the exact tower stops at ℚ, and `sqrt` is simply not defined on `Rational`, so the type system says where approximation begins.
Adopt in Warp: Boehm's approach: represent values as rational × (known symbolic factor) when possible, fall back to lazily refined constructive reals, and make `==` on such values three-valued (true / false / "equal to N digits"), so the type reports when a result is only approximate.

### Fast *and* lawful floats
**IEEE 754**: `(0.1+0.2)+0.3 == 0.1+(0.2+0.3)` → *false*; hardware floats are not associative.  
Warp today: `0`. Under `@f64` this stays true forever; the choice is exact numbers (slower) or floats whose laws are weaker. `law` makes the chosen trade-off checkable.

Solved elsewhere: **Python**: `(0.1+0.2)+0.3 == 0.1+(0.2+0.3)` → `False`, but `math.fsum([0.1,0.2,0.3]) == 0.6` → `True`: exactly-rounded summation (Shewchuk) recovers the lawful answer for the most common case.
**Julia** `sum` uses pairwise summation and `@fastmath` is a *local, explicit* opt-in to reassociation *(not run)*; **Rust** has no global fast-math flag at all: reassociation is only allowed through explicit intrinsics (`fadd_fast`, nightly) *(not run)*.
**Herbie** (tool) rewrites float expressions to more accurate equivalents automatically *(not run)*.
Adopt in Warp: keep IEEE semantics bit-exact under `@f64` (no global fast-math), make reductions like `sum` exactly rounded or compensated by default, and allow reassociation only through a scoped annotation (`@fastmath`) so `law` checks know which laws hold where.

### Function equality
`f == g` for arbitrary functions is undecidable.  
Warp: laws state the properties that matter and are tested or proved per function.

### "The" length of a string
There is no single right answer (bytes, UTF-16 units, codepoints, grapheme clusters), segmentation changes with Unicode versions,
and case mapping is locale dependent (Turkish `i` ↔ `İ`).  
Warp: make the unit explicit (`byte`, `char`, `grapheme`) instead of choosing silently.

Solved elsewhere: **Swift**: `s.count` / `s.utf8.count` / `s.utf16.count` / `s.unicodeScalars.count` → `1/8/4/2` for `"👍🏽"`. Every unit has a name, and only the grapheme count gets the short name.
**JS Intl / ICU**: `"i".toLocaleUpperCase("tr")` → `İ`, while `"i".toUpperCase()` → `I`. Locale-dependent case mapping takes the locale as an explicit argument; the default is locale-independent.
**Python**: `unicodedata.unidata_version` exposes the Unicode version, so the segmentation rules can at least be seen and pinned.
Adopt in Warp: a string has no `length`. It has `bytes.count`, `codepoints.count` and `graphemes.count` (`count` means graphemes), and case mapping takes an explicit locale or defaults to the invariant locale. The Unicode version used for segmentation should be recorded in the semantic artifact so results are reproducible across builds.

### Future civil time
What UTC instant is `2030-03-31 02:30 Europe/Berlin`? Time zone rules change by political decision after the code is written.  
Warp: store instants for past events and zone + local time for future ones; no type can know tomorrow's politics.

### Remote calls that look local
**Fallacies of distributed computing**: the network is not reliable.  
Warp: `fetch` carries the `IO` effect, so every caller sees it may fail or block; making it infallible is impossible.

### Guessing intent
`if fib 3 = 5` ([wiki/Bad.md](wiki/Bad.md)): no parser can know whether `=` means assignment, definition or comparison.  
Warp: never guess in the emitter; an ambiguity is resolved once (by the programmer or an agent) and the choice is persisted,
keyed by content hash ([DESIGN.md → Content-addressed resolutions](DESIGN.md#content-addressed-resolutions)). So: Really? The guess is impossible, the footgun is not.

Solved elsewhere: **Unison** *(not run)*: code is stored as a content-addressed AST; names are resolved once at `add`/`update` time and the hash, not the text, is what later builds consume — exactly "resolve once, persist".
**Python** (`:=` walrus) / **Pascal/Ada** (`:=` vs `=`): assignment, definition and comparison get *distinct tokens*, so no guess is needed.
**Rust/Swift/Python diagnostics**: `did you mean '=='?` — the compiler proposes, the programmer accepts (machine-applicable fix-it, `cargo fix`).
Adopt in Warp: keep the DESIGN.md plan (content-addressed resolution sidecar, Unison-style); canonical view uses distinct spellings (`:=` define, `=` assign, `==` compare) so an accepted resolution can be written back into source, and fix-its are machine-applicable so agents/`warp fix` persist the choice.

### Proofs about a model
A proof is only as good as its model of the runtime: a Lean proof over ℤ says nothing about wrapping i64, and nothing can prove the
compiler, the WASM engine and the hardware themselves correct from inside the program ("Reflections on Trusting Trust").  
Warp: the Lean export models Warp's actual integers (`BitVec 64`, see Solved); property tests and runtime assertions check the compiled program, not the model.

### Nondeterministic NaN bits
The WebAssembly spec allows NaN payload bits to differ between engines (and relaxed SIMD results to differ between CPUs).  
Warp: canonicalize NaNs where determinism matters, at a cost; exact numbers avoid NaN altogether.

### Timing side channels
No high-level language can guarantee constant-time execution on arbitrary hardware (Spectre, cache timing); WASM engines JIT differently.  
Warp: out of scope, crypto belongs in audited host functions with the `FFI` effect.