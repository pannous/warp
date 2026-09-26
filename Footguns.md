# Footguns

Footguns in other programming languages and how they are avoided in Warp 

# Solved

See [Solved elsewhere](Solved-elsewhere.md) for comparisons with other languages.
You may not be aware, but in other languages, you might stumble upon these questions with wrong answers. 
.1 + .2 == .3 ?  Warp : see below

How to read the entries: **offending language**: `one-liner` → *its wrong answer*; then what Warp does.
Every Warp answer below was produced by running the real compiler (`target/debug/warp eval …`, WASM GC round trip):
* `probes/footguns/footguns.sh` evaluates all cases in `probes/footguns/cases.warp`, output in `probes/footguns/results.txt`
* `tests/probe_footguns.rs` pins the Solved entries as `is!` tests; its `#[ignore = "next"]` tests are the NOT YET entries with a clear intended answer

Only entries whose Warp answer was verified are listed here; everything else is under # NOT YET.

### Integer division
**C, Java, Go, Python 2**: `7/2` → *3*  
Warp: `7/2` → `3.5`. `/` is always division; truncation must be asked for. (`test_division_is_not_truncating`)

### Octal literals
**C, Java, sloppy JS**: `010` → *8*  
Warp: `010` → `10`. A leading zero never switches the base; bases are explicit (`0x10` → `16`). (`test_leading_zero_is_not_octal`)

### Loose equality
**JavaScript, PHP**: `1 == "1"` → *true* (and `"0" == false` → *true*)  
Warp: `1=="1"` → `false`. No implicit string↔number coercion in comparisons. (`test_no_loose_equality`)

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

### Braceless call grabbing too little
**Ruby, Haskell-style juxtaposition** ([wiki/Bad.md](wiki/Bad.md) feared it): `fibonacci number-1` read as *(fibonacci number)-1* → infinite recursion  
Warp: `f := it*10; f 3-1` → `20`, the call takes the whole argument expression `f(3-1)`. (`test_braceless_call_takes_whole_argument`; but see NOT YET → Braceless calls)

### Parameter shadowing
**Many languages** accidentally read the outer variable when a parameter has the same name.  
Warp: `x=1;f(x):=x*2;f(5)+x` → `11`, the parameter shadows, the outer `x` is untouched. (`test_parameter_shadows_outer_variable`)

### "0" is falsy
**PHP, Perl**: `if ("0")` → *false*  
Warp: `if "0" {1} else {2}` → `1`; only the number `0` is falsy: `if 0 {1} else {2}` → `2`. (`test_zero_string_is_truthy`)

### Undefined variables
**JavaScript**: `a + 1` → *NaN*; `a = 1` in sloppy mode silently creates a global; **Perl/PHP**: *0*/warning  
Warp: `a+1` → compile error `Undefined variable: a`. (`test_undefined_variable_is_an_error`; the error is a panic, not yet a structured diagnostic)

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

### Big literals
**JS**: `100000000000000000000` → *1e+20* (a double); Warp before fcbd300b: a string of NUL bytes.  
Warp: `100000000000000000000` → `100000000000000000000`, round-trips exactly.

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

### Memory-safety classics: use-after-free, double free, dangling pointers
**C, C++**: `free(p); p->x` → *undefined behaviour*  
Warp: by construction. Nodes are WASM GC structs and the surface language has no pointers, no `free`, no pointer
arithmetic; the WASM sandbox traps any access outside linear memory. (Bounds inside a Warp list are NOT YET, see below.)

