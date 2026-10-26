# The `warp` launcher: programs through the pure pipeline (cards warp-launcher, warp-rust-install)

`warp` is a bash script, packaging/bin/warp. It runs programs through the warp compiler written in warp
(core/compiler.warp built as compiler.wasm, run as `warp-rust pure` runs it) and hands every other command to the
Rust binary, installed as `warp-rust`, unchanged (`exec`: same arguments, stdio and exit status). Plan A of the
investigation of 2026-10-22; no Rust changed. Progress is how many tests pass through the pure pipeline, not how close
it comes to Rust's output.

## Routing

| command | runs as |
|---|---|
| `warp f.warp [-- args]`, `warp run f.warp [-- args]` | `warp-rust pure f.warp [-- args]` |
| `warp <code>` (a word that is no command, file or option: `warp '6*7'`) | `warp-rust pure <code>` |
| `warp compile --wasm f.warp` | `warp-rust compiler.wasm -- f.warp f.wasm <folder of f> <lib>`, then `compiled N bytes to f.wasm` |
| `warp fix\|beautify [check\|yes] <file\|folder\|code>…` | `WARP_LIB=<lib> warp-rust --no-hints run <lib>/fix/command.warp -- fix\|beautify …` (lib/fix, card fix-route); warp-rust with the same words when that lib/ has no fix/command.warp |
| anything else: `warp`, `warp x.wasm`, `version`, `help`, `test`, `rebuild`, `serve`, `dev`, `app`, `deploy`, `eval`, `lower`, `parse`, `repl`, `compile --exe/--site/…`, `build`, a plain word (`warp pi`), `fast`, `fuel n` | `warp-rust` with the same words |

- Leading flags (`--no-hints`, `--strict[=level]`, `--warnings=…`, `--no-ask`, `--sandbox`, `--fuel n`) go along to
  warp-rust, which takes them wherever they stand. `--self` is the launcher's own and never reaches warp-rust.
- `warp test [--self] [folder]` already runs every test through `warp pure` (src/compiler/pure_tests.rs), so it passes on.
- A refusal ("outside the core: …") or any failure of the pure pipeline exits with its own status, loudly: nothing
  reruns unless asked (WARP_FALLBACK below). A program's exit status passes as it is (`exit(3)`, an Int value of a file).
