# Shell words and paths without quotes (card unix-words)

User docs: wiki/script.md. Decisions (user, 2026-10-15, via warp-interviewer): rm moves to the Trash with rails A–D (one
file or empty folder, no pattern, never root/home/working directory and above, sandbox); no recursive rm; wc is a map;
ls hides dotfiles; grep is a regex; paths expanded at use, not at parse.

## Path literals (src/warp_parser/literals/paths.rs `starts_path_literal`)

A path literal is a Text, glued to what follows and not to an operand before it:
- `./x`, `../x` (older, card play-mozart)
- `~/x`, and `~` alone when no operand follows (`cd ~`; `a ~ b` stays Op::Rough)
- `/x` where no operand stands before it (`x = /tmp`, `print(/etc)`), or after a word of words::PATH_COMMANDS
  (`ls /tmp`); `width /height` divides. PATH_COMMANDS is a const, not read from lib/shell.warp: the parser would
  parse the module while parsing it (std_module_words' OnceLock).

`~` is expanded where a path is used (util::home_expanded: std_adapters `path_of`, the host's read), so a quoted
"~/x" works too and compiled programs expand on the machine they run on.

## Words (lib/shell.warp, adapters src/modules/shell_adapters.rs, browser web/playground/host-files.js)

`sort`, `unique`, `find`, `size` are list words a module definition would shadow for the whole program (no overloading
by parameter type across modules), hence sort_lines, unique_lines, find_files, file_size.
`export a=b` is not added: `export` already declares a WebAssembly export.
cwd became a getter (`cwd := …`) so it follows `cd`; std modules' getters no longer get the "runs at every read" warning.

## Known problem

A map key named like a module word inside a module function is renamed (`{lines: …}` → `lib·lines`,
modules/needed_definitions.rs renamed_word); wc quotes its keys meanwhile (card queued).
A sample named like a standard module (samples/shell.warp) shadows it when warp runs in the repo root: the sample is
samples/unix_commands.warp.
