# Uniscript in warp (uniscript.wasp of the uniscript package)
Rust implementation over the same data files: https://github.com/pannous/uniscript (~/dev/uniscript).

`use uniscript; uniscript("<:fracture A>")` → 𝔄, `unicode_to_uniscript("α")` → `<:alpha>`. `use uniscript` fetches github.com/pannous/uniscript into packages/uniscript and calls its release asset uniscript_c.wasm since 1.0.8 (the C library of its Rust implementation as a core module) through lib/packages/uniscript.warp: 0.3 s per program instead of 1.5-3 s compiling uniscript.wasp, which `use uniscript version 1.0.0` and other older tags still load (notes/packages.md); index format: its src/index.rs.
Tests: tests/web/test_uniscript.rs (spec examples, round trip, index check), tests/text/test_text_bytes.rs, tests/text/test_text_functions.rs.

## Language features added for it (Rust)
- `text + text`, `text + 'c'`, `'a' + 'b'` concatenate (`text_concat`, a codepoint is UTF-8 encoded by reusing
  `text_with_char_at` on a one-byte placeholder); `s += "x"` lowers to `s = s + "x"`. `text + number` stays a type error.
  An Error operand is the result: errors propagate through concatenation.
- `read(path)` → the file's bytes as Text, or an Error with the reason (host import `host.read`, like `fetch`).
- `byte_at(text, offset)`, `byte_slice(text, start, end)`: 0-based byte offsets, end exclusive; slices share memory.
- `error message` makes an Error value.
- The module exports its text heap global `text_heap`; the host allocates `read`/`fetch` results from it.

## Language pitfalls to know when writing warp
- `/` is exact division (rationals: `1/3`): use `>> 1` for halving indices.
- `i<n{` glued to its block fails ("cannot extract a numeric value from n{…}"): write `i < n {`.

## Design decisions
- Controls follow their character (uniscript repository, fonts/README.md): `<:red A>` → A U+E0072. A block's own entry wins over the suffix,
  so `<:red circle>` → 🔴. Unicode's precomposed letters are separate blocks (`reversed`/`reverseInPlace`, `turned`) so that
  `mirror`/`turn` round-trip: e + U+E004D stays that and never becomes ɘ.
- Spaces inside `<:type …>` and inside `<:type> … <:/type>` separate operands and are dropped (spec "Spaces"); text outside tags is kept.
- `<:greek> a b c <:/greek>` gives αβψ (Greek keyboard layout), the wiki example says α β ζ.
- Effect words stack: in `<:mirror red A>` the last block word (red) takes the operands, the words before it add their
  suffixes after each character: A U+E0072 U+E004D. Reverse spells suffixes s1 s2 as `<:s2 s1 X>`, so it round-trips.
  Not supported: nested tags.
- Reverse prefers: own name, a well known short name (same in HTML and LaTeX, or the HTML name is the last word of the Unicode
  name: alpha), the block form (`<:fracture A>`), else the Unicode name.

## Porting note: meta information (TAG sequences), done in Rust and Swift, open in the package's uniscript.wasp
Reference: github.com/pannous/uniscript `src/compiler/meta.rs`, `src/lib.rs` (`meta_tag`, `to_uniscript`, `html`), tests
`tests/meta_test.rs`, Swift `Sources/Uniscript/Meta.swift`; spec wiki/uniscript.md "Meta information".
- Data: the package's index already has the 5 tables, appended, so `names_table`/`chars_table`/`suffixes_table` stay
  valid: `fonts_table = 3` (`han-japanese ` → "", `han-japanese lang` → ja, `… families`, `… features`), `meta_table = 4`
  (key → CSS template, `{}` is the value; `lang` → "" means the HTML lang attribute).
- Sequence: TAG characters U+E0020+ascii spell the text, CANCEL TAG U+E007F ends it. `<key value` opens a span,
  `</key` closes the innermost open span of that key, `:key value` attaches to the character before it. key:
  [a-z][a-z0-9-]*, value: one word of [A-Za-z0-9#.%+-_,()/], else `Error::InvalidMeta`.
- Forward, in `tag()` after the entity lookup (entity names win: `<:angle>` ∠, `<:angle with s inside>` ⦞): while the
  first word is a meta key take key + value; nothing left → open sequences; else convert the rest as a tag (fallback:
  space separated tokens, names looked up, other text literal) and put the attached sequences after each character
  and its marks/joiners/suffix TAGs. `<:/key>` with a meta key → close sequence, checked before the block closer.
  `font` with a value that is no font style warns ("… used as a font family").
- Reverse: a known open/close sequence → `<:key value>` / `<:/key>`; after a character first an emoji tag sequence
  (only letters/digits before CANCEL: 🏴 gbsct) is copied unchanged, else the single-letter suffixes, then attached
  sequences → `<:key value [effects] X>`. Unknown keys fall through to the character-by-character spelling.
- HTML (`uniscript --html`): parse tagged text to plain text + nested runs (a close over later-opened spans closes and
  reopens them), unknown keys and unmatched closes warn; font style → lang + font-family list + font-feature-settings.
