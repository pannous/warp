# Generators (card generators-function, user 2026-10-09: high priority)

## What works (src/lowering/generators.rs, after ruby_blocks)
- A function whose body yields is a generator: `count_to(n) := { k = 1; while k <= n { yield k; k += 1 } }`,
  Python `def squares(n):\n    for i in range(n):\n        yield i * i`. A function some call passes a block to
  stays a Ruby block function (ruby_blocks.rs runs first and takes it).
- `for x in g(args) { body }`, `for g(args) { … it … }`, `for f in fib` (no parameters): lazy. The generator's body
  is inlined (parameters and locals renamed `g·k·1`), each `yield v` becomes `x = v; body`. Endless generators work:
  `break` sets the stop flag `g·stop·1` and leaves the loop, every generator loop that may stop is followed by
  `if g·stop·1 { break }`, the whole body runs inside a loop run once (`while 1 { …; break }`). `continue` leaves a
  run-once loop around the body, so the generator goes on after its yield. `return` in the generator ends the loop.
- Any other use of the call collects: `g·yielded = []; … g·yielded += [v] …; g·yielded`, `return` returns the list so
  far. `count_to(3)` is [1 2 3], `sum(count_to(4))` is 10. `yield a, b` yields the list [a b].
- Not inlined (collected instead): a recursive generator, one with `global`.
- Fixed on the way: a loop whose body ends in `break`/`continue` (`while 1 { break }`) trapped "null reference": its
  value was taken as a reference the body never set (control_flow.rs ends_in_reference, loop_control.rs ends_in_jump).

## Next
- Iterator objects: `it = iter(g(args))` / `next(it)` resumable at any point, zip of two generators, `take 5 of
  naturals()`. Needs the generator as a state machine (a struct of its locals plus a state index; each yield a
  state) or wasm stack switching (wasmtime 49 has `wasm_stack_switching`, x86-64 Linux only; no browser): the state
  machine works everywhere.
- Iterator protocol for classes: a class with `next()` returning ø at the end is walked by `for`.
- `yield from xs` / `yield each xs`, a recursive generator lazily, a generator expression `(x*x for x in xs)` as a
  lazy value.
- Ruby `loop do … end` and `while c … end` inside a `def … end` do not parse (found writing a Ruby fib generator).