- Inline code prints its value and exits 0 (`warp '6*7'` prints 42). warp-rust gives an Int value as the exit status,
  so the launcher reads inline code's output: a status equal to the Int on its last line (low 8 bits) is the value's and
  becomes 0; any other status (an error's 1, `exit(3)`) passes. A program that prints that same number last and then
  exits with it would exit 0 as well.
- `fix` and `beautify` run the warp program lib/fix/command.warp by warp-rust (`run`, the Rust compiler, as warp-rust's
  own route since card fix-in-lib does), its lib/ found as for `compile --wasm` from the current folder, so it works
  from any folder of a checkout. WARP_LIB names that lib/ for the program's `use`s. `--no-hints`: the hints would be
  about lib/fix's own source, not the user's files. `fix check` exits 1 when a file would change; plain `fix` shows the
  diff and asks `apply? y/n` on stdin (no answer: not applied); `yes` writes without asking; code instead of files is
  printed fixed.
- Where lib/ comes from for `compile --wasm` (compiler.warp's fourth argument), as warp-rust finds it: WARP_LIB, else the
  nearest lib/ holding compiler/ above the source or the current folder, else the installed lib/, else the lib/ named in
  compiler.wasm's `warp.build` section if it still exists. The compiler file: `<WARP_LIB>/../data/compiler.wasm` when
  it exists, else the installed one (`compiler-self.wasm` for self). Installed: beside warp-rust, else
  `<prefix>/share/warp` (src/modules/program_file.rs `installed_folder`, card installed-compilers).

## Refusals

warp-rust has no exit status of its own for a refusal. compiler.warp raises `outside the core: …` before the program
runs (core/compiler.warp; the emitter's text, core/emitter.warp `outside_the_core`), and warp-rust
prints that Error as the run's value, `Error("outside the core: …")`, on stdout and exits 1 (standalone.rs `run` →
src/cli/display.rs `print_and_exit`; the source excerpt goes to stderr). The launcher counts a run as refused when it
exits 1 and its whole stdout is one line starting `Error("outside the core: `: nothing of the program ran before, so
a program's own output or failure (any other status, more lines, another Error) never counts. Only under
WARP_FALLBACK=rust (and for inline code) is stdout read, through `tee`: stdout is then a pipe, not the terminal.


## Environment

| variable | effect |
|---|---|
| `WARP_PIPELINE=rust` | every command goes to warp-rust as it is: the old Rust path, environment untouched |
| `WARP_FALLBACK=rust` | when the pure pipeline refuses the program (Refusals above), the launcher prints `warp: ⚠️  WARP_FALLBACK=rust: THE PURE PIPELINE REFUSED THIS PROGRAM, RUNNING IT THROUGH RUST INSTEAD` on stderr and runs the whole command through warp-rust. Off by default. Any other failure and exit status passes as it is |
| `WARP_COMPILER=pure\|self` (default pure), or the flag `--self` | `self`: compiler-self.wasm (compiler.warp compiled by itself, `warp-rust self`) instead of compiler.wasm. Another value is an error |
| `WARP_SELF_HOSTED` | warp-rust's own (src/compiler/self_hosted.rs: front, emit, pure, pure-only), passed through untouched |
| `WARP_RUST=<binary>` | the Rust binary; else `warp-rust` beside the script (links followed, as Homebrew links bin/warp), else on PATH |
| `WARP_HOST=<binary>\|none` | the native host warp-host (notes/warp_host.md) compiles and runs programs (`run`, inline code, `compile --wasm`) instead of warp-rust's pure lane; else `warp-host` beside the script when there is one; `none`: warp-rust's pure lane. With it, compiler-self.wasm is the default and `WARP_COMPILER=pure` asks for compiler.wasm (`--compiler full`) |
| WARP_LIB, WARP_HINTS, WARP_NO_WINDOW, WARP_EMIT_REFUSALS, … | warp-rust's, unchanged |

## Adding a command

A command implemented in warp is one line in `pure_command` at the top of packaging/bin/warp:

```bash
pure_command() {
	case $1 in
		run) echo run_file ;;
		compile) echo compile_module ;;
		fix | beautify) echo lib_program fix/command.warp "$1" ;;
		# ← a new line: `<word>) echo <function> [its first words] ;;`
	esac
}
```

The function, defined below it, gets its first words, then the words after the command. A command written as a warp
program in lib/ is one line reusing `lib_program <program in lib/> "$1"`, which runs `warp-rust run <lib>/<program> --
<word> <arguments>…` (as fix and beautify do). It runs through `pure …` (the arguments of warp-rust
that run the pure pipeline: failures loud with their own status, a refusal under WARP_FALLBACK=rust passed on) and calls `pass_on` for a form it does not take,
which hands the whole command to warp-rust. A new kind of command needs one such function; a command that is a warp
program reuses one. Add an outcome check to tests/launcher/test_launcher.sh.

## Install layout

```
~/dev/bin/warp               packaging/bin/warp (the launcher)
~/dev/bin/warp-rust          the Rust binary (quick-release build of main)
~/dev/bin/compiler.wasm      beside warp-rust: standalone.rs finds them next to its own (canonical) path
~/dev/bin/compiler-self.wasm
```

In the repo: scripts/self-check.sh and scripts/lib_bench.sh run /Users/me/dev/bin/warp-rust, plain warp while warp-rust
is not installed (both need the Rust binary: self-check copies it and runs `rebuild`; the lib bench measures Rust's
lanes). Nothing in ~/dev/bin was installed or swapped by this branch.

### Integrator follow-up (integrator_tools/, outside the repo, not changed here)

- refresh_main_binary.sh `build_and_install`: install `$target/$profile/warp` as `$bin/warp-rust` (binaries list:
  `warp` → `warp-rust` as the destination name), then the launcher: `install_binary $worktree/packaging/bin/warp
  $bin/warp`. The version line reads `$bin/warp-rust --version`.
- refresh_main_binary.sh `keep_main_build`: keep `$bin/warp-rust` (with compiler.wasm) in main-builds/<commit>/ and
  link `$bin/warp-<commit>` to it (as now, a Rust binary finding its compilers through the link); `cmp` against
  warp-rust. `rebuild_worktree_compilers`: `$bin/warp-rust rebuild pure`.
- express.sh l.81: `install_compilers.sh /Users/me/dev/bin/warp-rust $PWD` (it copies the binary it is given and runs
  `rebuild`; the launcher there would be copied without its warp-rust).
- pure_chain_check.sh: `MAIN_WARP=/Users/me/dev/bin/warp-rust` (its `warp pure` passes through either way).
- warp-debug, warp-runtime, warp-page-runtime keep their names. scripts/llm/llm.py runs generated programs with
  WARP_PIPELINE=rust until the pure pipeline covers enough (card llm-pure).

## Packaging that must follow later (not changed)

Since card installed-compilers the packages ship compiler.wasm, compiler-self.wasm and lib/ in `<prefix>/share/warp`,
where both warp-rust and the launcher find them; the Rust binary is still installed as `warp`. Left to do:
- the release archives: the Rust binary as `warp-rust` plus packaging/bin/warp as `warp`.
- packaging/homebrew/Formula/warp.rb: `bin.install` warp-rust and the launcher; its test `warp pure '6*7'` passes
  through as is, `warp eval '6*7'` too.
- packaging/linux/nfpm.yaml (`./warp*` → /usr/bin/): picks both up by its glob; bash becomes a dependency.
- packaging/pypi/build_wheels.py (`warp*` binaries into `.data/scripts/`): picks both up by BINARY_PREFIX; the launcher
  finds warp-rust beside itself in the environment's bin/ (a wheel has no symlinks to follow).

## Timings (measured 2026-10-22 on the box, debug build bf03df01c, the Mac's compilers of 9fc81b9241; warm, min–median of 5)

| run | time |
|---|---|
| `warp hello.warp` (launcher → pure) | 0.26 s |
| `warp-rust pure hello.warp` | 0.25 s |
| `warp --self hello.warp` | 0.31–0.40 s |
| `warp compile --wasm h.warp` | 0.24 s |
| `warp '6*7'` / `warp-rust pure '6*7'` | 0.23 s |
| `warp-rust run hello.warp` (Rust) | 0.019 s |
| `warp version` / `warp-rust version` | 0.013 s / 0.008 s |

The launcher adds 5–10 ms; the pure pipeline costs ~0.23 s per program over Rust's, the compiler module itself running.
The investigation measured warm pure hello at 0.24–0.31 s vs Rust 0.04–0.05 s. A first run after a new compiler.wasm
lowers compiler.warp's runtime words once: 8–14 s here (debug), `warp rebuild` warms it.

## Tests

tests/launcher/test_launcher.sh (outcome checks, expected output and exit status), through the queue:
`tests/queue.sh tests/launcher/test_launcher.sh`. WARP_LAUNCHER_RUST names the Rust binary under test (default the
checkout's target/debug/warp), compiler.wasm and compiler-self.wasm beside it; staged in scratch/launcher-test/.

Pure-pipeline coverage: `tests/queue.sh <warp-rust> test tests` (or `--self`) runs every test of tests/ through the
pure pipeline and prints `m of n failed`; n − m is the number going green.

Measured 2026-10-22 on the box (tests at 0445edb5e, debug warp-rust bf03df01c, compiler.wasm of 9fc81b9241; one targeted
queue run, 20 min wall, 8 children): `6006 of 10701 failed in 1630 files`, so **4695 of 10701 tests (44 %) pass
through the pure pipeline**; 5131 of the failures are refusals ("outside the core").
