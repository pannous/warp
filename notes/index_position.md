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

## Texts and declared counts (decision index-units, card index-of-bytes)

A text names the unit it counts: `t.find(part)` is a position in characters (graphemes, from 1, like `t#p`),
`t.byte_index_of(part)` a byte offset (an index from 0); a missing part is an error. A bare `t.index_of(part)` counts
as find does and warns. A declared index or position holds its value counted its way: `i: index = t.find("b")` is one
less than `position p = t.find("b")`, `position top = argmax(xs)` one more than the index (the pure lane,
core/index_positions.warp; Rust keeps the number as given). The unit is part of the declared type too:
`byte index b = t.find("b")` is 9 for "a👍🏽b" and `byte position` 10 (find answers in the declared unit, as
byte_index_of); a byte count given to a character count, or the other way, is an error. The parser reads `byte index`
and `byte position` as one type name (parser.warp COMPOUND_TYPES). `offset` is a synonym of index (`byte offset` of
byte index); lib/ names no parameter `offset`, which would make the word a name in every program reaching it. Still Rust only: a position meeting `[ ]`, an index
meeting `#`, `insert(at:)`, the error on adding two of them.
