# Shell words and paths without quotes (card unix-words)

User docs: wiki/script.md. Decided: rm moves to the Trash with rails A–D (one
file or empty folder, no pattern, never root/home/working directory and above, sandbox); no recursive rm; wc is a map;
ls hides dotfiles; grep is a regex; paths expanded at use, not at parse.

## Path literals (src/warp_parser/literals/paths.rs `starts_path_literal`)

A path literal is a Text, glued to what follows and not to an operand before it:
- `./x`, `../x`
- `~/x`, and `~` alone when no operand follows (`cd ~`; `a ~ b` stays Op::Rough)
- `/x` where no operand stands before it (`x = /tmp`, `print(/etc)`, `[/a, /b]`), after another path (`cp /a /b`,
  parser field path_end) or after a word that is no variable assigned so far but a function: the program's (parser
  field functions), a built-in (negated_calls::is_builtin_callable) or a standard module's (std_function_names, read
  from the modules' text: the parser may be parsing one of them). `width /height` and a parameter `w /h` divide.
  The parser decides with what it knows; deciding in the lowering (the SpacedMinus way) would need the path text and
  the division's precedence both kept.

`~` is expanded where a path is used, so a quoted "~/x" works too and compiled programs expand on the machine they run
on: lib/file.warp `home_expanded` ($HOME) in the file words and the shell words that read (card unix-words-native). The
prelude's own `read` does not expand yet: that would be the runtime's opened_file (core/runtime/extended/wasi.warp).

which, find_files and file_size are plain warp over list_files, exists and env (card unix-words-native): which walks the
PATH, find_files turns the wildcards into a regular expression and descends what lists, file_size counts read_bytes.
`use script` is lib/script.warp, the module that uses os, file, net and shell.

## Words (lib/shell.warp, adapters src/modules/shell_adapters.rs, browser web/playground/host-files.js)

`sort`, `unique`, `find`, `size` take paths through class dispatch (card cross-module-overload): lib/shell.warp has
`class Path { name: text }` and `sort(file:Path)` etc.; the list words stay the fallback. traits/generics.rs `as_path`
wraps a path literal (parser marks it with Meta `PathLiteral`) into a Path where a method takes Path there, and so
anything holding only paths (warp_parser `WrittenPaths`, card path-written): a variable only ever assigned one, a
parameter every call passes one (`f(p) := sort p; f(./n.txt)`), a call of a function giving only paths (`g() :=
./n.txt; sort g()`), grown to a fixed point. A name holding a path and a text somewhere stays text. find is an iteration word: lambdas.rs leaves it to the method only when its last argument is
a text or number (`calls_method`). The interim names sort_lines, unique_lines, find_files, file_size stay.
A word a used module defines itself is not hidden by its own `use` (loader.rs hidden_apart). Path parameters are `text`, not `any`: a word taking anything reads
`file_size /etc/hosts > 0` both ways (P143, broadcasting.rs ambiguous_call).
`export a=b` is not added: `export` already declares a WebAssembly export.
cwd is a getter (`cwd := …`) so it follows `cd`; std modules' getters get no "runs at every read" warning.

## Known problem

The pure compiler does not wrap path literals into Path yet (the Rust traits/generics.rs `as_path`): `sort
./names.txt` sorts the path's characters and `size ./a.txt` fails; until then use sort_lines, file_size.
A braceless `print "a:", n` inside `{ … }` fails with "not a list" (parser.warp's print phrase is read only from a
space-separated curly), which keeps samples/system/unix_commands.warp's last line from running.
`matches(line, pattern)` written as a call inside a module that uses regex is "outside the core"; `line matches
pattern` works.

A map key named like a module word inside a module function is renamed (`{lines: …}` → `lib·lines`,
modules/needed_definitions.rs renamed_word); wc quotes its keys meanwhile (card queued).
A pipe after a braceless call (`print cat "/etc/hosts" | count`) traps: card pipes-after.
A sample named like a standard module (samples/shell.warp) shadows it when warp runs in the repo root: the sample is
samples/system/unix_commands.warp.
