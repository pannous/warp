# Where aliases live

Each kind of word has one alias table. Add a spelling to its table, not a new list next to the code that needs it.

| words | table | note |
|---|---|---|
| operators: symbols, glyphs, words (`plus`, `≤`, `is at least`, `and=`) | src/operators.rs OPERATOR_SPELLINGS | longest match first (lookahead.rs peek_operator); context guards in operator_in_context |
| negated spellings (`is not`, `nand`, `⊼`) | src/operators.rs NEGATED_SPELLINGS | parsed as the positive operator wrapped in Not |
| prefix operator words (`sqrt`, `abs`, `norm`, `not`) | src/warp_parser/lookahead.rs PREFIX_OPERATOR_WORDS | analyzer checks.rs reads it too |
| library words (`sorted`, `uppercase`, `strip`) | src/lowering/library_words.rs SYNONYMS | canonical_spelling(); list_phrases uses it |
| other languages' words for warp words that are no library words (`Select`, `sapply`, `to_s`, `sumOf`) | src/lowering/library_words.rs FOREIGN_SYNONYMS | welcome_forms::foreign_words renames them with a note, in method and call position; a defined name or a bare `.W` the program has as a field stays; PHP's function-first `array_map` swaps its arguments (FUNCTION_FIRST) |
| built-in type words (`str`, `string`, `integer`) | src/lowering/type_tests.rs BUILTIN_TYPES | canonical_spec_word(), is_text_type_word() |
| print's spellings (`println`, `writeln`) | src/warp_parser/mod.rs PRINT_SPELLINGS | print is a statement word: a SYNONYMS entry made `print a \|\| b` bind as `(print a) \|\| b` |
| foreign keywords (`constant`, `new P`, `enum class`) | src/warp_parser/atoms.rs foreign_form | note_alias_at gives the note the word's own position |

Lessons (cards operator-spelling, synonyms-table, alias-hint-position, 2026-10-14):
- A word operator needs an operand after it (operand_follows). Otherwise `use same;` reads `same` as `===`.
- A table entry changes what predicates say about a word: is_library_word and is_operand_word read SYNONYMS. Before
  adding a statement word, check the binding.
- A note raised in the parser must set its own position (note_alias_at); the hint position is otherwise stale.

Folded (card rs-private, 2026-10-14): welcome_forms' RUBY_METHODS, LINQ_METHODS, FOREIGN_ITERATIONS and the sumOf entry
of BLOCK_REDUCTIONS became FOREIGN_SYNONYMS; their spellings of library words (`upcase`, `downcase`, `Contains`,
`First`, `Last`) are SYNONYMS entries.