### Data races
**C, C++, Go, Java**: two threads incrementing a shared counter → *lost updates*  
Warp: currently vacuous: generated modules are single threaded and share no memory. The plan keeps it that way: parallelism
is only inferred for code proved pure ([DESIGN.md → Cautions](DESIGN.md#cautions)).

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

### 🐞 Sum of quotients truncated
Warp today: `1/4+1/4` → `0` while `1/4+1/4 == 0.5` → `1` and `1/3` → `0.333…`. The value is right inside the program,
but the result type of the sum is inferred as Int and truncated on return. (`test_sum_of_quotients_is_not_truncated`)

### Proof model lags the runtime
The mirror image of the solved "Proofs about unbounded integers, run on wrapping ones": the Lean export still models Int as wrapping `BitVec 64`,
but since fcbd300b Warp Int is unbounded. `warp verify` on `square(x) := x*x⏎law square(x) >= 0` →
*FAILED lean counterexample x=-4611686018427388111*, while the program computes `square(-4611686018427388111) > 0` → `1`.  
Intended: export Int as Lean's `Int` again (and `as i64` values as `BitVec 64`), so the proof model follows the representation. (`test_proof_model_matches_unbounded_int`)

### Scientific notation and digit separators
**Python/JS/Rust**: `1e3` → `1000.0`, `1_000_000` → `1000000`  
Warp today: `1e3` → the list `1 e3`, `1_000_000` → the list `1 _000_000`: silently parsed as something else.
Intended: both are number literals (`test_scientific_notation`).

### NaN and infinity
**IEEE 754 everywhere**: `NaN == NaN` → *false*, `1/0` → *Infinity*, `sqrt(-1)` → *NaN*, and NaN poisons all later math quietly.  
Warp today: `x=0.0/0.0; x==x` → `0`, `1/0` → `inf`, `sqrt(-1)` → `NaN`; `nan` is not even a name.  
Intended: exact numbers make `1/0` an error value (or `∞` of the extended reals, [wiki/int60.md](wiki/int60.md) reserves bits for
±∞, NaN and overflow); `√-1` can be `i` since `Complex` exists in `src/extensions/numbers.rs`. NaN only under `@f64`, with a law-visible warning.

### Rounding mode
**Python 3, .NET** `round(2.5)` → *2* surprises users of **JS/Excel** (`3`) and vice versa.  
Warp today: `round(2.5)` → `2`, `round(0.5)` → `0` (banker's rounding).  
Intended: keep the IEEE default but name it (`round half even`) and offer `round half up`; document it in the signature.

### Negative modulo
**C, JS, Java**: `-5 % 3` → *-2*, **Python**: *1*, both surprise the other camp.  
Warp today: `-5 % 3` → `-2`.  
Intended: `%` is the mathematical modulo (sign of the divisor, as in Python), `rem` the truncating remainder; both named.

### Booleans are integers
**Python, C, JS**: `True + True` → *2*  
Warp today: `true + true` → `2`, `false == 0` → `1` (booleans are encoded as Int 1/0).  
Intended: a distinct `bool` kind in semantic IR; arithmetic on booleans is a type error unless explicitly converted.

## Implicit conversions

### String + number
**JavaScript**: `"5" + 3` → *"53"*, `"5" * 3` → *15*  
Warp today, worse: `"5"+3` → `56`, `"5"*3` → `159`, `"a"+1` → `98`, `3 + "4"` → `55`: one-character strings are
converted to their code point (C's `'5' + 3`).  
Intended ([DESIGN.md → Dangerous implicitness](DESIGN.md#dangerous-implicitness)): no silent coercion. `"5"+3` is a type error
with a fix-it (`"5" + str 3` or `int "5" + 3`); codepoint arithmetic only on values typed `char`.

### Parsing numbers from text
**C `atoi`, PHP**: `atoi("12a")` → *12*, `(int)"abc"` → *0*  
Warp today: `int("12a")` → `0`.  
Intended: `int "12a"` returns an error value (`Result`, [DESIGN.md → Effects](DESIGN.md#effects)), never a plausible number.

### Type annotations not enforced loudly
Warp today: `x:int=5;x="five";x` → compiler panic `Cannot extract numeric value from 'five'` (rejected, but as a crash);
`const x=5;x=6;x` → `6` (`const` is ignored).  
Intended: a type/constness diagnostic with span and fix-it; `const` and `::=` enforce single assignment.

### Lists and arithmetic
**Python**: `[1,2,3]*2` → *[1,2,3,1,2,3]*; **NumPy**: *[2,4,6]*; **JS**: `[1,2]+[3]` → *"1,23"*  
Warp today: `[1 2]+[3]` → `5`, `[1 2 3]*2` → `6` (the list is summed first).  
Intended: `+` on lists is concatenation (as `tests/test_lists.rs` already expects); element-wise lifting only through
a law-governed rule ([DESIGN.md → Dangerous implicitness](DESIGN.md#dangerous-implicitness), [wiki/broadcasting.md](wiki/broadcasting.md)).

## Equality and identity

### 🐞 String comparison
**Java**: `new String("abc") == "abc"` → *false*; **Python**: `a is b` works for `256` but not `257`  
Warp today: `"abc"=="abc"` → compiler panic `Cannot extract numeric value from 'abc'`; `"abc" is "abc"` stays unevaluated;
`0==""` and `null==false` panic.  
Intended ([wiki/equality.md](wiki/equality.md)): `==` and `is` compare values structurally, there is no identity operator in
the surface language. (`test_string_equality_is_by_value`)

### Unicode normalization
**Almost every language**: `"é" == "é"` (NFC U+00E9 vs NFD e+U+0301) → *false*  
Warp today: identical encodings → `1`, NFC vs NFD → compiler panic.  
Intended: text is normalized to NFC when parsed, so equal-looking strings are equal. (`test_unicode_normalization`)

### Duplicate keys
**JSON (RFC 8259 leaves it open), JS, Python `json`**: `{"a":1,"a":2}` → *silently {"a":2}*  
Warp today: `{a:1 a:2}` is accepted without complaint (field access on objects does not evaluate yet: `x={a:1 b:2};x.a` → unevaluated).  
Intended: duplicate keys are a parse diagnostic in data, an explicit override in code.

## Strings

### Bytes vs characters vs graphemes
**Python 2, C, Go, JS**: `len("👍🏽")` → *8* bytes (Go), *4* UTF-16 units (JS), *2* codepoints (Python 3); users mean *1*  
Warp today: `size "👍🏽"` → `8`, `'héllo'#2` → `'Ã'` (half of the UTF-8 `é`), though [wiki/indexing.md](wiki/indexing.md) promises `#` is character-safe; `'héllo'.length` is not evaluated.  
Intended: `#` and default iteration are by character (grapheme), `[]` by byte; the unit is part of the type
(`for byte in text`, [DESIGN.md → Cautions](DESIGN.md#cautions)). (`test_character_indexing_is_unicode_safe`)

## Truthiness and null

### Empty values
**Python/JS disagree**: `[]` is falsy in Python, truthy in JS; `if ("")` / `if ({})` differ too.  
Warp today: `if "" {1} else {2}` and `if [] {1} else {2}` → compiler panic.  
Intended: uniform rule of [wiki/truthiness.md](wiki/truthiness.md) (all empty values falsy)

### `and`/`or` as ternary
**Python, Lua**: `cond and x or y` → *y* when `x` is falsy  
Warp today: `1 and 0 or 2` → `2` while `if 1 then 0 else 2` → `0` (documented in [wiki/truthiness.md](wiki/truthiness.md)).  
Intended: lint `a and b or c` with a fix-it to `if a then b else c`.

### Null
**Java, C#, JS**: `obj.field` on null → *NullPointerException* / *TypeError at runtime*  
Warp today: `x=ø; x+1` → compiler panic (loud, but not a diagnostic).  
Intended ([wiki/null.md](wiki/null.md)): typed null, `T?` optional types, flow-sensitive `if x {…}` narrowing; no member access on `T?` without a check.

## Syntax and precedence

### Unary minus and power
**Excel, bash**: `=-2^2` → *4*  
Warp today: `-2^2` → `4`. Mathematics and Python say `-4`. (`test_negative_power_precedence`)

### `not` and bitwise operators vs comparison
**C**: `!1 == 2` → *0*, `3 & 4 == 4` → *1* (`&` binds weaker than `==`)  
Warp today: `not 1==2` → `0`, `3 & 4 == 4` → `1`, both C's answers. Python says `True` and `False`. Also `3 | 4` → `3` (`|` is pipe, not bitwise or).  
Intended: `not`, `&`, `|` bind weaker than comparisons; mixing bitwise and comparison without grouping is a diagnostic
([wiki/precedence.md](wiki/precedence.md)). (`test_logic_binds_weaker_than_comparison`)

### Chained comparison
**C, JS**: `3 > 2 > 1` → *false* (`true > 1`)  
Warp today: `3>2>1` → `0` (`1<2<3` → `1` only by luck). Intended: mathematical chaining as in Python, `3>2>1` → `true`.

### Assignment in a condition
**C, JS**: `if (x = 2)` → *always true, x overwritten*  
Warp today: `x=1;if x=2 {3} else {4}` → `3`; even `if 1=2 {3} else {4}` → `3`.  
Intended ([wiki/Bad.md](wiki/Bad.md) "assignment, declaration, comparison"): `=` in a condition is comparison or a
diagnostic, never assignment; persisted resolution if ambiguous ([DESIGN.md → Content-addressed resolutions](DESIGN.md#content-addressed-resolutions)).

### 🐞 Increment
**C**: `i++ + i++` → *undefined behaviour*  
Warp today: `x=1;x++;x` → `1` (the increment is lost), `++i` is a parse error; [wiki/equality.md](wiki/equality.md) says
`++` is immediate, so `i++` and `++i` are the same. (`test_increment_changes_variable`)

### Braceless calls
Warp today: `f := it*10; 1 + f 3` → `3` 🐞 (should be `31`), and the recursive case from [wiki/Bad.md](wiki/Bad.md)
`fib := it<2 ? it : fib it-1 + fib it-2` fails with `Undefined variable: it`. (`test_braceless_call_as_operand`)

## Mutation and scope

### Aliasing
**Python, JS, Java**: `a=[1]; b=a; b[0]=9; a` → *[9]*  
Warp today: `a=(1 2);b=a;b#1=9;a#1` → `9`, and even strings: `x="ab";y=x;y#1="z";x` → `'zb'`.  
Intended ([DESIGN.md → Ownership](DESIGN.md#ownership-and-resource-inference)): value semantics; mutation needs a unique place,
otherwise copy-on-write. (`test_mutation_through_alias_is_not_visible`)

### Mutable default arguments
**Python**: `def f(a=[]): a.append(1); return a` → second call returns *[1, 1]*  
Warp today: default arguments work for numbers (`tests/test_functions.rs`), `def f(a=()): a.add(1); f(); f()` panics. Unverified.  
Intended: defaults are values evaluated per call; with value semantics the footgun cannot occur.

### Closures capturing loop variables
**Python, JS `var`, Go < 1.22**: `[lambda: i for i in range(3)]` → all return *2*  
Warp today: functions do not see outer variables at all: `x=1;f(y):=x+y;f(1)` → `Undefined variable: x`. Unverified.  
Intended: closures capture values (immutable bindings), so each iteration's `i` is its own.

## Bounds

### Index out of range / negative index
**C**: `a[3]` on 3 elements → *undefined behaviour*; **JS**: *undefined*; **Python**: `a[-1]` wraps silently  
Warp today: `x=[1 2 3]; x[3]` → the unevaluated program text `x=[1 2 3]; x#4`; `x#0` → `1`, `x[-1]` → `1`.  
Intended: out of range is an error value with span; `#0` is an error (1-based), `#-1` means last only if spelled so. (`test_index_out_of_bounds_is_an_error`)

## Data formats

### The Norway problem
**YAML 1.1**: `country: NO` → *country: false*  
Warp today: `country: NO` → `country:0`, `yes` → `1`.  
Intended: only `true`/`false` (and `✔`/`✖`?) are boolean literals in data; `NO`, `no`, `yes`, `on` stay symbols. Needs a decision since `yes` is a documented alias.

### Numbers that are not numbers
**YAML, Excel, CSV importers**: `version: 1.10` → *1.1*, `zip: 01234` → *1234*  
Warp today: `version: 1.10` → `version:1.1`, `zip: 01234` → `zip:1234`.  
Intended: serialization round-trips the literal (keep the source text in `Meta`), a leading zero or trailing zero after
the point keeps the value text-like unless a numeric type is expected.

### Data that executes
**Early JS `eval(json)`, YAML `!!python/object`, pickle**: loading data runs code  
Warp today: `WaspParser::parse` is data only; `eval` of an untrusted `.wasp` file runs it, but only with the capabilities
its imports declare (`tests/test_effects.rs::test_imports_follow_effects`).  
Intended: a CLI/data loading path that parses without evaluating, and evaluation of foreign data only with an empty capability set.

## Errors

### Swallowed errors
**Go**: `v, _ := f()`; **Java**: `catch (Exception e) {}`; **JS**: unhandled promise rejection → *silent*  
Warp today: law and effect violations come back as `Error` values (verified above), but many failures are compiler panics
(`Cannot extract numeric value …`) or silent wrong values (`int("12a")` → `0`, `x[3]`).  
Intended: `Result<T, E>` as data, every diagnostic with span, resolved facts and fix-it ([DESIGN.md → The compiler is a query interface](DESIGN.md#the-compiler-is-a-query-interface)).

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

### Fast *and* lawful floats
**IEEE 754**: `(0.1+0.2)+0.3 == 0.1+(0.2+0.3)` → *false*; hardware floats are not associative.  
Warp today: `0`. Under `@f64` this stays true forever; the choice is exact numbers (slower) or floats whose laws are weaker. `law` makes the chosen trade-off checkable.

### Function equality
`f == g` for arbitrary functions is undecidable.  
Warp: laws state the properties that matter and are tested or proved per function.

### "The" length of a string
There is no single right answer (bytes, UTF-16 units, codepoints, grapheme clusters), segmentation changes with Unicode versions,
and case mapping is locale dependent (Turkish `i` ↔ `İ`).  
Warp: make the unit explicit (`byte`, `char`, `grapheme`) instead of choosing silently.

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
