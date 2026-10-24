# Where aliases live (card alias-mechanism, decision alias-mechanism)

lib/aliases.warp is warp's one alias table, plain warp data, one entry per canonical word:

```warp
add { operator: "+", symbol: Op.Add, aliases: [plus] }
modulo { operator: "%", symbol: Op.Mod, aliases: [mod], infix: yes }
"else if" { aliases: [elif, elsif, elseif] }
```

- The entry's key is the canonical form, the one format and print show. An entry keyed by an operator (`"<="`, `"->"`)
  is that operator.
- `operator` / `prefix-operator`: the operator the word names; `symbol`: the operation (Op of lib/compiler/node.warp).
- `aliases`: the other spellings. A symbol alias spells the operator (`≤` is `<=`, `**` is `^`), a word alias the word
  (`size` is count, `elif` is `else if`).
- `infix: yes`: the word and its word aliases are also written between operands (`7 mod 2`, `1 upto 5`), but name a
  block written right after them (`div {…}` is the html tag).
- Keywords, operator words and constants are quoted (`"and"`, `"yes"`, `"null"`): written bare, the parser already
  reads them as what they are.
- A library keeps the aliases of its own words (lib/math.warp: `alias sin = sine`, `operator ∧ := and`).

## Who reads it

- lib/compiler/parser.warp `read_aliases`, the first time a parse starts (its own parse sees the parser's own
  operators only): symbol aliases join `symbol_operators` (ordered longest first, so `**` wins over `*`), infix words
  join `word_operators`, word aliases of a keyword take its form in `keyword_forms` (`elif`, `has`, `true`, `null`).
  `canonical_word(spelling)` gives the entry's word.
- lib/compiler/builtin_signatures.warp `unaliased`: a call's library word is its canonical word, then the prelude's
  `alias keys = map_keys` lines (public name → implementation, not synonyms).
- The Rust compiler (frozen) keeps its own tables, below; lib/prelude.warp keeps its `alias has = collection_contains`
  lines for it.

## Next

- Notes: every alias used gets the got-it note "warp says <word>" with the "I meant: <word>" fix (Rust twin
  diagnostic::note_alias); first the compiler's own sources switch to the canonical words, so the stage programs stay
  quiet. Later per-alias data (whether it warns, whether it shows as canonical), maybe as @meta tags.
- The spellings of the Rust tables below move into lib/aliases.warp (`plus`, `is at least`, SYNONYMS, PRINT_SPELLINGS,
  BUILTIN_TYPES), each Rust table marked `// ⚠ WARP DOES THIS DIFFERENTLY`.
- More aliases from the wasp list (`!= : unequal, ≢ …`) and warp-layers' list, each checked against the parser: a
  word alias written infix reserves the word.
- alias-network (Later card): a class giving `plus` answers `+`, one giving `add` answers `.plus`.
- math.warp's `operator ∧ := and` lines become entries of math.warp in the same form.

## The Rust compiler's tables (frozen)

The Rust lane has one alias table per kind of word. Add a new spelling to lib/aliases.warp, not a new list next to
the code that needs it.

| words | table | note |
|---|---|---|
| operators: symbols, glyphs, words (`plus`, `≤`, `is at least`, `and=`) | src/operators/mod.rs OPERATOR_SPELLINGS | longest match first (lookahead.rs peek_operator); context guards in operator_in_context |
| negated spellings (`is not`, `nand`, `⊼`) | src/operators/mod.rs NEGATED_SPELLINGS | parsed as the positive operator wrapped in Not |
| prefix operator words (`sqrt`, `abs`, `norm`, `not`) | src/warp_parser/lookahead.rs PREFIX_OPERATOR_WORDS | analyzer checks.rs reads it too |
| glyph spellings of operators (`×`, `≤`, `√`) and circumfix pairs (`‖ ‖`, `⌊ ⌋`) | lib/math.warp `operator × := *`, `circumfix operator ⌊ ⌋ := floor` | src/operators/aliases.rs reads them; parser.warp operator_aliases, circumfix_aliases; a pair whose function is a prefix operator word (abs) parses to that operator, any other to a call |
| library words (`sorted`, `uppercase`, `strip`) | src/lowering/words/library_words.rs SYNONYMS | canonical_spelling(); list_phrases uses it |
| built-in type words (`str`, `string`, `integer`) | src/lowering/types/type_tests.rs BUILTIN_TYPES | canonical_spec_word(), is_text_type_word() |
| print's spellings (`println`, `writeln`) | src/warp_parser/mod.rs PRINT_SPELLINGS | print is a statement word: a SYNONYMS entry made `print a \|\| b` bind as `(print a) \|\| b` |
| foreign keywords (`constant`, `new P`, `enum class`) | src/warp_parser/atoms.rs foreign_form | note_alias_at gives the note the word's own position |
| pragmas (`use relaxed`/`lenient`/`loose`, `use script mode`) | src/errors/diagnostic/mod.rs RELAXED_PRAGMAS, PRAGMA_SUFFIX | any pragma may end in `mode`; is_pragma (program) and says_pragma (source lines) both read it |

Gotchas:
- A word operator needs an operand after it (operand_follows). Otherwise `use same;` reads `same` as `===`.
- A table entry changes what predicates say about a word: is_library_word and is_operand_word read SYNONYMS. Before
  adding a statement word, check the binding.
- A note raised in the parser must set its own position (note_alias_at); the hint position is otherwise stale.

Ruby, LINQ and sumOf method spellings are one METHOD_ALIASES table (welcome_forms); R's and PHP's iteration functions live in lib/R.warp and lib/PHP.warp.
