# Parked natural phrases

The dedicated natural-phrase passes were removed by card `cleanup-park-phrases`. Definitions written with `to` still
produce ordinary functions, but calls use the explicit function form so definitions and calls have one shape:

```warp
to move piece from a to b: piece + a + b
move(1, 2, 3)
```

List filtering, keyed sorting, and slicing use the general operations instead of phrase-specific rewrites:

```warp
positive = xs where it > 0
ordered = xs.sort_by(it => abs(it))
first_two = xs.slice(0, 2)
middle = xs#(2…4)
```

The parked tests remain ignored with the shared reason that preposition parameters may later provide the feature
without three whole-program lowering passes.

Preposition parameters are built (notes/function_calls.md): `to move piece from a to b: …` is
`move(piece, from a, to b)` and is called `move 1 from: 2 to: 3`, as any `f(x, for y)` is called `f 1 for: 2`.
The list phrases (`sorted by abs`, `take first 2`, `keep only positive`) and type-word heads stay parked: they are
method and library-word phrases, not prepositions of a function the program declares.
