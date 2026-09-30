# Uniscript entity data

Used by `lib/uniscript.wasp` (`use uniscript; uniscript("<:fracture A>")` → 𝔄), spec in `wiki/uniscript.md`.

| file | what |
|---|---|
| `entities.wasp` | the human readable source of truth: names → text, block types, block aliases |
| `entities.idx` | binary index built from it, what the wasp code reads |
| `uniscript_index.py` | `seed` (sources → entities.wasp, overwrites edits!), `build` (→ entities.idx), `check` (both agree) |

After editing `entities.wasp` run `python3 data/uniscript/uniscript_index.py build`; `tests/test_uniscript.rs` runs `check`.
The build is deterministic: the same entities.wasp gives the same bytes.

## Sources of the seed

- `names`: Unicode 16 character names from Python's `unicodedata`, lower case with hyphens (`greek-small-letter-alpha`).
  Algorithmic names (CJK unified ideographs, Hangul syllables, Tangut, …) are left out: they only repeat the code point.
- `latex`: command names of `unicode-math-table.tex` (TeX Live), without the backslash (`mfrakA`, `upalpha`, `infty`).
- `html`: the HTML5 entity list (`html.entities.html5`), backwards compatible, discouraged (`dopf`).
- `uniscript`: own names, first priority: `less` `colon` `greater` escape the `<:` marker.
- `blocks`: derived from Unicode names (`MATHEMATICAL FRAKTUR CAPITAL A` → `fracture A`, `MODIFIER LETTER SMALL A` → `upper a`,
  `LATIN SMALL LETTER REVERSED E` → `reversed e`, `LARGE RED CIRCLE` → `red circle`), `greek` from the Greek keyboard layout
  (ELOT 1000: c → ψ, q has no letter), the suffix controls from `fonts/README.md`.

When a name occurs in several sections, the first of uniscript, names, latex, html wins.

## Block control keys

A block entry `operand: text` maps an operand; keys starting with `*` are controls:

| key | meaning | example |
|---|---|---|
| `*suffix` | follows any character without its own entry | `red { "*suffix": "\u{E0072}" }`: `<:red A>` → A U+E0072 |
| `*suffix egyptian` | the same, only after hieroglyphs | `mirror`: U+13440 |
| `*prefix cjk` | goes before the parts of a group whose first part is CJK | `beside`: ⿰, `<:beside 犭 句>` → ⿰犭句 |
| `*infix egyptian` | goes between the parts of a hieroglyph group | `above`: U+13430 |

## Binary format of entities.idx

All integers are u32 little endian; offsets count from the start of the file.

```
0    "USX1"                      magic
4    T                           number of tables (3)
8    T × (records offset, count) one pair per table
…    records                     20 bytes each: hash, key offset, key length, value offset, value length
…    string pool                 UTF-8 keys and values, deduplicated
```

Records of a table are sorted by (hash, key bytes). `hash` is `h = (h * 31 + byte) mod 2^32` over the key's UTF-8 bytes,
so a lookup is a binary search for the first record of the hash followed by a byte comparison of the keys (collisions are allowed).

| table | key | value |
|---|---|---|
| 0 names | a name (`alpha`), a block entry `block operand` (`fracture A`, `red *suffix`), a block itself `block ` (trailing space) | its text (`""` for a block) |
| 1 chars | a single non-ASCII character | its preferred uniscript: own name, well known short name (HTML = LaTeX, or HTML = last word of the Unicode name), block form, Unicode name |
| 2 suffixes | a suffix control (U+E0072, U+FE0F, U+13440) | its block type (`red`, `iconic`, `mirror`) |
