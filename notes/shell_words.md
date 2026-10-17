# Shell words and paths without quotes (card unix-words)

User docs: wiki/script.md. Decisions (user, 2026-10-15, via warp-interviewer): rm moves to the Trash with rails A–D (one
file or empty folder, no pattern, never root/home/working directory and above, sandbox); no recursive rm; wc is a map;
ls hides dotfiles; grep is a regex; paths expanded at use, not at parse.

## Path literals (src/warp_parser/literals/paths.rs `starts_path_literal`)

A path literal is a Text, glued to what follows and not to an operand before it:
- `./x`, `../x` (older, card play-mozart)
- `~/x`, and `~` alone when no operand follows (`cd ~`; `a ~ b` stays Op::Rough)
- `/x` where no operand stands before it (`x = /tmp`, `print(/etc)`, `[/a, /b]`), after another path (`cp /a /b`,
  parser field path_end) or after a word that is no variable assigned so far but a function: the program's (parser
  field functions), a built-in (negated_calls::is_builtin_callable) or a standard module's (std_function_names, read
  from the modules' text: the parser may be parsing one of them). `width /height` and a parameter `w /h` divide.
  (warp-reviewer asked for the SpacedMinus way, deciding in the lowering; the path text and the division's precedence
  can't both be kept until then, so the parser decides with what it knows.)

`~` is expanded where a path is used (util::home_expanded: std_adapters `path_of`, the host's read), so a quoted
"~/x" works too and compiled programs expand on the machine they run on.

## Words (lib/shell.warp, adapters src/modules/shell_adapters.rs, browser web/playground/host-files.js)

`sort`, `unique`, `find`, `size` take paths through class dispatch (card cross-module-overload): lib/shell.warp has
`class Path { name: text }` and `sort(file:Path)` etc.; the list words stay the fallback. traits/generics.rs `as_path`
wraps a path literal (parser marks it with Meta `PathLiteral`) or a variable only ever assigned one into a Path where a
method takes Path there. find is an iteration word: lambdas.rs leaves it to the method only when its last argument is
a text or number (`calls_method`). The interim names sort_lines, unique_lines, find_files, file_size stay.
shell's own `use list` used to hide list's words (loader.rs hidden_apart renames `unique` → `lib·unique`), which
renamed shell's method too, so `unique_lines` called the Path method on a list ("no field name"); a word a used
module defines itself is no longer hidden. Path parameters are `text`, not `any`: a word taking anything reads
`file_size /etc/hosts > 0` both ways (P143, broadcasting.rs ambiguous_call).
`export a=b` is not added: `export` already declares a WebAssembly export.
cwd became a getter (`cwd := …`) so it follows `cd`; std modules' getters no longer get the "runs at every read" warning.

## Known problem

A map key named like a module word inside a module function is renamed (`{lines: …}` → `lib·lines`,
modules/needed_definitions.rs renamed_word); wc quotes its keys meanwhile (card queued).
A pipe after a braceless call (`print cat "/etc/hosts" | count`) traps: card pipes-after.
A sample named like a standard module (samples/shell.warp) shadows it when warp runs in the repo root: the sample is
samples/system/unix_commands.warp.
