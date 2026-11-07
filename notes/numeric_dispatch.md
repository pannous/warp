# Numeric dispatch, Julia-style (card numeric-dispatch-julia)

Decision operators-are-functions: `+ - * / ^ % < <= > >= == !=` are ordinary functions, the method chosen by the
run-time types of *both* operands. A number class (Quantity, Interval, Gaussian, Real, Complex, Money, Matrix …)
composes with any other without per-pair glue. The static int/float/text fast paths stay exactly as fast as today.

## Today (2026-11-07)
- emitter/operators.warp `unmarked_operation`: static kinds (int, float, text, list, time) are compiled inline. Only
  when an operand is a boxed Node and a dispatcher `+·operator` exists does it call that dispatcher.
- extensions/class_methods/methods.warp builds one dispatcher per operator, with **single dispatch on the left**
  operand's class. A right-hand instance is reached by the `commutative_operators` swap (`+ * == !=` only, so `5 - q`
  and `5 / q` have no route). Declared conversions (`n: number as Real`) promote the left operand (decision
  as-promotes).
- Every class carries its own per-pair glue inside one untyped method:
  - units.warp `*(other) := if other is Unit … else if other is Quantity … else …`, and the same for `/`
  - uncertain.warp `as_interval(o)` and `as_gaussian(o)`, complex.warp `as_complex(o)`, money.warp `money_same(…)`
  - reals.warp `exact_real(other)`, `alike(other)` and `real_float(other)` branches
  - matrix.warp `if o is number …`
- Fields are typed float or number (Quantity.amount), so a Quantity of an Interval can't exist.

## The method table
One table per operator, gathered program-wide by methods.warp (classes' operator methods plus free definitions
`*(a: number, b: Quantity) := …`):

    row = [left type, right type, lifted function]
    types: a class name, a builtin kind word (int, float, ratio, number, text, list, time), or any (an untyped param)

- A method in a class, `*(other: Unit) := …`, is the row [Quantity, Unit, *·Quantity·Unit]. An untyped `other` is
  [Quantity, any]. Several methods of one operator per class are allowed, told apart by the parameter's type.
- Specificity, as in Julia: row r beats s when each of r's types is the same as or a subtype of s's (class lineage;
  int ⊂ number, float ⊂ number, ratio ⊂ number, every type ⊂ any). The dispatcher tests the rows most specific first.
- **Ambiguity** (neither row beats the other and both match some pair) is refused at compile time, naming the pair
  to declare: `ambiguous a * b of Interval and Quantity: declare *(a: Interval, b: Quantity)`.
- **Promotion** replaces the commutative swap and the as_x helpers. When no row matches (A, B) and a declared
  conversion takes B to A (or A to B), the dispatcher converts and retries: `promote` is the existing conversion
  declaration `n: number as Interval := …` (decision as-promotes), now applied to either side and for every operator.
  Commutativity is no longer guessed: `5 - q` promotes 5 when a conversion exists, else it is refused.
- The fallback (no row, no promotion) is the builtin on two nodes (runtime node_operation), as today.

Generated code: still one dispatcher function per operator, `*·operator(a, b)`, now an if-chain over
`instance_of(a, L) and instance_of(b, R)` in specificity order, then the promotions, then the builtin. A builtin-kind
test is the runtime's kind test (`a is int`), so no new runtime words are needed.

## Fast paths (unchanged, measured)
- The emitter short-circuits before any dispatcher whenever both operand kinds are static and builtin (int, float,
  bool, text, list, time). This is exactly today's code, untouched: hello, loops and the compiler itself never
  reach a dispatcher.
- The dispatcher exists only when the program reaches a class with operator methods (shaken out otherwise), so a
  program without number classes compiles the same bytes.
- Inside a dispatcher the rows come first and the builtin is the last branch. A node holding two plain numbers pays
  the row tests, which is the same cost as today's left-class test chain.

Measure before/after: `hyperfine 'warp samples/hello/hello.warp'` (today 0.15–0.18 s), the compile of
samples/numbers/*, and core size (scripts/vitals.py). Neither may get worse.

## Generic amounts (composition)
- Quantity.amount, Interval.value/low/high, Gaussian.value and Money.minor become `number or any number class`. In
  practice that means an untyped field, a boxed node, so `amount * other.amount` goes through the table: `(2 ± 0.1) m`
  is Quantity(Interval(…)), and `(2 m) ± (0.1 m)` is an Interval of Quantities.
- Cost: unit arithmetic on plain floats becomes boxed. Only classes pay this, never the static paths; measure
  test/units with the same timer.

## What goes away
- units.warp: the `if other is Unit / Quantity / else` chains in `* /` become three typed methods each. The
  Unit-times-number glue becomes the row [Unit, number].
- uncertain.warp `as_interval`, `as_gaussian`; complex.warp `as_complex`: these become conversions
  `x: number as Interval := …`, and the methods take `o: Interval`.
- money.warp money_same's type check: the row [Money, Money], plus the refusal for [Money, number] under `+`.
- reals.warp: the `exact_real` / `alike` / float fallbacks become rows [Real, Real], [Real, int], [Real, float].
- methods.warp: `commutative_operators` and the left-only `promoted()` are replaced by two-sided promotion.

## Phases (each lands on its own, tests green)
1. methods.warp: rows from typed operator methods, a specificity-ordered dispatch on both operands, ambiguity
   refusal and two-sided promotion. Libraries unchanged; every existing untyped method is a [C, any] row, so all of
   today's behaviour stays. Extension only, no core.
2. Convert the libraries one at a time (units, uncertain, complex, money, reals, matrix), deleting their glue.
   Feedback is test/units, test/numbers and each library's tests.
3. Generic amounts, with the new outcome tests `(2 ± 0.1) m * 3` and `2 m ± 10 cm`.
4. Drop the commutative swap. Card leftovers: time arithmetic (emitter time_operation) could become table rows later.

Core: none expected; the emitter already calls `op·operator` for boxed operands.

## Phase 2 carries the prebuilt-module closure (card prebuilt-module-links, 2026-11-07)
Decision no-lib-reads: every module that a prebuilt module links must be prebuilt itself. Today units, math and sound
link reals, and file, hash, text, markup and wagi link path, bits, encoding, html, letter_ranges, slugs, unit_names
and url; none of those is in std_module_names. The closure in core/compiler.warp write_modules is
`for name in in_dependency_order(prebuilt_module_names())`, with:

    prebuilt_module_names(): list := {
        names = std_module_names
        at = 0
        while at < #names {
            for used in used_modules(library_text(names[at] + module_extension)) { if used not in names and library_text(used + module_extension) != "" { names += [used] } }
            at += 1
        }
        names
    }

It builds and self-checks, but prebuilt reals breaks samples/numbers/measurements.warp: `2 * π * √(interval)` fails
with "no field radicand". A linked module's operator dispatcher knows only its own classes, so real's `*` never meets
uncertain's Interval. The per-operator tables baked across modules (Supervisor's condition 2) fix that, and
measurements.warp with prebuilt reals is the proof. Until then, linked_libraries asks exists() first, so the page
skips a missing module as native does.
