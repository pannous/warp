# warp format (= warp lint)

`warp format [check] [hash-comments] <file|folder>…` writes the one canonical layout of warp code
(src/formatting/, src/cli/formatting.rs; decision warp-format, card warp-format). `warp lint` is the same command.
Options are words, never `--flags` (user, 2026-10-15; the older flags move with card cli-words).

## Layout (user, 2026-10-15)

- tabs, one per nesting level; a line that opens several brackets (`xs.map(x => {`) opens one level; an offside block
  (`def f(x):` and the lines indented below it) one more; a continued line (after `=`, `+`, `|>`, … or starting
  with `|>`, `.`, `then`) one more
- `name: value`, a parameter's type too: `f(x: int)` (user: "spaced everywhere", so help and the standard-library
  docs print signatures that way); one space after a comma, none before; none just inside `(…)` and `[…]`
- every binary operator spaced: `x = a + b * 2` (ranges `..`, `//`, `<`, `>`, `.` stay as written)
- list separators stay as written (commas or spaces)
- no trailing whitespace, at most one blank line in a row, one final newline; the spaces before a trailing comment and
  comment text stay
- `hash-comments`: `// text` → `# text`, `/// doc` → `## doc` (user: `//` also means floor division); belongs to
  the coming opinionated verb (card warp-format-strict: `warp beautify`, synonyms canonical, idiomatic)
- folders named `golf/` are skipped (code.golf counts bytes); a file named explicitly is formatted

## Never changing the meaning

Whitespace is meaning in warp (`f it-1` vs `f it - 1`, `1 -1` a list, `xs ./2`, offside blocks). So every rule's edits
are checked: the source is parsed before and after and the trees compared (`shape`: positions and comments dropped,
the other Meta kept: `@unit(cm)`, SpacedMinus, a literal's source text). An edit that changes the tree is dropped; a
rule's edits are bisected so one bad line costs only itself. `check` lists `file:line: layout: <rule>`, exit 1 when
something would change. A file that doesn't parse is reported and left as it is.

probes/warp_format_samples.sh formats a copy of samples/ and lib/ and compares `warp lower` of every formatted file
with the original (109 files, 0 differences on 2026-10-15), then checks a second pass changes nothing.

## Soft hints (report only, exit unaffected)

- abbreviations (user: every word, built-ins too): `cnt` → count, `str` → text, … (ABBREVIATIONS in soft_hints.rs),
  each word once per file at its first line, also inside names (`max_cnt`, `maxCnt`)
- `//` comments: one hint per file with the count, pointing to `hash-comments`

## Gate

The Integrator's gate.sh runs `warp format check samples lib` report-only (`FORMAT n to format`), never failing.
