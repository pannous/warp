# index and position

An **index** counts from 0 and belongs in brackets: `xs[i]`. A **position** counts from 1 and belongs after the
hash: `xs#p`. Both are ints and print as plain numbers. What differs is which end the value counts from, and the
compiler keeps track of that.

| gives / takes | counts as |
|---|---|
| `xs.index_of(x)` | a position (0 when x is missing) |
| `argmax(xs)`, `argmin(xs)` | an index |
| `xs.insert(v, at: …)` | takes an index |
| `p: position`, `i: index` | declared (checked as `int`) |
| a name assigned only positions (only indexes) | the same |

When a value meets the other kind's brackets, it converts:

```warp
xs = [10 20 30]
xs[xs.index_of(20)]               // 20: the position meets [ ], read as xs#2
xs#(argmax(xs))                   // 30: the index meets #, read as xs[2]
xs.insert(15, at: xs.index_of(20)) // [10 15 20 30]
```

Rules:

- **Convert, don't refuse.** A position inside `[ ]` gives what the programmer obviously meant. It is not an error.
- **A missing element is an error.** `xs[xs.index_of(99)]` reads `xs#0`, which gives "index out of range: 0 not in
  1…3". It never wraps to -1.
- **Arithmetic keeps the dimension** (like size_t or a pointer): position ± int stays a position,
  index ± int stays an index, so `xs[xs.index_of(10) + 1]` is the next item. position − position is a plain count
  (`q - p`). Adding two positions or indexes is an error. A mixed sum (position + index) is an error too.
- **Names live in scopes.** Each function is its own scope. A parameter or a loop variable rebinds a name as a plain
  int. A name assigned both a position and something else counts as neither.
- **Program names take priority.** If a program binds `index` or `position` as a name, that word is no type word.

Code: src/lowering/collections/index_positions.rs (`mark_declarations` runs before the declaration passes; `lower` runs after
library_words names index_of `collection_position`). Tests: tests/types/test_index_position.rs.
