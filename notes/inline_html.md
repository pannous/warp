# Inline HTML (card jsx, decision jsx)

User decision (notes/decisions.md jsx): `<tag …>…</tag>` in expression position reads into the same markup nodes,
with `{expr}` holes. Inline HTML is accepted with a warning that `warp format` turns it into warp markup, and
warp format does that rewrite.

- `extensions/inline_html/markup.warp` `markup_of_inline_html(source, lines_kept)` is the one conversion, on the
  code points with the lexer's words (span_end skips texts and comments).
  - The compiler runs it as a source pass, `extensions/inline_html.warp`, with the lines kept: the newlines an element
    took go before its `}`, so the nodes after it keep their lines and need no shifts.
  - It also adds an `inline_html(tag)` warning (core/analysis/warnings.data) at each element.
  - lib/fix/fix.warp (`warp format`, lint, beautify, fix) runs the same function with the lines not kept, and lists
    the change as "inline html".
- What is read: `<tag attr="text" attr={expr} bare>` (bare means `yes`), `<br/>`, children texts, `{expr}` holes and
  nested elements, and `{/* comment */}` (which shows nothing). `onClick` becomes `onclick`, the handler attribute
  lib/markup.warp knows. Texts follow JSX's whitespace rule: lines are trimmed where they break, blank lines are
  dropped, and the rest is joined by one space.
- Where: a `<` glued to a letter, at a line start or after `= ( [ { , : >` or `return`/`then`/`else`. The element must
  also close (`</tag>` matching, or `/>`). Anything else stays as written, so `a <b` and `x<y` are still comparisons.
- Not read: `<>…</>` fragments, `{...spread}` attributes, and `<` after other operators (`x + <b>…</b>`). To use
  these, write warp markup.
