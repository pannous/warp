# Where aliases live

Each kind of word has one alias table. Add a spelling to its table, not a new list next to the code that needs it.

| words | table | note |
|---|---|---|
| operators: symbols, glyphs, words (`plus`, `≤`, `is at least`, `and=`) | src/operators/mod.rs OPERATOR_SPELLINGS | longest match first (lookahead.rs peek_operator); context guards in operator_in_context |
| negated spellings (`is not`, `nand`, `⊼`) | src/operators/mod.rs NEGATED_SPELLINGS | parsed as the positive operator wrapped in Not |
| prefix operator words (`sqrt`, `abs`, `norm`, `not`) | src/warp_parser/lookahead.rs PREFIX_OPERATOR_WORDS | analyzer checks.rs reads it too |
| library words (`sorted`, `uppercase`, `strip`) | src/lowering/words/library_words.rs SYNONYMS | canonical_spelling(); list_phrases uses it |
| built-in type words (`str`, `string`, `integer`) | src/lowering/types/type_tests.rs BUILTIN_TYPES | canonical_spec_word(), is_text_type_word() |
| print's spellings (`println`, `writeln`) | src/warp_parser/mod.rs PRINT_SPELLINGS | print is a statement word: a SYNONYMS entry made `print a \|\| b` bind as `(print a) \|\| b` |
| foreign keywords (`constant`, `new P`, `enum class`) | src/warp_parser/atoms.rs foreign_form | note_alias_at gives the note the word's own position |
| pragmas (`use relaxed`/`lenient`/`loose`, `use script mode`) | src/errors/diagnostic/mod.rs RELAXED_PRAGMAS, PRAGMA_SUFFIX | any pragma may end in `mode`; is_pragma (program) and says_pragma (source lines) both read it |

Lessons (cards operator-spelling, synonyms-table, alias-hint-position, 2026-10-14):
- A word operator needs an operand after it (operand_follows). Otherwise `use same;` reads `same` as `===`.
- A table entry changes what predicates say about a word: is_library_word and is_operand_word read SYNONYMS. Before
  adding a statement word, check the binding.
- A note raised in the parser must set its own position (note_alias_at); the hint position is otherwise stale.

Folded (card rs-private, 2026-10-14): welcome_forms' RUBY_METHODS and LINQ_METHODS and the sumOf entry of
BLOCK_REDUCTIONS are one METHOD_ALIASES table; R's and PHP's iteration functions live in lib/R.warp and lib/PHP.warp.
