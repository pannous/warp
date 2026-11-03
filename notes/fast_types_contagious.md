# Fast types contagious? (investigation 2026-11-03)

The user's question: tonight finger_paint was slow because one untyped index `at = row * w + col` (w a
`global w: int`) took the general path (boxed number, grown_product), 229 ms against 18 ms. What if fast types were
contagious: everything a fast int touches becomes fast, unless the program asks for exact numbers explicitly? Which
tests would need adapting?

Short answer: there are two ways to read "contagious", and they come out very differently.
- **Flow contagion** (a fast value stays fast as it moves: assignment to an untyped local or global, `#xs`, range
  counters, int globals, calls declared int) is cheap, fixes the finger_paint pattern (766 → 23 ms on the probe below)
  and changed **no test expectation** in the 2,799 test lines I ran.
- **Strong contagion** (also `int op nat → int`: mixing a fast int with any whole number makes it fast) breaks exact
  arithmetic quietly where an accumulator starting at a literal meets a loop counter: `24!` gives
  -7835185981329244160, `from_base("ffff…", 16)` gives -1. Its only test casualty is `24!`, but the failures are
  silent wrong values in lib/ code, the kind of plausible wrong value the project rules call the worst bug.

Recommendation: flow contagion, written into decision nat-and-int as the rule that a fast int keeps being a fast int
wherever its value goes; keep `int op nat → nat`.

## 1. How it works today

Decision nat-and-int (and nat-and-int-core): every whole-number literal is a `nat`, unbounded; `int` is wasm's i64 and
wraps silently. The emitter decides per `+ - *` (core/emitter/operators.warp):

- `arithmetic()`: two Int operands take the runtime's grown words (`grown_sum`, `grown_difference`,
  `grown_product`), which give a boxed $Node that becomes a big int past 2^63, **unless** `wraps(body, left, right)`.
- `wraps()`: yes when both sides are `fast_int`, or one side is a number literal and the other `fast_int`
  (decision fast-int-readings: `xs[i + 1]` stays int), or the function is a runtime word.
- `fast_int(n)`: a symbol that `declared_int` (a local declared `x: int`, an `int?`/`int|…` slot (`boxed_ints`), a
  parameter typed int; a range loop counter is declared int by its lowering), `#xs`, a call of a function whose
  declared result is int (`gives_declared_int`, decision declared-int-result), and `+ - *` of fast ints, recursively.
  Not fast: `/`, `//`, `%`, calls with an undeclared result, and every untyped local or global, whatever it holds.
- `type(x)` follows the same predicate (`type_value_of`): an Int-kind value that is not `fast_int` says `nat`.
- Globals: on main, a `global w: int` is **not** fast. The landing on branch `untyped-local` (d64fe90109, card
  untyped-local, not on main yet) adds `module.int_globals`: a global declared int counts in `declared_int`.
- No analysis pass decides this: analyzer.warp has no growth notion; a local's kind comes from its first value, a
  boxed result of grown_product makes the local a Node, and every later use of it (an index included) runs the
  dynamic path (node_operation, keyed_assignment).

Overflow per lane:

| lane | spelled | `+ - *` past 2^63 |
|---|---|---|
| nat (default) | literals, untyped locals, untyped params, `x: nat` | grows into a big int (boxed, exact) |
| int / i64 / int64 / fast int | declared, range counters, `#xs`, int results | wraps silently (no trap, no hint) |
| `as int` | conversion | raises "too big for int" (decision cast-checks); `as! int` wraps |
| byte, int8..int32 | FFI widths | trap (D16) |
| float | `x: float`, a float literal | IEEE |
| `/` of whole numbers | exact | a rational, never fast |

The finger_paint case: `row = cell(y)` — cell is declared `: int`, but `row` is an untyped local, so `row` is not
fast; `canvas_width` was an untyped global; so `row * canvas_width + col` went through grown_product, `at` became a
boxed Node, and `canvas_pixels[at] = …` took the dynamic write. lib/draw.warp now writes `at: int = …` (efd582253e).

## 2. The contagion rule, stated precisely

### Flow contagion (recommended)
A fast int keeps being a fast int wherever its value goes:
1. **Sources** (as today): a declared int local/parameter/field, a range loop counter, `#xs`, a call of a function
   declared to give int (cell, ord, index, find …), plus **a global declared int** (int_globals).
2. **Assignment carries it**: an untyped local or global whose first value is a fast int is a fast int
   (`row = cell(y)`, `n = #xs`, `k = i`). The rule should be "every value assigned to it is fast", proven over the
   function; the probe uses the first assignment only.
