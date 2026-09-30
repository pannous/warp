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

# Property access on objects

`o.a`, `a of o`, `o's a`, `o["a"]` are one lookup: the subscript of `o` by the text key `"a"` (`library_words::field_lookup`),
emitted by `emit_indexed_node` as `map_find`, and on a miss the runtime error `no_field_a` (eval reports `no field a`).
`{a:1 b:2}.a` → 1, `p.b.c` → 3 (a chain of lookups), `name of p`, `c of b of p`, `p's name`, `p["name"]`.

- An object is an object literal, a variable only ever assigned one, or a field of such an object. Its field names win over the
  library and counting words (`p={first:"A" last:"B"}; p.last` is "B"); a word that is no field falls back to them, then to the error.
- The lookup of a chain (`p.b.x` after `p.b`) is treated as an object, so the miss is `no field x`, never `undefined function`.
  A non-object (`x=5; x.foo`) keeps `undefined function: foo`.
- `map_find` never traps: a non-list is a miss; a text key equals the symbol of the same letters; a one-character string is a
  codepoint and is a key only as a literal (`p["x"]`).
- `p[k]` with a symbol `k` keeps the old meaning: the variable `k` if there is one, else the name; its miss is `key not found`.
- The possessive is a parser rule: an identifier directly followed by `'s ` and an identifier (`p's name`).
- Not done: `a in {a:1 b:2}` (the word `in` has no meaning yet), assignment to a field (row 8).
- Quirk found: `{a:1}.length` counts 2 (the single entry is the Key node itself, counted as a pair), `{a:1 b:2}.length` is 2.

## Field assignment and `?.`

- `p.a = v` and `p["a"] = v` lower to `p = field_with(p, "a", v)`; `p.b.c = 4` to `p = field_with(p, "b", field_with(p.b, "c", 4))`.
  `field_with` (runtime function, also callable) copies the object: value semantics, `q=p; p.a=9; q.a` stays. A new name is added
  at the end. Only a variable that is an object (assigned an object literal, or another such variable) can be assigned to; any
  other target is `undefined function: a` at the lowering.
- `a?.name` is `(t=a; if t == ø then ø else t.name)` (`Op::SafeDot`, lexed only before a letter, so `x ?.5 : 1` stays a ternary);
  on a non-ø receiver that is no object the miss is `no field name`. `y = x?.a` with a ø result into a variable still fails
  ("cannot extract a numeric value from ø"): a variable cannot hold the ø of an if-branch yet.
- `count {a:1}` and `{a:1}.length` are 1: a key:value node counts as one item.
- Nested objects are square lists in memory and print as `{a:1 b:[c:3]}` (existing: emit_default_key converts the inner braces).

# Typed and fixed arrays

`x : 100 int`, `x : 100 * int`, `pixel:int[100]`, `letters = char[3]`, `upcases = 26 * char` (also `x = int[4]`) are zero-filled
lists (`analyzer::typed_array_value`, `zero_list`). `x:[number]` is `x:list of number`, `numbers x = [1 2]` declares a list
(a plural type word before a name is a declaration). The bracket spelling `[number]` hints the plural `numbers`
(`ListTypeStyle::Bracket`); the fixed-array spellings have no canonical form yet (both are decided as valid), so they get no hint.

Ignored tests that still cannot pass unedited: test_array_creation (`pixel=[];pixel[1]=15` assigns past the end, Decided an error;
`pixel array`), test_array_initialization_basics (`analyze(parse(..))` on `x : 100 numbers` counts the parse tree, not the lowered list),
test_array_initialization (`x : 100 * ints;[ x.length` is a typo, and `x is array of size 100`, `x is a 100 integer array` are
natural-language forms), test_array_type_generics (expects `list<int>`, decided `list of int`).
