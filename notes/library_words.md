# Library words: `.word`, `word(x)`, `word x`

`src/library_words.rs` lowers the three spellings to one call `word(x, args)`; `src/wasm_emitter/library_ops.rs` holds the
runtime functions. A user function or a variable of the same name wins (the word is not lowered).

| Word (aliases) | Args | How |
|---|---|---|
| `first`, `last`, `sum` | 1 | expanded to source in `library_words.rs` (`x#1`, `x#(count(x))`, a `for` fold) |
| `reverse`, `sort` | 1 | `list_reverse`, `list_sort` (ints only; other elements are an error) |
| `upper` (`uppercase`), `lower` (`lowercase`) | 1 | `text_upper`, `text_lower`: ASCII only, a non-ASCII byte is the error `non ascii text` |
| `split` | 2 | `text_split`: the pieces share the bytes of the text; an empty separator is an error |
| `join` | 2 | `list_join`: texts, ints and ASCII characters |

An unknown `.word` on a name, a text, a number or a square list is `undefined function: word`. `{a:1}.a` (an object literal) stays
data: property access is row 7 of wiki_features.md.

## Known gaps

- `sum` of a list of decimals or floats: the `for` loop over such a list traps (`for i in [1.5 2] {i}` → cast failure). Fix the loop, not `sum`.
- `first [10, 5]` (prefix word, then a comma list) is read as the subscript `first[10, 5]` (a space before `[` still subscripts,
  Decided: `[1] [2]` is a subscript). Write `first([10, 5])`. Lists with spaces (`first [10 5]`) work.
- `upper`/`lower` beyond ASCII need Unicode case tables. `reverse` and `first`/`last` of a text: `first`/`last` work (graphemes), `reverse` of a text is an error.
- `sort` compares ints only (no floats, texts, mixed): the comparison of two Nodes is not a runtime function yet.
- `x.upper!` (in-place, inventions.md) is not implemented: `x.upper` returns a new text, value semantics.
- Words not added because the meaning is unclear: `uppercase!`/`x.upper!` (in-place spelling), `sum := fold +` (needs first-class operators, row 22).
