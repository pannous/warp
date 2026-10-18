# Markup as a plain library (card compiler-modules, markup slice; decisions.md markup-plain-library)

Goal: the markup forms are ordinary definitions in lib/markup.warp, no Rust lowering pass. A markup value is data
(`label{ for: x; "L" }` evaluates to a Node) and lib/markup.warp's to_html is the one renderer (src/web/markup.rs
runs it too), so most forms are a meaning the renderer gives that data, not a rewrite.

| pass (Rust lines) | form | route | state |
|---|---|---|---|
| select_options (81) | `select{ ["apple" "pear"] }`, `select{ fruits }`, `"s": "small"` | renderer: what a select holds besides elements is its options (html_options) | done |
| glued_attributes (43) | `label{ for: name "Name" }` | dropped (user): `label{ for: name; "Name" }` | done |
| markup_tags form routes | `form post "/t" {…}` | a library function `form(method: symbol, action: text, body)` (user); the call form waits for card function-words-consume | next |
| markup_tags html attributes | `input{type="text"}` | core rule (user): `name = value` inside any data block is a key | next |
| markup_tags tags | `div(class:"x")`, spaced `ul { … }`, `li all xs`, a loop among children | one by one | open |
| style_rules (220) | unquoted selectors `.card {…}`, `ul > li`, lengths `8px` | lengths already evaluate (units); selectors need the sheet as data | open |
| transitions (241) | `transition: fade 200ms`, `leave: pop 400ms reverse` | renderer: words as CSS text; fade/scale/slide a table in the library | open |

Found on the way: inside a block a literal list's items are the block's (`select{ ["a" "b"] }` holds "a" and "b"),
so the renderer cannot tell a listed option from a text child; in a select both are options. A one-letter text is a
Char, which contains() refuses (card contains-fails; html_is_custom_element works around it with str()).