3. **Operators**: `+ - *` of two fast ints, or a fast int and a number literal, are fast (as today). Unary minus,
   `abs`, `min`/`max` of fast ints should be fast too (not today, not in the probe).
4. **Mixing with a nat stays nat**: `fast * nat_variable` is exact, as decision nat-and-int-core says (`int op nat →
   nat`). This is the difference to strong contagion and what keeps `factorial`, `from_base`, `fac(25)` exact.
5. **Division**: `/` stays exact (a rational), `//` and `%` of fast ints should give fast ints (both are i64
   operations that cannot overflow except `MIN // -1`); today they are not fast, the probe leaves them so.
6. **Comparisons** give bools; nothing to propagate.
7. **Indexing**: `xs[i]` with a fast `i` is the fast index; the item read from a list of nats is not fast (a list
   element carries no static fastness; a `[int]` list could, later).
8. **Function results**: an untyped function whose every return value is a fast int gives a fast int
   (`f(n: int) := n * 2`). Today only a declared result counts; not in the probe.
9. **Loop counters**: `for i in 1 to n` is already a declared int; `for x in xs` over a list is not (the items are
   whatever the list holds).
10. **Opt-out**: `x: nat = …` (and `rational`, `real`, `any`) declares an exact slot; a nat-typed parameter or local
    never becomes fast, and assigning a fast value to it boxes it. No new spelling is needed.

### Strong contagion (the literal reading of the question)
As flow, plus: any `+ - *` with one fast operand and the other an Int-kind whole number wraps. An exact operand only
stays exact when declared `nat`/`rational` or held boxed. Consequence: an accumulator `out = 1` (a nat) times a loop
counter becomes fast and wraps.

## 3. Test overview

### Method
A prototype compiler in /Volumes/Worktrees/fast-types-probe (origin/main + the untyped-local int_globals landing +
~12 lines: `WARP_FAST_CONTAGION=flow|strong` read at compile time; flow = rule 2 for untyped locals and main-level/
`global` variables; strong = flow + rule "either side fast"). Same compiler for all three modes, so every
difference is the rule. Full suite not run (fleet rule); instead:
- 579 test files whose text mentions a fast source or an exactness topic (`: int`, `int(`, `i64`, `wrap`,
  `type(`, `nat`, `factorial`, `*=`, range loops, numbers ≥ 2^63).
- control/ (96 files) run whole; numbers/, operators/ (35 files) run whole; 62 more files of numbers, operators,
  modules, lists, programs run whole; the rest (~270 files) with only their lines naming a fast source kept.
- 2,799 test lines in today's mode and in strong mode; flow mode on the files that differed (and the numbers files
  that pin wrapping and growth: test_nat_and_int, test_unbounded_int, test_int_global_wraps), with the same result as today.

### Result

| folder | lines run | changed by flow | changed by strong | current → contagious |
|---|---|---|---|---|
| operators | test_factorial (3) | 0 | 1 | test_factorial.pairs.warp:2 `24!` 620448401733239439360000 → -7835185981329244160 (lib/math factorial: `out = 1; out = out * i`) |
| numbers, control, functions, types, lists, modules, programs, text, parser, welcoming, web, … | 2,796 | 0 | 0 | — |
| **total** | **2,799** | **0** | **1** | |

Lines that fail today fail identically in all modes (about 650 lines across the three runs: outside-the-core forms, missing
words; not related to this question).

Why so few: the tests that pin wrapping already use declared ints (test_fast_int, test_nat_and_int,
test_int_global_wraps, test_fixed_width_ints), and the tests that pin growth (test_unbounded_int,
test_boxed_int_growth, test_big_int_*) start from literals and untyped values, which neither rule makes fast. No test
asks `type(x)` of an untyped copy of an int, and none multiplies `#xs` or a range counter copied into a local past
2^63.

### What contagion changes that no test pins (probe outputs: today / flow / strong)

| code | today | flow | strong |
|---|---|---|---|
| `def f(n: int) { x = n; type(x) }; f(3)` | nat | int | int |
| `def f(n: int) { x = n; x * 4611686018427387904 }; f(4)` | 18446744073709551616 | 0 | 0 |
| `xs = [1, 2, 3]; n = #xs; n * 4611686018427387904` | 13835058055282163712 | -4611686018427387904 | same |
| `for i in 3 to 3 { k = i; print k * 4611686018427387904 }` | 13835058055282163712 | -4611686018427387904 | same |
| `def f(n: int) { m = 4611686018427387904; n * m }; f(4)` | 18446744073709551616 | same | 0 |
| `24!` | 620448401733239439360000 | same | -7835185981329244160 |
| `from_base("ffffffffffffffffffff", 16)` | 1208925819614629174706175 | same | -1 |
| `use math; choose(100, 50)`, `mod_pow(3, 1000003, 1000000000039)` | exact | same | same (their factors are not fast: `min(…)`, `%`) |

