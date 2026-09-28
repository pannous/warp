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

### Sum of quotients truncated
Warp's own bug, fixed: `1/4+1/4` → `0` while `1/4+1/4 == 0.5` → `1`; the sum's result type was inferred as Int and truncated on return.  
Warp: `1/4+1/4` → `0.5`: type inference knows `/` never yields an Int (`analyzer::arithmetic_kind`). (`test_sum_of_quotients_is_not_truncated`)

### Decimal fractions
**Python, JS, Java, C, …**: `0.1 + 0.2 == 0.3` → *false* (`0.30000000000000004`), `1/3*3 == 1` → *false* in some cases  
Warp before this change: `0.1+0.2==0.3` → `0`, `1/3*3==1` → `0`.  
Warp: numbers are exact rationals by default ([DESIGN.md → Exact numbers by default](DESIGN.md#exact-numbers-by-default)):
`0.1+0.2==0.3` → `1`, `0.1+0.2` → `0.3`, `1/3*3==1` → `1`, `1/3` → `1/3` (`Number::Quotient`), `2^-2` → `0.25`.
An Int handle may point at a `$Ratio` next to the `$BigInt`s, so integers keep the i64 fast path and only the slow paths
see ratios (`src/wasm_emitter/exact.rs`). A decimal literal with up to 15 significant digits is exact; f64 only comes from
`sqrt`/FFI float functions, irrational constants (`π`) and `as float`, and an f64 operand makes the result an f64.
`x /= y` keeps an integer `x` an integer (truncating), `as int` truncates a ratio.
Still inexact: `3 == 3.0000000000000001` → `1`, the literal is parsed to f64 before it becomes exact.
Ratios whose parts exceed i64 come back from WASM as an f64 approximation (with a warning) because `Quotient` is `(i64, i64)`.
(`test_exact_decimal_arithmetic`, `test_exact_rationals_stay_exact`)

### NaN and infinity
**IEEE 754 everywhere**: `NaN == NaN` → *false*, `1/0` → *Infinity*, `sqrt(-1)` → *NaN*, and NaN poisons all later math quietly.  
Warp before this change: `x=0.0/0.0; x==x` → `0`, `1/0` → `inf` (an f64).  
Warp: exact division by zero gives the extended rationals ±∞ = ±1/0 and NaN = 0/0 ([wiki/int60.md](wiki/int60.md) reserves ±∞ and NaN too):
`1/0` → `∞`, `-1/0` → `-∞`, `1/0 > 10^100` → `1`, `1/(1/0)` → `0`, `1/0 - 1/0` → `NaN`; NaN equals only itself:
`x=0/0; x==x` → `1`, `0/0 == 1` → `0`. Truncating ∞ or NaN to an integer (`(1/0) as int`) traps.
Left open: `sqrt(-1)` → `NaN` is still the f64 of the float path (`√-1` could be `i` since `Complex` exists), and
ordering against NaN is not total (`0/0 < 1` → `0` but `0/0 > 1` → `1`).
(`test_division_by_zero_is_extended_rational`, `tests/test_types.rs::test_auto_type_nan`)

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

### Unary minus and power
**Excel, bash**: `=-2^2` → *4*  
Warp: `-2^2` → `-4`, as in mathematics and Python; unary minus binds weaker than `^` but tighter than `*`
(`-2*3` → `-6`, `(-2)^2` → `4`). (`test_negative_power_precedence`, `test_negative_literals_after_power_fix`)

### `not` vs comparison
**C**: `!1 == 2` → *0* (`(!1) == 2`)  
Warp: `not 1==2` → `true`, also `!1==1` → `false`: `not` binds weaker than comparisons, tighter than `and`/`or`
(`not 1==2 and 2==2` → `true`). (`test_not_binds_weaker_than_comparison`; `&` see NOT YET)

### Chained comparison
**C, JS**: `3 > 2 > 1` → *false* (`true > 1`)  
Warp: `3>2>1` → `true`, `1<3<2` → `false`: mathematical chaining as in Python, `a<b<c` means `a<b and b<c`; all comparisons
share one precedence level so `1<2==2` → `true` chains too. Explicit grouping does not chain: `(3>2)>1` → `false`. (`test_chained_comparison`)

### Assignment in a condition
**C, JS**: `if (x = 2)` → *always true, x overwritten*  
Warp: `x=1;if x=2 {3} else {4}` → `4` and `x` stays `1`; `if 1=2 {3} else {4}` → `4`. Inside an `if`/`while` condition
`=` is comparison ([wiki/Bad.md](wiki/Bad.md) "assignment, declaration, comparison"); a `{block}` or the `:` branch
inside the condition assigns again. (`test_equals_in_condition_compares`)

### Increment
**C**: `i++ + i++` → *undefined behaviour*  
Warp: `x=1;x++;x` → `2`, `i=1;++i` → `2`, `i=3;--i;i` → `2`. As [wiki/equality.md](wiki/equality.md) says, `++` is immediate,
so `i++` and `++i` are the same. (`test_increment_changes_variable`, `test_prefix_increment`)

### Braceless call as operand
Warp: `f := it*10; 1 + f 3` → `31`: a braceless call is a valid operand of `+ - * /` (`2 * f 3` → `60`). (`test_braceless_call_as_operand`)

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

### Scientific notation and digit separators
**Python/JS/Rust**: `1e3` → `1000.0`, `1_000_000` → `1000000`  
Warp before 9682281d: `1e3` → the list `1 e3`,
`1_000_000` → the list `1 _000_000`, `.1` → parse error: silently parsed as something else.
Warp: `1e3` → `1000`, an exact integer like the literal it abbreviates (`1e20 == 100000000000000000000` → `true`);
`1.5e3` → `1500`, `2E-3` → `0.002` (floats); `1_000_000` → `1000000` (`_` only between digits); `.5+1` → `1.5`, `-.5+1` → `0.5`,
while `[.1 .2]` stays a two-element list. `2em`, `1e`, `1_` are not numbers.
(`test_scientific_notation`, `test_scientific_notation_forms`, `test_digit_separators`, `test_leading_dot_literal`)

### Proof model matches unbounded integers
**Lean/Coq/Dafny exports over a mismatched numeric type** can prove statements the runtime violates or reject statements it satisfies.
Warp exports its unbounded Int as Lean `Int`: `law square(x) >= 0` is both true at runtime for `3037000500` and proved universally.
Explicit `as i64` values still wrap and are not exported yet. Details in `notes/laws.md`.
(`test_proof_model_matches_unbounded_int`, `tests/test_law.rs`)

### Closures capturing loop variables
**Python, JS `var`, Go < 1.22**: `[lambda: i for i in range(3)]` → all return *2*  
Warp: functions capture the outer variables they read by value, at the point of definition:
`x=1;f(y):=x+y;x=5;f(1)` → `2`; a function defined in a loop sees that iteration's value:
`x=0;r=0;i=0;while i<3 { i+=1; x=i; f(y):=x+y; x=100; r+=f(0) }; r` → `6` (by reference: 300).
Captured lists and texts work too: `xs=(1 2 3);f(i):=xs#i;f(2)` → `2`.
(`test_closures_capture_values`, `test_closures_in_loop_capture_each_iteration`)  
Decision: capture by value at definition time, matching DESIGN.md "immutable local bindings"; a name bound to a function is
its latest definition (functions are not yet first-class values).
(alternatives: capture by reference, as wiki/assignment.md sketches for `z := y*y` re-evaluating with the current `y`, which
reintroduces this footgun; or no capture at all, the old behaviour.)
Implementation: each captured variable gets a WASM global per function, set where the function is defined.

### Mutable default arguments
**Python**: `def f(a=[]): a.append(1); return a` → second call returns *[1, 1]*  
Warp: a default is an expression evaluated at every call that omits the argument, so each call gets a fresh value:
`def f(a=(0 0)){ a#1 = a#1 + 1; a#1 }; f()+f()` → `2` (shared default: 3). A parameter takes the kind of its default
(list, text, float), untyped parameters stay Int. (`test_default_argument_is_fresh_per_call`)  
Decision: defaults are evaluated per call, at the call site (alternatives: evaluate once at definition, Python's choice,
safe only once value semantics / copy-on-write lands).  
Still open: `a.add(1)` on a list is a no-op today (`a=();a.add(1);a` panics, `pixel.add(5)` does not grow `pixel`), see the lists tests.

### Date guessing in data
**Excel**: typing the gene name `SEPT2` → *2-Sep*  
Warp: `SEPT2` → symbol `SEPT2`; no date or unit guessing when reading data.

### The Norway problem
**YAML 1.1**: `country: NO` → *country: false*  
Warp: `warp data` / `parse_data("country: NO")` → `country:NO`; `answer: yes` → `answer:yes`; `[de gb no]` stays three symbols.
In data only JSON's words `true`, `false`, `null` (and glyphs like `✔`, `⊥`, `ø`) are literals. (`test_norway_problem_in_data`)  
Decision: code and data differ, as in wiki Todo.md "norway-problem" solution 1 (symbol vs expression context): in code
`yes`/`no` stay the documented boolean aliases (`warp 'country: NO'` → `country:0`, quote `'NO'` there); in data (the
parse-only path) every English word except `true`/`false`/`null` is a symbol, so aliases `yes`, `no`, `on`, `none`, `pi`
never change foreign data. (alternatives: drop `yes`/`no` everywhere (breaks the documented alias, `peq!("no",Empty)`);
case-sensitive aliases only (still turns lowercase `no` into false); a `%w[de gb no]`-style symbol list syntax.)

### Numbers that are not numbers
**YAML, Excel, CSV importers**: `version: 1.10` → *1.1*, `zip: 01234` → *1234*  
Warp: `parse_data("version: 1.10").serialize()` → `version:1.10`, `zip: 01234` → `zip:01234`, `0xFF` and `1_000` too:
a number whose value prints differently keeps its source text in `Meta` (`Node::source_literal`), and serialization prints it.
The value is still the number 1234. (`test_data_keeps_number_literals`)  
Decision: the literal stays a number with its spelling kept in `Meta` (DESIGN.md: metadata rides along, the value stays
exact); code keeps evaluating `010` → 10 (Octal literals above). (alternatives: leading-zero / trailing-zero literals
become text unless a numeric type is expected (breaks `010` → 10 and makes the type depend on spelling); require quoting.)

### Data that executes
**Early JS `eval(json)`, YAML `!!python/object`, pickle**: loading data runs code  
Warp: `warp data <file>` and `warp::parse_data` read `.wasp` data without evaluating it (`secret = fetch …` comes back as
the text of the call). Evaluating foreign data goes through `wasm_emitter::eval_untrusted`, which runs with an empty
capability set: a program resolving any host, WASI or FFI call (`puts`, `fetch`, `use m;floor`) is an error before it is
compiled; pure code (`x:=3;x*x` → 9) runs. Plain `eval` still grants the capabilities the program's effects need
(`tests/test_effects.rs::test_imports_follow_effects`). (`test_data_does_not_execute`)

### Memory-safety classics: use-after-free, double free, dangling pointers
**C, C++**: `free(p); p->x` → *undefined behaviour*  
Warp: by construction. Nodes are WASM GC structs and the surface language has no pointers, no `free`, no pointer
arithmetic; the WASM sandbox traps any access outside linear memory, and indexing is bounds checked (next entry).

### Index out of range / negative index
**C**: `a[3]` on 3 elements → *undefined behaviour*; **JS**: *undefined*; **Python**: `a[-1]` wraps silently  
Warp before: `x=[1 2 3]; x[3]` → the unevaluated program text `x=[1 2 3]; x#4`; `x#0` → `1`, `x[-1]` → `1`, `"ab"#5` → `''`.  
Warp: `x=[1 2 3]; x[3]`, `x#0`, `x[-1]`, `"ab"#3` and `x#4=0` → `Error('index out of range')`. Every list and text
index is checked at runtime; the trap becomes an error value instead of the program text. Not yet: the error carries no
source span, and `#-1` (last element, only if spelled so) is not implemented, so it is an error too.
(`test_index_out_of_bounds_is_an_error`, `test_negative_index_is_an_error`)

### Parsing numbers from text
**C `atoi`, PHP**: `atoi("12a")` → *12*, `(int)"abc"` → *0*  
Warp before: `int("12a")` → `0`, `float("1.5x")` → `0.0`, `int("123456789012345678901234567890")` → a truncated double.  
Warp: `int("12a")`, `float("1.5x")`, `int("")` → `Error('invalid number')`; only a text that is one whole number literal
converts (`int(" -12 ")` → `-12`, `int("2.7")` → `2`, big literals stay exact). Not yet: the error aborts the program
instead of being a recoverable `Result` ([DESIGN.md → Effects](DESIGN.md#effects)), and runtime (non-literal) text is not converted at all.
(`test_invalid_number_text_is_an_error`)

### Aliasing
**Python, JS, Java**: `a=[1]; b=a; b[0]=9; a` → *[9]*  
Warp before: `a=(1 2);b=a;b#1=9;a#1` → `9`, and even strings: `x="ab";y=x;y#1="z";x` → `'zb'`; worse, equal literals
share one string-table entry, so `x="ab";y="ab";y#1="z";x` → `'zb'` without any alias.  
Warp: value semantics ([DESIGN.md → Ownership](DESIGN.md#ownership-and-resource-inference)): `y#i=v` stores an updated copy
in `y` (`node_with_at`: a list copies the cells up to `i` and shares the tail, a text gets fresh bytes), nothing is mutated in
place, so all three examples leave `x`/`a` unchanged. Not yet: no uniqueness analysis, so every index assignment copies
(O(i) for lists, O(length) for texts, runtime texts are bump-allocated and never freed).
(`test_mutation_through_alias_is_not_visible`, `test_equal_literals_are_not_shared`)

### String + number
**JavaScript**: `"5" + 3` → *"53"*, `"5" * 3` → *15*  
Warp before: `"5"+3` → `56`, `"5"*3` → `159`, `"a"+1` → `98`, `3 + "4"` → `55` (one-character strings became code points, C's `'5' + 3`),
`"ab"+3` → compiler panic.  
Warp: all of them → `Error('type error: codepoint + int: no implicit conversion, convert explicitly, e.g. int("5") + 3')`
([DESIGN.md → Dangerous implicitness](DESIGN.md#dangerous-implicitness)); `int("5") + 3` → `8`. Arithmetic on a text,
character or list operand is a compile-time type error (`arithmetic_kind` → `Kind::Error`), reported before the module runs.
Not yet: no source span; text concatenation (`"a" + "b"`) is not implemented, so it is a type error too.
(`test_text_plus_number_is_a_type_error`)

### Lists and arithmetic
**Python**: `[1,2,3]*2` → *[1,2,3,1,2,3]*; **NumPy**: *[2,4,6]*; **JS**: `[1,2]+[3]` → *"1,23"*  
Warp before: `[1 2]+[3]` → `5`, `[1 2 3]*2` → `6` (the list was evaluated as a statement sequence: its last item).  
Warp: `[1 2]+[3]` → `[1 2 3]` (`list_concat` copies the left cells and shares the right list, `a+b` leaves `a` unchanged);
`[1 2 3]*2` → `Error('type error: list * int: lists only concatenate with lists (+), element-wise arithmetic needs an explicit map')`,
element-wise lifting only through a law-governed rule ([wiki/broadcasting.md](wiki/broadcasting.md)).
Not yet: `pixel + 4` (append a scalar, expected by the ignored `test_array_operations`) is a type error.
(`test_list_plus_concatenates`)

### Compound assignment to an element
Warp before: `a=(1 2);a#1 += 1;a#1` → compiler panic `Expected symbol in compound assignment`.  
Warp: `2`; `x#i op= v` is `x#i = x#i op v` with the same value semantics as `x#i = v`. (`test_compound_index_assignment`)

### `const` ignored
Warp before: `const x=5;x=6;x` → `6`.
Warp: `Error('x is const, cannot assign it again: x=6 at 1:11; fix: use a new name instead of x, or declare it without const')`,
also for `x+=1`, `x++` and element assignment `a#1=3`; the check runs before emission (`check_constants`), then `const x=v`
lowers to `x=v`. Not yet: `::=` does not parse (`Unexpected character '='`), and a const is not scoped (a function
parameter of the same name is not reassigned, so no false positives, but no shadowing rules either).
(`test_const_is_single_assignment`)

### Methods that should update a list
Warp before: `pixel=(1 2);pixel.add(5);pixel` → `(1 2)` (the call was evaluated and dropped), `a=();a.add(1);a` → compiler panic
`Cannot extract numeric value from ø`, `[4]#1` → trap (a one-element `[…]` was emitted as its element).
Warp: `x.add(v)` (also `append`, `push`) on a variable is sugar for `x = x + [v]`, so `pixel` → `(1 2 5)` and an alias keeps
its value; `[4]` stays a list. Not yet: `()` parses as ø, so `a=();a.add(1)` is now the null-use diagnostic
`a may be ø (null) in a.(add 1)` instead of a panic; making `()` the empty list is the null/truthiness owners' call.
(`test_append_method_rebinds_the_list`)

### Data races
**C, C++, Go, Java**: two threads incrementing a shared counter → *lost updates*  
Warp: currently vacuous: generated modules are single threaded and share no memory. The plan keeps it that way: parallelism
is only inferred for code proved pure ([DESIGN.md → Cautions](DESIGN.md#cautions)).

### String comparison
**Java**: `new String("abc") == "abc"` → *false*; **Python**: `a is b` works for `256` but not `257`; **JS**: `[1,2]==[1,2]` → *false*  
Warp before: `"abc"=="abc"` → compiler panic `Cannot extract numeric value from 'abc'`; `"abc" is "abc"` unevaluated;
`0==""` and `null==false` panicked.  
Warp: `"abc"=="abc"` → `1`, `x="abc";x=="abc"` → `1`, `"abc" is "abc"` → `1`, `x=257;x is 257` → `1`, `[1 2]==[1 2]` → `1`,
`0==""` → `0`, `null==false` → `0`, `3==3.0` → `1`. `==`, `!=` and `is` compare by value ([wiki/equality.md](wiki/equality.md)):
kinds must be compatible (Int and Float are), texts byte by byte, lists and keys recursively; there is no identity operator.
(`test_string_equality_is_by_value`, `test_is_compares_by_value`, `test_equality_across_kinds_is_structural`)

### Unicode normalization
**Almost every language**: `"é" == "é"` (NFC U+00E9 vs NFD e+U+0301) → *false*  
Warp before: NFC vs NFD → compiler panic.  
Warp: `'\u{e9}'=='e\u{301}'` → `1`. Source text is normalized to NFC when parsed, so equal-looking texts and
identifiers are equal. (`test_unicode_normalization`)

### Duplicate keys
**JSON (RFC 8259 leaves it open), JS, Python `json`**: `{"a":1,"a":2}` → *silently {"a":2}*  
Warp before: `{a:1 a:2}` accepted without complaint.  
Warp: `{a:1 a:2}` → `Error('duplicate key 'a'')` at parse time. Code blocks without braces may still redefine
(`global x=5; global x=10; x` → `10`). Not yet: the error has no source span. (`test_duplicate_keys_are_reported`)

### Character indexing
**C, Go**: `"héllo"[1]` → *a byte, half of `é`*  
Warp before: `'héllo'#2` → `'Ã'`.  
Warp: `'héllo'#2` → `'é'`, `'a👍c'#3` → `'c'`, `'héllo'#6` → `Error('index out of range')`. `#` on text decodes UTF-8 and
counts code points (graphemes: see NOT YET → Bytes vs graphemes). (`test_character_indexing_is_unicode_safe`)

### Type annotations not enforced loudly
**Python** (hints are not checked), **TypeScript** (`as any`): `x: int = 5; x = "five"` → *accepted*  
Warp before: `x:int=5;x="five";x` → compiler panic `Cannot extract numeric value from 'five'`; `x:int="five"` → `0`; `x:float=5;x` → `0`.  
Warp: `x:int=5;x="five";x` → `Error('type mismatch: x is declared int, cannot assign text 'five' at 1:9; fix: x=int('five') or declare x:text')`;
`x:int=5⏎x=2.5` is rejected (no silent lossy conversion), `x:float=5;x` → `5.0` (widening), `x:int=42;type(x)` → `int`.
Declarations are checked before emission (`analyzer::diagnose`) and lowered to a typed local (`lower_declarations`).
Not yet: only literal values are checked against the declaration; `const` (see NOT YET). (`test_type_annotation_is_enforced`)

### Empty values
**Python/JS disagree**: `[]` is falsy in Python, truthy in JS; `if ("")` / `if ({})` differ too; **C**: `if (0.5)` is true, but a truncating cast makes it false  
Warp before: `if "" {1} else {2}` and `if [] {1} else {2}` → compiler panic; `if 0.5 {1} else {2}` → `2`, `if 2^32 {1} else {2}` → `2` (condition truncated to i32).  
Warp: `if "" …`, `if [] …`, `if ø …`, `x="";if x …` → else branch; `if "0" …`, `if 0.5 …`, `if 2^32 …` → then branch. One rule
([wiki/truthiness.md](wiki/truthiness.md)): `false`, `0`, `0.0`, `ø` and empty text/lists are falsy, everything else is truthy; it is defined
once (`Node::is_falsy`, the `is_truthy` runtime) and used by `if`, `while`, `?:`, `and`, `or`.
(`test_empty_condition_does_not_panic`, `test_empty_values_are_falsy`)  
Decision: keep "empty is falsy", uniformly (alternatives: (b) only `bool` is a condition, `if ""` a type error with fix-it
`if not empty ""` (Rust, Swift, Go); (c) empty is truthy except `ø`/`false` (Ruby, Lua)). Reasons: the wiki specifies it and
[wiki/null.md](wiki/null.md) builds optional narrowing on `if x {…}`; the Solved entry `"0" is falsy` pins `if "0"` → then branch,
which (b) would turn into an error; DESIGN.md's "arbitrary truthiness" objection targets per-type emitter heuristics, which one
rule defined in one place is not. The ambiguous `a and b or c` idiom is linted instead (next entry). Revisit with (b) if a
`bool` kind is introduced (NOT YET → Booleans are integers).

### `and`/`or` as ternary
**Python, Lua**: `cond and x or y` → *y* when `x` is falsy  
Warp: `1 and 0 or 2` → `2` (well defined, as in [wiki/truthiness.md](wiki/truthiness.md)), but it prints the warning
``warning: `1 and 0 or 2` yields 2 whenever 0 is falsy at 1:1; fix: if 1 then 0 else 2``; `if 1 then 0 else 2` → `0`.
`analyzer::lint` returns the warnings as `Diagnostic`s. (`test_and_or_ternary_is_linted`)

### Null
**Java, C#, JS**: `obj.field` on null → *NullPointerException* / *TypeError at runtime*  
Warp before: `x=ø; x+1` → compiler panic.  
Warp: `x=ø; x+1` → `Error('x may be ø (null) in x+1 at 1:6; fix: if x { x+1 }')`, likewise `x=ø; x.size`. A variable assigned `ø`
may not be used in arithmetic or member access until checked: `if x {x+1} else {2}` is accepted (flow-sensitive narrowing,
[wiki/null.md](wiki/null.md)), as is reassignment `x=ø; x=3; x+1`. Not yet: `T?` syntax and running such programs (NOT YET → Null: optional types).
(`test_null_needs_a_check`)
### SQL and shell injection
**Every language with string building**: `"SELECT * FROM t WHERE name='" + name + "'"` with `name = "x' OR '1'='1"` → *all rows*  
Warp before: no SQL or shell API; building a query was plain text concatenation.  
Warp: `name="x' OR '1'='1";q=sql "SELECT * FROM t WHERE name = $name"` → `q#1` is `SELECT * FROM t WHERE name = ?` and
`q#2` is the value `x' OR '1'='1`: the literal is the query, every `$name` / `${expr}` hole is a parameter, so no value can
change the query's shape. `sh "rm -f $file"` is an argument vector: with `file="a; rm -rf ~"` the hole is one argument
(`c#3`), nothing parses it as shell. Misuse is a diagnostic with position and fix-it: `sql("…'" + name + "'")` →
`sql takes a literal template`, a hole inside SQL quotes (`'$name'`) → `a parameter is a value`, `sh "ls | grep x"` →
`no shell`, `sh "cp --out=$f"` → `a sh hole must be a whole argument`, `execute q` of plain text → `execute takes a sql template`.
Running is a capability: `execute` (sql) and `exec` (process) are IO externals, so `effects of` shows them and `! Pure`
rejects them along the call chain, and `eval` grants neither: `execute sql "SELECT 1"` →
`capability denied: execute needs the sql capability, which eval does not grant` ([DESIGN.md → Effects as enforced capabilities](DESIGN.md#effects-as-enforced-capabilities)).
Decision: tag-prefixed templates `sql "…"` / `sh "…"` with holes as parameters, `?` placeholders, shell commands as argv
without a shell, runners gated by the `sql` / `process` capability (alternatives: an escaping function like `quote(name)`
(opt-in, forgotten once is enough, PHP's `mysql_real_escape_string`); typing every text with its language (`Text<Sql>`,
heavier, and concatenation would still need rules); JS tagged templates with a user tag function (flexible, but the tag
sees the pieces at runtime, so the capability cannot be checked statically); running `sh` through `/bin/sh -c` with quoted
holes (keeps pipes, but one quoting bug is an injection)).
Not yet: no host grants `sql` or `process`, so nothing actually runs a query or program; the template's language is tracked by
the lowering pass (`src/injection.rs`) per variable, not by the type system; pipes and redirection have no typed form.
(`test_sql_holes_are_parameters_not_text`, `test_sql_from_built_text_is_rejected`, `test_shell_holes_are_whole_arguments`,
`test_running_queries_and_commands_needs_a_capability`)

### Dates and time zones
**JS**: `new Date(2024, 1, 31)` → *March 2*; **Java**: `new Date().getYear()` → *124*; everyone: local time jumps at DST  
Warp before: no date/time type; `2024-01-31` parsed as the subtraction `2024-1-31` → `1992`.  
Intended: distinct `instant`, `date`, `local time`, `zoned time`; no implicit current zone; months are 1-based; arithmetic on calendar units is explicit about overflow (`Jan 31 + 1 month`).

Decision: four distinct types, modelled on JS Temporal / java.time and TOML's literal rule ([Solved elsewhere → Date guessing](Solved-elsewhere.md)):
`date` (`2024-01-31`), `local time` (`2024-01-31T10:00`, date + wall clock, no zone), `instant` (`2024-01-31T09:00Z` or
`…T10:00+01:00`, a point on the UTC line) and `zoned time` (`2024-01-31T10:00[Europe/Berlin]`, RFC 9557).
Literals are the RFC 3339 / RFC 9557 forms only (4-digit year, 2-digit month and day, no spaces), so `2024-01-31` is a date
while `2024 - 1 - 31` stays arithmetic and `SEPT2` stays a symbol; `date(y,m,d)` is the constructor.
Months are 1-based (`2024-02-29.month` → 2); invalid fields are errors, never rolled over (`date(2024,2,30)` → error, JS: March 1).
No implicit zone: `now` is an `instant`, which has no `year`/`hour` until placed in a zone (`t in "Europe/Berlin"` → zoned time);
nothing reads the host's zone. Types never convert implicitly: `date < local time` and `local time < instant` are errors.
Calendar arithmetic rejects overflow by default: `2024-01-31 + 1 month` → error naming `2024-02-31`; clamping is spelled out,
`add(2024-01-31, 1 month, overflow: clamp)` → `2024-02-29` (Temporal's `overflow: "constrain"`). A nonexistent wall time in a
DST gap is an error for literals, never a silent shift; `1 day` on a zoned time is a calendar day, `24 hours` is exact.
`date - date` → days as Int.
(alternatives: Temporal's default `constrain` for `+` (hides the footgun behind a quiet clamp), JS/`java.util.Date` rollover
(the footgun itself), a single timestamp type plus zone field (Python naive/aware datetime: mixing them fails only at runtime),
constructor-only dates with no literal (safe but verbose; the RFC 3339 form is already unambiguous data, as in TOML).)
Warp: implemented as decided. `2024-02-29.month` → `2`, `date(2024,0,1)` → `Error('month out of range: 0 …')`,
`date(2024,2,30)` → `Error('day out of range: 2024-02-30')`, `2024-01-31 + 1 month` → `Error('2024-02-31 does not exist: use
add(…, overflow: clamp) …')`, `add(2024-01-31, 1 month, overflow: clamp) == 2024-02-29` → true (Int 1, as all eval booleans), `2024-03-01 - 2024-02-01` → `29`,
`now.hour` → `Error('instant has no hour …')`, `(2024-03-31T01:30Z in "Europe/Berlin").hour` → `3`,
`2024-03-31T02:30[Europe/Berlin]` → `Error('… does not exist in Europe/Berlin …')`, `date < local time` and
`local time < instant` → errors naming both kinds, `2024-01-31T10:00+01:00 == 2024-01-31T09:00Z` → true; `2024-1-31` is still `1992`.
How: `parse_number` lexes the strict RFC 3339 / RFC 9557 form into a `TimeLiteral` data node (validated on evaluation, like
`date(…)`), and an integer followed by a unit word (`year month week day hour minute second`, singular or plural) into a
`Duration` (months, days, exact nanoseconds). `src/time.rs` evaluates programs that use them at compile time
(`time::answer`, before emission); the calendar core is `src/time/calendar.rs` (no dependencies).
Not yet: dates have no WASM GC runtime representation, so a date program is folded at compile time and anything the folder
does not know (functions, loops, lists of dates) is an error saying so; `now` reads the compiler's clock (needs a host clock
import and a clock effect); the tz database is an embedded table of ~40 zones with today's EU/US rules only (no historical
rules, no southern-hemisphere DST): unknown zones are errors, the full IANA database via a host import is future work.
(`test_months_are_one_based`, `test_calendar_overflow_is_explicit`, `test_no_implicit_time_zone`,
`test_date_and_time_types_are_distinct`, `test_date_literal_needs_strict_form`)

### Exact results at the Node boundary
Since exact rationals, `3.14+2` returns the exact `Number::Quotient(257, 50)` (prints `5.14`), not `Number::Float`.
Decision (user, 2026-09-28): results stay exact at the API; `tests/test_type_upgrading.rs`'s helper also accepts `Quotient` and compares it as f64 (8c8a18bb).
(alternatives: convert non-integer results to f64 when leaving WASM, silently dropping exactness; decimal literals stay f64, giving up `0.1+0.2==0.3`.)

### Variance
**Java**: `Object[] a = new String[1]; a[0] = 1;` → *ArrayStoreException at runtime*  
Warp today: no generic/subtyping rules exist to be unsound. Lists are heterogeneous (every element is a `Node`), so
`a=("x" "y");a#1=1` has no static element type to violate; the Java scenario cannot even be written, there is no
`String[]` to upcast to `Object[]`. `a#1` → `1`, verified (`test_no_array_store_exception`). The inferred-invariance rule below applies once typed collections exist.  
Decision: variance is inferred, never annotated: an immutable collection (no mutation reachable through it, same analysis
as ownership/effects) is covariant, `[Text]` may be used as `[Any]`; a collection that is mutated through the widened
view is invariant, so `a:[Text]=…; f(b:[Any]) := b#1=1; f(a)` must be a type error, not a runtime trap. Value semantics
(copy-on-write, see Aliasing) makes the widened copy a new list, which is the other sound way out.
(alternatives: Java/C# covariant arrays with a runtime store check; Kotlin/Scala declaration-site `out`/`in`/`+T`/`-T`
annotations; Java/C# use-site wildcards `? extends T`; everything invariant like Rust/Go generics.)
Why: DESIGN.md prefers inference with optional constraints over mandatory annotations and requires unsound or lossy
operations to be explicit; inferred mutability is the same fact the ownership and effect analyses already compute.

### Rounding mode
**Python 3, .NET** `round(2.5)` → *2* surprises users of **JS/Excel** (`3`) and vice versa.  
Warp before: `round(2.5)` → `2` (banker's rounding), with no name for the rule and no alternative.  
Warp: `round` is round half even (IEEE 754 default, Python 3, .NET): `round(2.5)` → `2`, `round(3.5)` → `4`; `round_half_even`
names it; `round_half_up(2.5)` → `3`, `round_half_up(-2.5)` → `-2` (ties toward +∞, like JS `Math.round`), `round_half_up(5/2)` → `3`.  
Decision: `round` stays half even, `round_half_up` and `round_half_even` are explicit (alternatives: a mode argument
`round(x, half: up)`; switching `round` to half up like JS/Excel, a silent change).
Not yet: rounding goes through f64 (exact rationals could round exactly); no half-away-from-zero (Excel for negatives).
(`test_rounding_mode_is_named`)

### Negative modulo
**C, JS, Java**: `-5 % 3` → *-2*, **Python**: *1*, both surprise the other camp.  
Warp before: only `%`, the truncating remainder: `-7 % 3` → `-1`.  
Warp: `%` is the truncating remainder (sign of the dividend, as C/JS/Java/Rust): `-7 % 3` → `-1`; `mod` is the floored modulo
(sign of the divisor, as Python `%`, Haskell `mod`): `-7 mod 3` → `2`, `7 mod -3` → `-2`, `-123456789012345678901234567890 mod 1000` → `110`.
`mod` binds like `%` (`1 + -7 mod 3` → `3`) and works for unbounded integers and exact ratios.  
Decision: keep `%` truncating and add `mod` (alternatives: `%` floored and `rem` truncating, which contradicts
`tests/test_unbounded_int.rs` `is!("-7 % 3", -1)`; Euclidean modulo, always ≥ 0).
Not yet: `a mod b` is lowered in the parser to `(a % b + b) % b`, so the divisor is evaluated three times
(harmless for pure divisors, wrong for a divisor with side effects); no `mod=`.
(`test_modulo_and_remainder_are_both_named`)

### Booleans are integers
**Python, C, JS**: `True + True` → *2*  
Warp before: `true + true` → `2` (booleans are encoded as Int 1/0).  
Warp: arithmetic on a boolean is a compile error with a fix-it: `true + true` → `Error('arithmetic on a boolean: true+true at 1:1; fix: int(true) + int(true)')`,
likewise `(1<2) + 1` and `(not 1) + 2`. Booleans are still used as conditions (`x = 1 < 2; if x {1} else {2}`).  
Decision: reject arithmetic on booleans in the analyzer, keep the Int 1/0 runtime encoding (alternatives: a distinct
`Kind::Bool` with its own payload, which DESIGN.md's `Bool` semantic type calls for, but True/False are Int 1/0 at the
Node boundary, pinned by `tests/test_node_operators.rs` (`&True + &True == 2`) and by every boolean-returning `is!` test;
keep and document).
Not yet: the check sees boolean literals, comparisons and `not`, not variables holding booleans (no semantic `Bool`
type yet); `false == 0` → `1` still compares across kinds.
(`test_booleans_are_not_numbers`)

# NOT YET
...

Footguns Warp still has (verified with the probes above: the Warp answer shown is today's output), or where the fix
is designed but not implemented. Each entry names the intended resolution. Entries marked 🐞 are plain bugs, not design questions.

## Strings

### Bytes vs graphemes
**Python 2, C, Go, JS**: `len("👍🏽")` → *8* bytes (Go), *4* UTF-16 units (JS), *2* codepoints (Python 3); users mean *1*  
Warp today: `#` indexes code points (see Solved → Character indexing), but `size "👍🏽"` → `8` and `'héllo'.length` is not
evaluated; `'👍🏽'#1` is the thumb without its skin tone modifier.  
Intended: `#` and default iteration are by grapheme, `[]` by byte; the unit is part of the type
(`for byte in text`, [DESIGN.md → Cautions](DESIGN.md#cautions)).

## Truthiness and null

### Null: optional types
Warp today: the null check exists (Solved → Null), but `T?` is not parsed (`x:int?=ø` → `Unexpected character '='`),
and a local first assigned `ø` cannot be emitted (`x=ø; if x {x+1} else {2}` → compiler panic `Cannot extract numeric value from ø`).  
Intended ([wiki/null.md](wiki/null.md)): `T?` declarations, `x!` unwrap, typed null (`Person.null`) as a runtime value. (`test_optional_local_runs`)

## Syntax and precedence

### `&` and `|` vs comparison
**C**: `3 & 4 == 4` → *1* (`&` binds weaker than `==`)  
Warp today: `3 & 4 == 4` → `1`, C's answer; `3 & 4` → `4`. Python says `False`. Also `3 | 4` → `3` (`|` is pipe, not bitwise or).
(`not` is solved, see above.) (`test_logic_binds_weaker_than_comparison`)  
Decision needed: the test expects `false`, which only `(3 & 4) == 4` with a bitwise `&` gives (`3&4` is `0`). But in Warp `&` is
an alias of logical `and` ([wiki/&.md](wiki/&.md): "1 & 1 == 1 and 1 == true"), and the intended rule "`&`, `|` bind weaker
than comparisons" yields `3 and (4==4)` → `true`. Options:
1. `&` stays logical `and`, weaker than `==` (today): answer `true`; the test's `false` must change.
2. `&` becomes bitwise and, tighter than `==` (Python): answer `false`; breaks wiki/&.md and `square & print` composition.
3. Mixing `&`/`|` with a comparison without grouping is a diagnostic ([wiki/precedence.md](wiki/precedence.md) calls such mixes
   ambiguous): neither C's nor Python's reading can be silently picked.

Recommendation: 3 for the mixed form, keeping `&` = `and` for booleans and `bitand` as a named bitwise operator; then the test
should assert an error instead of `false`.

### Braceless calls
Warp today: `1 + f 3` → `31` is solved (see above), but as an operand the call only takes the next atom:
`f := it*10; 1 + f 3-1` → `30` (`1 + f(3) - 1`) while at statement level `f 3-1` → `20` (`f(3-1)`). The recursive case from
[wiki/Bad.md](wiki/Bad.md) `fib := it<2 ? it : fib it-1 + fib it-2` still fails with `Undefined variable: it`
(an identifier argument is only applied in assignment context).  
Decision needed: [wiki/precedence.md](wiki/precedence.md) calls `square 3 + square 3` ambiguous. Options: (1) argument extends to
the end of the enclosing operator's operand, `1 + f 3-1` → `1 + f(3-1)` → `21`; (2) argument is one atom (today, Haskell);
(3) diagnostic when an operator follows a braceless argument. Recommendation: 3, with the persisted resolution of
[DESIGN.md → Content-addressed resolutions](DESIGN.md#content-addressed-resolutions).

## Mutation and scope

### Index assignment of a multi-byte character
Warp today: `x="ab";x#1='é';x` writes one byte (reading `#` is character-safe since 91122fec, writing is byte-wise). Found by the strings agent.
Intended: `text_with_char_at` re-encodes the character as UTF-8 and splices it at the character index (the copy already allocates).

## Errors

### Swallowed errors
**Go**: `v, _ := f()`; **Java**: `catch (Exception e) {}`; **JS**: unhandled promise rejection → *silent*  
Warp today: law, effect, declared-type and null violations, invalid number text and index errors come back as `Error` values;
`src/diagnostic.rs` gives compile-time ones a position and fix-it (`… at 1:9; fix: …`). Still left: other emitter
failures are compiler panics (`Cannot extract numeric value …`, `Undefined variable`, pinned by `test_undefined_variable_is_an_error`),
runtime traps carry no span, and a link/instantiation failure still returns the parsed program (`failed_run`).  
Intended: `Result<T, E>` as data, every diagnostic with span, resolved facts and fix-it ([DESIGN.md → The compiler is a query interface](DESIGN.md#the-compiler-is-a-query-interface)).

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
Warp: the Lean export models Warp's actual unbounded integers as Lean `Int` (see Solved); property tests and runtime assertions check the compiled program, not the model.

### Nondeterministic NaN bits
The WebAssembly spec allows NaN payload bits to differ between engines (and relaxed SIMD results to differ between CPUs).  
Warp: canonicalize NaNs where determinism matters, at a cost; exact numbers avoid NaN altogether.

### Timing side channels
No high-level language can guarantee constant-time execution on arbitrary hardware (Spectre, cache timing); WASM engines JIT differently.  
Warp: out of scope, crypto belongs in audited host functions with the `FFI` effect.
