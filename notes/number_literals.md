# Number literals: decimals are floats (decision exact-default), formerly why `[0.0, 0.0]` was a list of int

Cards int-list (this one), int-exact, float-calls, float-nodes, int-declaration, exact-reals.

## History

- 2026-09-27 `2b74cd0f8` "exact rationals by default": a decimal with up to 15 significant digits is exact
  (`Number::is_exact_decimal`), so `0.1 + 0.2 == 0.3` is yes. f64 only comes from `1.5f`, `as float`, √, π-free
  irrational results and FFI (notes/footguns.md).
- P196b (notes/decisions.md): "`1.0` is the exact int 1", so `1 === 1.0` is yes. A whole decimal is the integer.
- 2026-09-30 `8bb316185`: decimal literals in a list are exact like everywhere else.
- 2026-09-30 `943f0f8b4`: type names: `int` (also `2.0`), `rational` (exact fractions and decimals), `real` (π),
  `float` (approximations).
- notes/typed_lists.md "Open": a list variable of exact ints becomes an `(array (mut i64))`; float arrays came second
  because few literal lists are float.

- 2026-10-10 decision exact-default (notes/decisions.md): a decimal literal is an f64 float, reversing the exact
  decimals and P196b; integer division stays exact. Fraction math was ~90% of finger paint's stroke time.

Today:

```warp
type(0.0)          # float
type(1.5)          # float
type(3/2)          # rational
type([0.0, 0.0])   # list of float
0.1 + 0.2 ≈ 0.3    # yes; `==` on floats warns, fix: use ≈
```

The rest of this note is the history of the exact decimals.

## The bug (card int-list)

```warp
shown = [0.0, 0.0]
for i in 0..2 { shown[i] = max(sqrt(2.0), shown[i]) }
```

gave "WASM validation failed: expected i64, found f64". The value representation was fine: the analyzer already widens
a list of int written a float to a list of float (`widen_element_type`). But it typed variables in one pass in source
order: `max` lowers to `(a = sqrt(2.0); b = shown#i; a > b ? a : b)`, and `b` was typed from `shown#i` before the write
that widens `shown`, so `b` became an i64 local fed an f64. The same for plain variables:

```warp
x = 0.0; y = 0.0
for i in 0..3 { y = x * 2; x = sqrt(2.0) }   # "x is a float where an exact Int is expected"
```

So the problem is not the literal's type but that a type decided by a later write was not visible to earlier reads.

## Options

(a) **Static join, recommended.** The value stays exact; a variable's type is the join of its literal AND every write
the analyzer can see, so a read anywhere in the program sees the final type (int ⊔ float = float for a list's elements,
an int variable later given a float holds floats throughout). Implemented: `collect_variables` collects again with the
widened types from the start until nothing widens (`Scope::widened_bindings`). Nothing changes for code that worked.

```warp
shown = [0.0, 0.0]                 # list of float, because of the write below
shown[0] = sqrt(2.0)
```

(b) **Runtime widening.** The list stays an i64 array; the first float store converts it to a float (or Node) array.
Needed only where the analyzer cannot see the write (a list passed to a function that writes floats into it). Costs a
check on every store.

(c) **A decimal point means float**, reversing P196b and the exact-decimal default for whole numbers:

```warp
type(0.0)   # float
1 === 1.0   # no
```

Simple to explain, but loses exactness (`0.1 + 0.2 == 0.3`) unless only whole decimals change, which would make `2.0`
float while `2.5` is rational: an odd split.

(a') **Variant of (a): a literal with a point is `rational` statically** (not int) while staying exact, so
`[0.0, 0.0]` is a list of rational and `type(0.0)` is rational. Changes what `type(2.0)` prints and P196b's static side
(`1 === 1.0` compares static types). Not needed for the bug; only a naming question.

## Recommendation

(a), implemented on branch int-list (warp-numbers). (b) as a later fallback for writes through parameters. Ask the user
whether `type(0.0)` should say `rational` (a') or stay `int`.

## int-exact

`int i=3.3*2; i` now gives the declared-type mismatch ("i is declared int, cannot assign … fix: i=int(3.3*2) or declare
i:float"), as `int i=3.5` does: already fixed on main (`c7dc0cba3`, arithmetic on exact literals is checked by its value against
the declared type).
