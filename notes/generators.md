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

- Iterator objects (generators.rs `lower_iterators`, before class_methods): `for x in c` of `c = Countdown(3)` or
  `for x in Countdown(3)`, a class with a method `next()` that gives ø at the end, is `x·iterator = c; x =
  x·iterator.next(); while x != ø { body; x = x·iterator.next() }` (the last a marked step: `continue` runs it).
  Only a variable assigned a constructor call of such a class, or the call itself, is known as one so far.
- Fixed on the way: a method that changes its object and returns early (`if n <= 0 { return 0 }; n -= 1; n`) gave
  "index out of range": its early `return v` now gives the pair `[v, self]` (`return self` for a method giving
  its object), class_methods.rs `with_returns`.

## Generator objects (src/lowering/generator_objects.rs, first step of generators::lower)
- `counter = count_to(3)` of a variable some `next(counter)` or `counter.next()` advances, and `iter(count_to(3))`:
  a resumable object, `next` gives the next yielded value, ø once the generator ended (Python's generator object,
  without StopIteration). Two objects of one generator advance apart (`f = fib(); g = fib()`).
- The generator becomes the class `count_to·generator`: parameters and locals are `any` fields, plus
  `generator·state`; `next()` is a state machine `while 1 { if generator·state == 0 {…}; …; return ø }`, cut at the
  yields. A while, if/else or for (lowered to its while, its step a state of its own so `continue` runs it) that
  yields, returns, breaks or continues inside becomes states and jumps; any other statement stays as it is. The
  class goes first in the program, then lower_iterators and class_methods::lower run once more for it.
- Chosen without asking (undoable): a plain call still collects, the object is made only for a variable advanced
  with next or by `iter(…)`; Python makes every call an object.
- Not yet: a yield inside an expression (`x = yield v`, Python's send) leaves the call collecting; each field
  update copies the object (`field_with`), fine for a few fields.

## take, zip, list, sum (src/lowering/generator_consumers.rs, first step of generator_objects::lower)
- `take(naturals(), 3)`, `take 3 of naturals()`, `first 2 of g()`, `first(g(), 2)` and `zip(naturals(), xs)` pull only
  the values they need; `n = naturals(); take(n, 2)` takes from the object and leaves it advanced (a later take goes
  on); `list(c)`, `sum(c)` (count max min mean sort) of a variable advanced with next take the rest of its values.
- Each call becomes statements before its statement: `take·1 = []; while count(take·1) < limit { v = source.next();
  if v == ø { break }; take·1 += [v] }`, the call replaced by `take·1`. A source is a generator call (made an object
  with `iter(…)`), a variable holding one, or any list (pulled by index). Calls with no generator source stay as they
  are (lib/list.warp, `use list`). A call in a `while` condition is not rewritten (it would be computed once).
- An argument `naturals()` arrives as the bare symbol `naturals`: generator_call takes it as the call.

## Next
- wasm stack switching (wasmtime 49 has `wasm_stack_switching`, x86-64 Linux only; no browser) would resume any
  generator without the state machine; not needed now.
- `yield from xs` / `yield each xs`, a recursive generator lazily, a generator expression `(x*x for x in xs)` as a
  lazy value.
- Ruby `loop do … end` and `while c … end` inside a `def … end` do not parse (found writing a Ruby fib generator).
