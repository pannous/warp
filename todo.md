# TODO

- DONE: Unbounded Int promotes to bignum on overflow, and `law::lean` exports Warp Int as Lean's unbounded `Int`; `square(3037000500)` is `9223372037000250000` and `law square(x) >= 0` is provable. Explicit `as i64` values still wrap and need a future `BitVec 64` proof model. See notes/laws.md.
- `x << 1` and `x >> 1` are not shift operators: `<<` parses as `<` followed by an angle-bracket group (`x<(1)`), so `2 << 1` evaluates to 1 and `f(x:float) := x << 1` reports an undefined variable. Either add shift operators (exact Ints only, a float operand is the float-in-exact-context error) or make the parser reject `<<`/`>>`. Found in A5c.
