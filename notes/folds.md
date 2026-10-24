# Folds: sum is a function like any other (card sum-plain-fold)

User 2026-10-22: sum is just a function, done in warp, the pure clean way without special cases; the Rust compiler
is frozen legacy and only carries `// ⚠ WARP DOES THIS DIFFERENTLY` banners at its sum hacks (19 sites).

## What warp does (lib/prelude.warp, lib/list.warp, lib/compiler/lambdas.warp)

- `sum(xs) := fold(xs, 0, +)`, `product(xs) := fold(xs, 1, *)`: plain definitions. A program's own `sum` replaces the
  prelude's (prelude reach brings only undefined words), and its range rules with it (rewrites.warp skips a module rule
  whose words the program binds).
- `operator_laws` in the prelude: +, *, and, or, max, min are associative; the identity each fold starts from. Data,
  read by nobody yet: the consumers to come are parallel reduction and regrouping (see Open).
- A range's sum is a rewrite rule in warp: `rewrite sum($first to $last) => (if $last < $first then 0 else …)` (and
  `..`), Gauss's closed form with the even factor halved first. `sum(1 to 3000000000)` is instant. The template names
  no helper word (card after-sum): it lands in the program's code, so a named helper (the former `range_sum`) meant the
  program's own word of that name, and `range_sum(a, b) := sum(a..b)` called itself forever (Rust's rewrites.rs has no
  fresh names). A lambda applied to the bounds would evaluate each once, but the pure emitter refuses applying a
  lambda group, so the bounds are written out; Rust's lazy_ranges.rs repeats them the same way.
- lambdas.warp, no new pass:
  - an operator is a fold's function: a bare `+` / `*` (the parser's key with neither side) or the words `and`,
    `or`, `xor` (operator_function): the round is `acc op item`.
  - map-fold fusion for every iteration word (each_of): `for item in xs.map(f) {…}` loops over xs with
    `item = f(source)`, the mapped list never built. `sum(xs .* 2)` is one loop.
  - a function whose body is one iteration over its first parameter, each parameter read once (iteration_function),
    is inlined at its calls when the other arguments are steady (a name, literal, lambda or operator): the loop runs
    where it is called and fuses with the list given. By shape, never by name, so a program's own
    `total(xs) := fold(xs, 0, +)` gets the same.
- min_max.warp no longer folds sum (min/max as folds: card extrema-plain-fold).

## Open (cards)

- The pure emitter has no ints beyond 64 bits: `sum(1 to 10000000000)` (50000000005000000000) is refused there, the
  Rust lane prints it.
- `xs .* ys` of two lists is not ported to broadcasting.warp (it maps `e * ys`), so the dot product `sum(xs .* ys)`
  waits for it; the fusion then applies as is once it is a map over pairs.
- Typed-list fast path and parallel reduction from operator_laws: the pure emitter has neither typed arrays nor
  threads yet.
- The Rust lane still expands its own sum: a program's own `sum` loses there (card rust-lane-program-2), `fold(xs,
  no, or)` is "undefined function: or" there, and WARP_SELF_HOSTED=1 (Rust's lambdas) inherits that.
