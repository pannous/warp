# Class places (decisions class-types-checked, bare-number-to-unit)

A place typed by a class the program declares, a parameter `f(d: Length)` or a variable `d: Length = v`, takes:
- a value of the class or of a class extending it (by name: `f(5s)` is refused, Time does not extend Length);
- for a class extending none, also an instance or map with every one of its fields (`a: A = B(1, 2)` when B has x);
- a value a conversion to the class makes (`f(5)` is 5m by units' `n: number as Length`).
Any other value is refused: `f takes a Length, not 5s (a Time)`, `d takes a Length, …` for a variable.

Mechanism: extensions/conversions.warp, no core. Each call of a function defined once with such a parameter passes
the argument through `passed·T(value, "f")` (argument_conversion), each `x: T = v` its value so. A value known to
be a T at compile time skips it: a parameter typed T (or a subclass) of the enclosing definition, a constructor
call `T(…)`. Optional class places (`p: Point? = ø`) and overloaded functions (dispatch tests them) are not wrapped.
`place_refused`, `has_fields` and `with_article` are prelude words.
