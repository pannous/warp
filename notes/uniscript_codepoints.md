# Uniscript code point escapes (user 2026-11-15)

A text names a character by its code point after the entity mark `\:`, in single and double quotes alike:

    "\:U+12C00"  "\:u+12C00"  "\:0x12C00"  '\:U12C00'  '\:u12C00'   # each is 𒰀, U+12C00

- `U+` `u+` `0x`: one or more hex digits, read greedily; these marks never start an entity name, so no clash.
- `U` `u` with no plus: 4 to 6 hex digits that end the word (`\:U12C00x` is a word, left as written), and only when
  the whole word is no known name: a name always wins (`\:Ua` is Ⲩ).
- A bare number `\:12C00` is no code point (it clashes with names and reads ambiguously): left as written, like an
  unknown name.
- A surrogate (U+D800…DFFF) or a number past U+10FFFF is refused: `invalid code point` (the `as codepoint` check that
  `\u{…}` uses too).
- Named entities `\:alpha` are read in single quotes too now: every extension's text escape (text_escape_readers)
  applies to both quote styles; before, `'\:alpha'` gave `:alpha`.

Where: extensions/entities.warp (`code_point_escape`, read by `entity_escape`); core/parsing/parser.warp `unescaped`
no longer skips the extension escapes in single quotes. Outside texts the source pass is unchanged: a code point
escape is a text form only. Tests: test/text/test_uniscript_codepoint_escape.pairs.warp.
