# Uniscript in wasp: what it took (lib/uniscript.wasp, data/uniscript/)

`use uniscript; uniscript("<:fracture A>")` → 𝔄, `unicode_to_uniscript("α")` → `<:alpha>`. Data format: data/uniscript/README.md.
Tests: tests/test_uniscript.rs (spec examples, round trip, index check), tests/test_text_bytes.rs, tests/test_text_functions.rs.

## Language features added for it (Rust)
- `text + text`, `text + 'c'`, `'a' + 'b'` concatenate (`text_concat`, a codepoint is UTF-8 encoded by reusing
  `text_with_char_at` on a one-byte placeholder); `s += "x"` lowers to `s = s + "x"`. `text + number` stays a type error.
  An Error operand is the result: errors propagate through concatenation.
- `read(path)` → the file's bytes as Text, or an Error with the reason (host import `host.read`, like `fetch`).
- `byte_at(text, offset)`, `byte_slice(text, start, end)`: 0-based byte offsets, end exclusive; slices share memory.
- `error(message)` makes an Error value.
- The module exports its text heap global `text_heap`; the host allocates `read`/`fetch` results from it. Before, the host
  wrote at 65536+ by its own counter, which could overlap texts the module allocated at runtime.

## Bugs fixed on the way (all were silent wrong results or validation failures)
- while loops in functions took local 0 and 1 as temps: they overwrote the first parameter and variable.
- `if a < n {…}` / `while i < n {…}` parsed the block as an argument of `n`. Still open: `i<n {` without spaces lexes `<n` as a tag.
- `return x` inside a statement of a Node-returning function returned a number; `if c { return "t" }; …` did not make
  the function text-valued; `if … {"a"} else {"b"}` in a function was numeric.
- `{ if c {…}; x }` was read as a data list `{0 x}`: `if`, `while`, `i++`, `return` now mark a statement sequence.
- A dropped statement that updates a text (`s += "a"`, `if c { s += "a" }`, a while body doing that) was emitted as a number.
- `f("a")` for `f(t:text)`: a one-character literal is a codepoint and is now converted to text at the call.

## Language pitfalls to know when writing wasp (not fixed)
- `"a"` with one character is a codepoint: a variable first assigned `"x"` is a codepoint variable, `s = s + "ab"` then fails loudly. Start texts with `""`.
- `/` is exact division (rationals): use `>> 1` for halving indices.
- `global g = read(…)` fails ("undefined variable: read"); `const g = read(…)` works and is imported by `use`.
- Unannotated parameters are Int: annotate text parameters `f(t:text)`.
- The ignored test `test_string_concat_wasm` (tests/test_wasm.rs) now passes; it is left ignored (existing tests are not edited).

## Design decisions
- Controls follow their character (fonts/README.md): `<:red A>` → A U+E0072. A block's own entry wins over the suffix,
  so `<:red circle>` → 🔴. Unicode's precomposed letters are separate blocks (`reversed`/`reverseInPlace`, `turned`) so that
  `mirror`/`turn` round-trip: e + U+E004D stays that and never becomes ɘ.
- Spaces inside `<:type …>` and inside `<:type> … <:/type>` separate operands and are dropped (spec "Spaces"); text outside tags is kept.
- `<:greek> a b c <:/greek>` gives αβψ (Greek keyboard layout), the wiki example says α β ζ.
- Not supported: two suffixes on one character from uniscript (`<:mirror red A>` mirrors r, e, d and A); nested tags.
  Reverse spells a second suffix by its Unicode name (`<:tag-latin-small-letter-r>`), which still round-trips.
- Reverse prefers: own name, a well known short name (same in HTML and LaTeX, or the HTML name is the last word of the Unicode
  name: alpha), the block form (`<:fracture A>`), else the Unicode name.
