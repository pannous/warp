# Pipe composition (card pipe-compose)

wiki/pipe.md: `printSum := sum|print; printSum 1 2 3` prints 6. D6 makes `|` one operator dispatched on its operands:
truth values → or, a value and a function → the call (`2|square|root`), and now a function and a function → their
composition.

- src/lowering/pipes.rs: `f|g` where the left side is a function name (not a variable, a user function with
  parameters, a library word, a spaced definition) or itself such a chain becomes `it => g(f(it))`. Word operators
  count as stages: `square|sqrt` is `it => √(square(it))`.
- Works assigned (`h := sum|print; h 1 2 3`) and called in parentheses (`(sum|print)([1,2])`).
- Not yet: `sum|print 1 2 3` (wiki `f|g x`) parses as `sum | print(1 2 3)`: card pipe-applied.
