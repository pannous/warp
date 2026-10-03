# Normalization audit

Each row was checked with `normalize::capture_hints` (the built parser) against the default `Style`; the test column names the test in
`tests/node/test_normalization.rs` that asserts exactly one hint with the canonical text and position for the non-canonical form and
none for the canonical one. Every `Style` field also has a test with the style swapped (the form that is canonical by default is then hinted).
Positions are `line:column` of the first character of the form (the operator for infix operators, the opening quote for strings).

Result before this audit: only the operator hints (`&&` `||` `**` `?`), `'…'` quotes and the `int(x)` constructor cast printed anything,
with stale positions; definitions, `!`, `&`, `|`, `list<int>`, `s[1]`, `s.length()` printed nothing.

| Form | Canonical (default Style) | Hint | Test |
|------|---------------------------|------|------|
| `x:list<int>=[1 2]` (any element type word) | `x:ints=[1 2]` | was missing, now ok | test_generic_list_type_hints_plural |
| `x:list of int=[1 2]` (any element type word) | `x:ints=[1 2]` | was missing, now ok | test_of_list_type_hints_plural |
| `x:ints` / `texts` / `floats` / `numbers` / `strings` / `chars` | canonical, no hint | ok | test_plural_list_type_is_canonical |
| `x:list<list<int>>` | no plural spelling, no hint | ok (by design) | test_list_of_lists_has_no_plural_spelling |
| list type, `ListTypeStyle::Generic` / `Of` | `list<int>` / `list of int`, nested chains too | new Style field | test_list_type_style_generic, test_list_type_style_of |
| `'hello'` | `"hello"` | wrong position (after the string), now ok | test_quotes_are_symmetric, test_quote_hint_position_is_the_opening_quote |
| `"hello"` under `QuoteStyle::Single` | `'hello'` | was missing (one direction only), now ok | test_quotes_are_symmetric |
| `'a'`, `"a"` | one character is a codepoint, no canonical quote | ok | test_one_character_strings_have_no_canonical_quote |
| `x && y`, `x & y` | `x and y` | `&` was missing, now ok | test_logical_operators |
| `x \|\| y`, `x \| y` | `x or y` | `\|` was missing, now ok | test_logical_operators |
| `!x` | `not x` | was missing (prefix), now ok | test_logical_operators |
| `and` / `or` / `not` under `LogicalStyle::Symbols` | `&&` / `\|\|` / `!` | was missing, now ok | test_logical_operators_symbol_style |
| `x ** 2` | `x ^ 2` | ok; `^` under `PowerStyle::DoubleStar` was missing, now ok | test_power_operator |
| `c ? a : b` | `if c then a else b` | ok; `if` under `ConditionalStyle::Ternary` was missing, now ok | test_conditional |
| `int(x)`, `float(x)`, `char(x)` | `x as int` | text of the argument lost its quotes, position stale, now ok | test_cast_constructor |
| `str(x)`, `String(x)` | `x as string` | two hints (cast and type word), now one | test_string_type_word_is_hinted_once_with_the_cast |
| `x as str`, `x:str` | `x as string`, `x:string` | was missing, now ok | test_string_type_word_is_hinted_once_with_the_cast |
| `x as int` under `CastStyle::Constructor` | `int(x)` | was missing, now ok | test_cast_style_constructor |
| `let x = 5`, `var x = 5` | `x := 5` | was missing, now ok (real name and value in the text) | test_variable_definition |
| `x := 5` under `VarStyle::Let` | `let x = 5` | was missing, now ok | test_variable_definition_let_style |
| `def f(x): …`, `fn f(x) = …`, `fun f(x) = …`, `function f(x) { … }` | `f(x) := …` | was missing (hook never ran for evaluated code), now ok | test_function_definition |
| `define f(x): …` | `f(x) := …` | was missing, `FunctionStyle::Define` added | test_define_keyword |
| `f(x) := …` under another `FunctionStyle` | that style's spelling | was missing, now ok | test_function_definition_def_style |
| `s[1]` | `s#2` (brackets count from 0, `#` from 1) | was missing; canonical text now shifts the index | test_indexing |
| `s#2` under `IndexStyle::Bracket` | `s[1]` | was missing, now ok | test_indexing |
| `s.length()` | `#s` | was missing, now ok | test_indexing |
| `#s` under `IndexStyle::Bracket` | `s.length()` | was missing, now ok | test_length_prefix_under_bracket_style |
| evaluated code | each form hinted once | emitter and analyzer used to emit on their own passes | test_each_form_is_hinted_once_when_evaluated, test_list_type_hint_is_emitted_once_when_evaluated |
| hint mode `Off` | no hints | ok | test_hints_are_off_when_the_mode_is_off |

## Forms that are not normalized, on purpose or open

| Form | Why no hint |
|------|-------------|
| `a to b` vs `a..b` | not synonyms: `1 to 3` is `[1 2 3]`, `1..3` is `[1 2]` (exclusive end); test_ranges_have_no_canonical_form |
| `x = 5` vs `x := 5` | assignment and definition differ in meaning, no canonical form |
| Unicode `≤ ≥ ≠ × ÷ ¬ ∧ ⋁ √`, `is` for `==` | accepted aliases, no canonical form declared in `Style` (open decision) |
| `s.size`, `s.count` vs `s.length` | wiki/alias.md wants them normed; needs the receiver's type (a user object may own a `count` field), so the parser cannot decide. Open: do it in the analyzer |
| `x be 5` (wiki/be.md, alias of `:=`) | not accepted by the parser at all, unimplemented |
| `int[100]`, `100 int` typed arrays | distinct declarations, not spellings of one form |
| `#s` at the start of a line | a comment, not a length |

## Mechanism changes

- `normalize::capture_hints(|| …)` records the hints emitted on the thread (original, canonical, position) for tests.
- Operator hints are emitted where the parser consumes the operator (`hint_operator`), not in the `peek_*` functions that may run repeatedly.
- `normalize::check_style` walks the parsed AST once and hints the forms that are only visible there; the emitter/analyzer hooks were removed.
- Hint positions come from the node's `LineInfo`; an unknown position is cleared instead of reusing the last one.
- Only default-mode parses hint: data, XML and WIT parses (`parse_data`, `.wit`) do not check the style of foreign text.
  (Quote hints inside `parse_data` text existed before; they remain.)
