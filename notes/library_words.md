# Library words: `.word`, `word(x)`, `word x`

`src/lowering/words/library_words.rs` lowers the three spellings to one call `word(x, args)`; `src/wasm_emitter/reflection/library_ops.rs` holds the
runtime functions. A user function or a variable of the same name wins (the word is not lowered).

| Word (aliases) | Args | How |
|---|---|---|
| `first`, `last`, `sum` | 1 | expanded to source in `library_words.rs` (`x#1`, `x#(count(x))`, a `for` fold) |
| `reverse`, `sort` | 1 | `list_reverse`, `list_sort`; `reverse` of a text reverses its graphemes |
| `upper` (`uppercase`), `lower` (`lowercase`) | 1 | `text_upper`, `text_lower`: Latin, Greek and Cyrillic case (wasm_emitter/text/text_unicode.rs) |
| `split` | 2 | `text_split`: the pieces share the bytes of the text; an empty separator is an error |
| `join` | 2 | `list_join` |

An unknown `.word` on a name, a text, a number or a square list is `undefined function: word`. A space before `[`
never indexes: `first [10, 5]` and `reduce [7] (a b)->a+b` pass the list (tests/parser/test_surface_syntax.rs).
`x.upper!` (in-place, inventions.md) is not implemented: `x.upper` returns a new text, value semantics.

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
- On a list of instances a field read maps (broadcasting.rs `element_fields`): `people's name`,
  `people.name`, `name of people` are `people.map(p => p.name)`. The shapes come from InstanceTypes over the program as
  type_constructor will construct it, since broadcasting runs before it; a list word (`people.count`) stays the list's.

## Map words

One map path for every spelling (tests/welcoming/test_welcoming_maps.rs):
- Keys compare by name at runtime: `map_key_name` turns a text or a character (`"A"` is a Codepoint) into the symbol of the
  same letters, used by `map_entry_has_key` and `field_with`. So `{"A":1}`, `{A:1}`, `m["A"]`, `m.A`, `k="A"; m[k]` are one key.
