# Word slices (card word-slices, src/lowering/word_slices.rs)

`xs from #2 to #4`, `xs starting from second`, `xs up to 2nd`, `xs from (n)th to last`, `xs up to nth`:
positions like `xs#1` (first = 1), both ends inclusive, lowered to the parser's `slice(xs, start0, end_exclusive)`.

- A bare number (`xs up to 2`) is a loud error naming `#2` and `second`: 2 may be an item of the list.
- Only a program variable, a list literal or a text is sliced: `count from 1 to 10` and phrase calls
  `move x from a to b` are untouched.
- The parser nests the words (`xs (to (# from 2) (# ø 4))`, `from#2` is one index); `flat_words` reads them flat.
  Inside a call (`print(xs up to #3)`, `f(xs up to #3)` = `[f, xs, (to up #3)]`) the argument words are sliced.
- Not built: `xs to #2` as an alias of `up to`, it clashes with the range `1 to #xs`.
- The pass runs before list_phrases (src/pipeline.rs SOURCE_PASSES).
