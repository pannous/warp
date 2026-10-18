# Markup as a plain library (card compiler-modules, markup slice; decisions.md markup-plain-library)

Goal: the markup forms are ordinary definitions in lib/markup.warp, no Rust lowering pass. A markup value is data
(`label{ for: x; "L" }` evaluates to a Node) and lib/markup.warp's to_html is the one renderer (src/web/markup.rs
runs it too), so most forms are a meaning the renderer gives that data, not a rewrite.

| pass (Rust lines) | form | route | state |
|---|---|---|---|
| select_options (81) | `select{ ["apple" "pear"] }`, `select{ fruits }`, `"s": "small"` | renderer: what a select holds besides elements is its options (html_options) | done |
| glued_attributes (43) | `label{ for: name "Name" }` | dropped (user): `label{ for: name; "Name" }` | done |
| markup_tags form routes | `form post "/t" {…}` | a library function `form(method: symbol, action: text, body)` (user); the call form waits for card function-words-consume | next |
| markup_tags html attributes | `input{type="text"}` | core rule (user): `name = value` inside any data block is a key: lowering/data/tag_entries.rs | done |
| markup_tags tags | `div(class:"x")`, spaced `ul { … }`, `li all xs`, a loop among children | one by one | open |
| style_rules (220) | unquoted selectors `.card {…}`, `ul > li`, lengths `8px` | `style(sheet: data)` in lib/markup.warp (decision data-parameter): the sheet as the parser read it, selectors and lengths as texts, then the renderer as before | done (branch style-data) |
| transitions (241) | `transition: fade 200ms`, `leave: pop 400ms reverse` | renderer: words as CSS text; fade/scale/slide a table in the library | open |

Found on the way: inside a block a literal list's items are the block's (`select{ ["a" "b"] }` holds "a" and "b"),
so the renderer cannot tell a listed option from a text child; in a select both are options. A one-letter text is a
Char, which contains() refuses (card contains-fails; html_is_custom_element works around it with str()).

Data block (tag_entries.rs): the block of a tag, `word{…}` or `word: {…}`, the word no keyword (HARD_KEYWORDS,
HIGHLIGHTED_WORDS) and no function the program defines, holding no statement besides its `name = value` entries; an
element's block always (its loops and handlers give children). `kill{ hp = 0; print hp }` stays code. Wiki: data.md
gets the paragraph once the branch lands.

Library data functions (branch style-data): a standard function with a `x: data` parameter (std_modules.rs
std_data_functions, a line scan) quotes its tag's block or its argument in any program and loads its module without
`use` (loader.rs implicit_std_modules); a function of the program's own of that name wins. Bound names in the block are
their values (decision data-names), so `style: { color: theme }` keeps working. `style{…}` and the inline `style: {…}`
are the same call: the result is the pair `style: {…}`, and the renderer tells a sheet (rules) from declarations as
before. The parser's style-sheet context (in_style_sheet: hex colors, a blank before `.x`) still keys on the word style.
