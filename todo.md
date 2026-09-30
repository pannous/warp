# TODO

- DONE: Unbounded Int promotes to bignum on overflow, and `law::lean` exports Warp Int as Lean's unbounded `Int`; `square(3037000500)` is `9223372037000250000` and `law square(x) >= 0` is provable. Explicit `as i64` values still wrap and need a future `BitVec 64` proof model. See notes/laws.md.