Flow's changes are all of one kind: a value that came from an int (a count, an index, a declared int) wraps where it
used to grow. That is the meaning decision nat-and-int gives `int` ("not expected to grow beyond a quadrillion"),
so these are wanted, and a test pinning one would be a new test, not an adapted one.

Strong's changes are silent wrong values in exact code that never mentioned int: lib/math.warp factorial (117),
bit_count (162), prelude from_base (334), and every user loop `p = 1; for i in 1 to n { p *= i }`. Each would need
`out: nat = 1` to opt out, i.e. the opt-out lands on the code that wanted the default.

## 4. Performance upside

Probe (scratchpad paint_bench.warp, the dot() of lib/draw.warp before efd582253e: `col = cell(x); row = cell(y);
at = row * w + col; pixels[at] = color` with `global w: int`, 1,024,000 dots):

| mode | ms |
|---|---|
| today (+ int_globals) | 766 |
| flow | 23 |
| strong | 24 |

33× on the pattern the user hit; strong adds nothing over flow there. finger_paint itself is already fixed by hand
(`at: int`), so it would not speed up further; contagion makes the hand annotation unnecessary.

Samples timed whole (`warp <sample>`, wall, outputs identical in all modes): mandelbrot 0.62 / 0.59 / 0.59 s,
golf collatz, prime-numbers, pascals-triangle, catalan-numbers, niven-numbers, divisors all ~1.0 s in every mode
(within noise, compile dominated). They do not benefit because their hot loops have no fast source: mandelbrot's
`iter = 0; iter = iter + 1` starts at a literal, so neither rule makes it fast. That pattern needs the other half of
nat-and-int-core, "values proven small stay i64" (an interval proof: `iter < max_iter` bounds it), not contagion.
No raytracer sample exists.

## 5. Recommendation and risks

Adopt **flow contagion** (rules 1–10 above), as a refinement of decision nat-and-int: "a value computed from ints
stays an int wherever it goes, unless declared nat".
- Core cost: small. The probe is ~12 lines in core/emitter (emitter.warp assignment hook, operators.warp
  declared_int) plus the int_globals landing. The proper version wants "every assignment fast" instead of "first
  assignment fast" (a local first given `#xs`, later a big nat, must stay nat or widen), and fast results of untyped
  functions; that is a small analysis, best in analyzer.warp's local kinds rather than a new pass. Needs the user's
  core OK (operators.warp/emitter.warp are core).
- Tests to adapt: none found. New tests to add: the four flow rows of the table above.
- Risks:
  - A value from `#xs`, an index or a range counter copied into an untyped local now wraps past 2^63 instead of
    growing. Realistic programs never get there with counts; a program computing `n * huge` from a count gets a
    wrapped value silently (no trap by decision). Mitigation: the compiler's known-in-advance warning (cast-checks)
    for a fast product with a literal ≥ 2^32.
  - `type(x)` of an untyped copy says int instead of nat: visible, harmless.
  - First-assignment rule is order dependent in the probe; the real rule must look at all assignments.
- Do **not** adopt strong contagion: it inverts nat-and-int-core's `int op nat → nat`, silently breaks lib/math
  factorial, bit_count, from_base and the accumulator idiom, and gains nothing measurable over flow on the probe.

Prototype (branch deleted after this report; on top of d64fe90109's int_globals). The whole rule, as probed:
```warp
# emitter.warp: Body gets `fast_locals: map = {}`; before `if operator in assignment_operators …`:
if operator == Op.Assign and left is symbol and contagion() != "" { caught_fast(body, name_of(left), right) }
contagion(): text := env("WARP_FAST_CONTAGION") ?? ""
def caught_fast(body: Body, name: text, value: Node) {
	if name in body.declared or not fast_int(body, value) { return }
	if is_global(body, name) or (name in module.shared and body.function.name == "main") {
		if global_key(body, name) not in module.globals { module.int_globals[global_key(body, name)] = yes }
	} else if name not in body.names { body.fast_locals[name] = yes }
}
# operators.warp: declared_int also counts `name in body.fast_locals`; strong only:
wraps(…) := … or (contagion() == "strong" and (fast_int(body, left) or fast_int(body, right))) or …
```
