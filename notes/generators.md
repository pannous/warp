# Generators: dropped (decision cleanup-cuts, user 2026-10-13)

Generators and `yield` (generators.rs, generator_objects.rs, generator_consumers.rs, generator_expressions.rs, about
1200 lines) were removed on branch cleanup-drop-generators. The last version is in git history before that merge.
Run-time closures and plain loops cover what they did; their tests are ignored with
`cleanup-edge: cleanup-drop-generators`.

Kept:
- Iterator objects: `for x in c` of an object whose class has `next()` walks it until ø (for_loop.rs,
  `lower_iterators`).
- Ruby blocks: `yield v` in a function some call passes a block to (blocks.rs lower_trailing_blocks).
- Comprehensions `[x * x for x in xs]`, and Python's generator argument `sum(x * x for x in xs)` (comprehensions.rs).
