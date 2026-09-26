# TODO

- Unbounded Int: Warp Int is currently a wrapping i64, so `square(3037000500)` is `-9223372036709301616` and `law square(x) >= 0` is false. DESIGN.md "Exact numbers by default" wants Int to be mathematical: promote to bignum on overflow (or trap) and treat i64 as an inferred representation choice (`@i64`). Once that lands, `law::lean` can export Int as Lean's unbounded `Int` again, and properties like `x*x >= 0` become provable. See notes/laws.md.