- `{}` is the empty object: `d["A"]=5` grows it (`field_with` on ø returns the entry), `d[k]=v` with a variable key too.
- Words (runtime functions named like the canonical word): `m.keys()`→`map_keys` (keys read as texts), `m.values()`→`map_values`,
  `xs.has/contains/includes(x)` and `x in xs`→`collection_contains` (a map's keys, a list's elements), `m.get(k[, default])`
  →`map_get_or` (ø by default). `m["Z"]` stays the error `no field Z` (Python's KeyError).
- `for k in m` walks the keys of a known map (`library_words::for_over_map` → `for k in map_keys(m)`); `for k, v in m` and
  `for (k, v) in m` add `v = m[k]`. Any other destructuring loop (`for (r, c) in pairs`) binds `r = e#1; c = e#2` of
  `map_entries(xs)` (`for_loop::destructuring_loop`); a pair `k:v` indexes as k #1 and v #2.
- Typing: a `{k:v}` literal is a `map of <value type>` (`[]`/ø values fit any list type); values of mixed type, a lookup by a
  name in a map of unknown values (a parameter) and anything indexed out of such a value are `Kind::Empty`, held as a Node;
  `d[k]`/`d[k]=v` with such a key dispatches at runtime (`node_at_key`, `node_with_key`: an Int indexes, else a key).
  A parameter read as a map (`g.keys()`) is a List. Text + such a Node concatenates.

## Field assignment and `?.`

- `p.a = v` and `p["a"] = v` lower to `p = field_with(p, "a", v)`; `p.b.c = 4` to `p = field_with(p, "b", field_with(p.b, "c", 4))`.
  `field_with` (runtime function, also callable) changes the object in place and gives it back: objects are references
  (P200/P200b), `q=p; p.a=9; q.a` is 9 for maps and instances. A new name is added at the end, in front of meta entries. Only a variable that is an object (assigned an object literal, or another such variable) can be assigned to; any
  other target is `undefined function: a` at the lowering.
- `a?.name` is `(t=a; if t == ø then ø else t.name)` (`Op::SafeDot`, lexed only before a letter, so `x ?.5 : 1` stays a ternary);
  on a non-ø receiver that is no object the miss is `no field name`.
- `count {a:1}` and `{a:1}.length` are 1: a key:value node counts as one item.
- Nested objects are square lists in memory and print as `{a:1 b:[c:3]}` (existing: emit_default_key converts the inner braces).

# Typed and fixed arrays

`x : 100 int`, `x : 100 * int`, `pixel:int[100]`, `letters = char[3]`, `upcases = 26 * char` (also `x = int[4]`) are zero-filled
lists (`filled_lists::typed_array_value`, `zero_list`). `x:[number]` is `x:list of number`, `numbers x = [1 2]` declares a list
(a plural type word before a name is a declaration). The bracket spelling `[number]` hints the plural `numbers`
(`ListTypeStyle::Bracket`); the fixed-array spellings have no canonical form yet (both are decided as valid), so they get no hint.
Setting past the end is `index out of range` (P59).


# Data as scope (Decided #6)

`resolve_data_scope` (analyzer.rs): a symbol that is no variable reads the data key of that name given earlier in the same block
(`a-b:2 c-d:4 a-b` is 2; inside an object literal too: `{c:4 b:c}` is `{c:4 b:4}`, also when the literal is assigned). A key wins over
the subtraction of its parts, with the lint warning "`a-b` is a data key here, but a and b are also variables". Without a key, a hyphenated
name whose parts are all variables (assigned names and function parameters) is the difference: `a=5; b=1; a-b` is 4, `a-b-c` chains.
Any other hyphenated name stays a symbol. test_hyphen_units stays ignored: it needs `==` on ranges and values with tolerance.

# Type tests

`src/lowering/types/type_tests.rs` (first lowering pass): a type phrase after `is` (or `==`) turns equality into `is_type(x, "spec")`:
`3 is int`, `x is a number` (a/an optional), `[1 2] is list of int`, `[1 2] is ints`, `π is real`. The emitter answers it statically from the
name `type(x)` reports (`static_type_name`); `type_matches`: `number` covers int, rational, real, float; `real` covers int, rational,
real; `rational` covers int; `list of number` covers every list of numbers; `text` also accepts a codepoint (a one-character string).
A variable or user function named like the type word (`int=3; 3 is int`) keeps equality; `x is y` stays equality. `x is number 9` tests
type and value; with x defined nowhere it is an error teaching `x be number 9` (P61). `type of x` is `type(x)` (the parser reads `type of` as a declaration head). `x as number = 9` is `x:number=9`.
Static means `x=f(); x is int` uses the compile-time kind of x, like `type(x)`.

# Functions as values

Functions are first-class values (closures: notes/closures.md). `map`, `filter`, `reduce`, `fold`, `each` and the
other iteration words (`lambdas::ITERATIONS`, one loop template per word) inline a literal block, lambda or known
function as a loop: `filter xs {it>2}`, `reduce xs (a b)->a+b` (first element is the start; an empty list is the
error `reduce of an empty list`), `fold xs 10 (a b)->a+b`, `each xs {it*10}` (the value is the body of the last
item). An operator is a function value (`fold [1 2 3] 0 +`, `reduce xs *`); `sum := fold +` defines a function of the
list. `&name` is glued to the name (`a &b` is `a (&b)`, `a & b` and `a&b` stay `and`). Body `a-b` of a lambda with
parameters a and b is the difference (kebab rule). Write `it < k`, not `{it<k}`, which reads as a type application.
A user function named like a type word (`double`) is read as the type cast.

## Python's list, dict and tuple words (tests/lists/test_python_collections.rs)

Aliases of warp's own words: `reversed` → reverse, `tuple(xs)` → list, `len` → count (with its hint), `xs.index(x)` (lib/list.warp:
the position from 0, a `ValueError` raise when absent; `index_of` stays the 1-based one), `xs.extend(ys)` (appends in place),
`m.items()` → entries, `m.update(n)` → merge (in place), `m.setdefault(k, d)`, `dict(pairs)` (prelude). Already warp words:
append, remove, pop(), count, copy, keys, values, get, sorted, any, all, sum, min, max.
Not aliased, on purpose or open:
- `xs.sort()` / `xs.reverse()` stay copies with the unused-value warning; warp's in-place form is `xs.sort!` (wiki/mutable.md).
- `xs.insert(i, v)` is the ambiguity error naming both readings (decided earlier).
- `x.pop(k)` (0-based position of a list, -1 the last; key of a map) and `x.clear()` of a variable lower
  in list_mutations (removed_key, cleared_collection) to prelude words (popped_item/popped_rest, cleared), which
  dispatch with `x is map` at run time; library_words::LOWERED_TO_PRELUDE makes modules load them. `m.popitem()` (lib/map.warp),
  `fromkeys(ks, v)` (prelude), `dict.fromkeys(ks, v)` (library_words method_call drops the `dict` receiver).
- pop, pop(i) and clear() change the list in place like append/remove: pop(i) and clear() wrap the
  fresh rest in `list_become(xs, …)` (list_ops emit_list_become); an alias group changed in place is no typed array
  (list_dispatch find_typed_lists), whose Node copy would part from the alias. A map of one entry passed to a function is its lone key at run time (`type` says key).
- `x is list` of a value of unknown type is no for a map (a curly list at run time); lib/markup.warp
  asks html_is_group (list or map) where a block `{…}` and a list `[…]` both hold items.
