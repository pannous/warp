# warp format (= warp lint)

`warp format [check|yes] [hash-comments] <file|folder|code>…` writes the one canonical layout of warp code
(lib/fix/command.warp, run by core/native/warp.warp; decision warp-format). `warp lint` is the same command.
Options are words, never `--flags`; `help`, `--help`
and `-h` anywhere print the usage. `warp beautify [check|yes] <file|folder>…` (= `warp fix`) is the opinionated verb:
the layout plus the best-practice rewrites (hash-comments, other languages' words the warp words), shown as a diff
first (decision beautify-preview): "apply? y/n" (the end of the input answers no), `yes` applies without
asking, `check` only lists. warp format previews and asks the same way.
Code given instead of files (`warp lint 'use script; ls'`: no argument exists or looks like a path)
prints its formatted text; `check` lists the layout changes.

## Layout (lib/fix/layout.warp)

- rules in order: `# comment`, trailing whitespace, blank lines, spaces, space after a comma or semicolon, space after
  a key's colon, spaces around an operator; then a final newline
- `name: value`, a parameter's type too: `f(x: int)`; one space after a comma or semicolon
- binary operators spaced (spaced_operators: `x = a + b * 2`); `<` and `>` stay as written (tags, generics), `/` too
  (units like km/h), `-` is spaced only after a number or a closing bracket (`full-name` is a name, `f it-1` reads
  f(it - 1)); an operator glued to another operator character (`===`, `x=-1`, `..`) is left alone
- no trailing whitespace, at most one blank line in a row, none at the start or the end, one final newline; texts and
  block comments keep their blanks
- `hash-comments`: `// text` → `# text`, `/// doc` → `## doc`, `//!` stays (`//` reads like Python's floor division);
  always on in `warp beautify`
- folders named `golf/` are skipped (code.golf counts bytes), also when named; non-warp files are skipped

## Rewrites (beautify only, lib/fix/rewrites.warp)

Hints read from the text, each naming what is written and the warp spelling meant, applied by the fix: `auto` → var,
other languages' calls (`len(xs)` → `count(xs)`, `xs.push(3)` → `xs.add(3)`), `7 div 2` → `7 ÷ 2` (decision
core-int-division), `alias a = b` → `alias a => b`, a text unit plural read as a number is counted, inline HTML as the
warp markup it reads as; a `use` line for each standard module the file uses without saying it (`warp uses`).

## Never changing the meaning

Whitespace is meaning in warp (`f it-1` vs `f it - 1`, `1 -1` a list, `xs ./2`, offside blocks). The rules read the
text only, never the parse tree, so they change only what reads the same. `check` lists `file:line: layout: <rule>`,
exit 1 when something would change; a file that cannot be read is reported and counted as not formatted (exit 1).
