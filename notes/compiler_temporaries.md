# Compiler temporaries

A name a lowering pass makes carries the mark `·` (analyzer::TEMPORARY_SEPARATOR): `word·sum·3`, `x·iterator`,
`try·value·1`. The program cannot write `·` in a name (`a·b` parses as a product), so:
- `analyzer::is_compiler_temporary(name)` tells them apart; the local-or-global scope check skips them (card
  sum-helper: `total(xs) := sum(xs)` plus a main-level `sum(…)` asked whether `word_sum` was main's).
- `diagnostic::written_text` shows the source instead of a node holding one.

Making them:
- A template parsed from text cannot hold `·`: write `prefix_part` and call `library_words::named_apart(node, prefix,
  base, number)` after parsing, before filling placeholders (so the program's own `prefix_…` names stay); placeholders
  use another prefix. Examples: library_words (`hidden_`, `try_tmp_`, `safe_tmp_`), min_max (`extremum_`), parallel.
- A name built directly: `library_words::temporary_name(&[base, part, number])`.
- Splice a variable into a template by substituting its node, never by formatting its name into the text (a `·` name
  would parse as a product).

Not marked on purpose: names that are shared state the checks should see (`tests_failed_`, `signal_listening_0`,
`once_fired_0`).
