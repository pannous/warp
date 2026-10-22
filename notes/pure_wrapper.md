# The `warp` launcher: programs through the pure pipeline (cards warp-launcher, warp-rust-install)

`warp` is a bash script, packaging/bin/warp. It runs programs through the warp compiler written in warp
(lib/compiler/compiler.warp built as compiler.wasm, run as `warp-rust pure` runs it) and hands every other command to the
Rust binary, installed as `warp-rust`, unchanged (`exec`: same arguments, stdio and exit status). Plan A of the
investigation of 2026-10-22; no Rust changed. Progress is how many tests pass through the pure pipeline, not how close
it comes to Rust's output.

## Routing

| command | runs as |
|---|---|
| `warp f.warp [-- args]`, `warp run f.warp [-- args]` | `warp-rust pure f.warp [-- args]` |
| `warp <code>` (a word that is no command, file or option: `warp '6*7'`) | `warp-rust pure <code>` |
| `warp compile --wasm f.warp` | `warp-rust compiler.wasm -- f.warp f.wasm <folder of f> <lib>`, then `compiled N bytes to f.wasm` |
| anything else: `warp`, `warp x.wasm`, `version`, `help`, `test`, `rebuild`, `serve`, `dev`, `app`, `deploy`, `eval`, `lower`, `parse`, `repl`, `compile --exe/--site/…`, `build`, a plain word (`warp pi`), `fast`, `fuel n` | `warp-rust` with the same words |

- Leading flags (`--no-hints`, `--strict[=level]`, `--warnings=…`, `--no-ask`, `--sandbox`, `--fuel n`) go along to
  warp-rust, which takes them wherever they stand. `--self` is the launcher's own and never reaches warp-rust.
- `warp test [--self] [folder]` already runs every test through `warp pure` (src/compiler/pure_tests.rs), so it passes on.
- A refusal ("outside the core: …") or any failure of the pure pipeline exits with its status, loudly: nothing reruns
  unless asked (WARP_FALLBACK below).
- Inline code on the pure pipeline prints its value as a file run does and exits with an int value as status
  (`warp '6*7'` prints 42, exits 42); warp-rust's own `warp <code>` prints `» 42` and exits 0.
- Where lib/ comes from for `compile --wasm` (compiler.warp's fourth argument), as warp-rust finds it: WARP_LIB, else the
  nearest lib/ holding compiler/ above the source or the current folder, else the lib/ named in compiler.wasm's
  `warp.build` section if it still exists. The compiler file: `<WARP_LIB>/../data/compiler.wasm` when it exists, else
  the one beside warp-rust (`compiler-self.wasm` under `--self`).

## Environment

| variable | effect |
|---|---|
| `WARP_PIPELINE=rust` | every command goes to warp-rust as it is: the old Rust path, environment untouched |
| `WARP_FALLBACK=rust` | when the pure pipeline fails (refusal, error, nonzero status), the launcher prints `warp: ⚠️  WARP_FALLBACK=rust: THE PURE PIPELINE FAILED (exit n), RUNNING THROUGH RUST INSTEAD` on stderr and runs the whole command through warp-rust. Off by default. A program that printed before failing prints again; a program whose value is a nonzero int counts as failed |
| `WARP_SELF_HOSTED=1`, or the flag `--self` | compiler-self.wasm (compiler.warp compiled by itself, `warp-rust self`) instead of compiler.wasm. The launcher unsets WARP_SELF_HOSTED=1 before running warp-rust, where the variable means the in-process pure lane (src/compiler/self_hosted.rs: front, emit, pure, pure-only); other values pass on unchanged |
| `WARP_RUST=<binary>` | the Rust binary; else `warp-rust` beside the script (links followed, as Homebrew links bin/warp), else on PATH |
| WARP_LIB, WARP_HINTS, WARP_NO_WINDOW, WARP_EMIT_REFUSALS, … | warp-rust's, unchanged |

## Adding a command

A command implemented in warp is one line in `pure_command` at the top of packaging/bin/warp:

```bash
pure_command() {
	case $1 in
		run) echo run_file ;;
		compile) echo compile_module ;;
		fix) echo run_tool ;;   # ← the new line: `<word>) echo <function> ;;`
	esac
}
```

The function, defined below it, gets the words after the command. It runs through `pure …` (the arguments of warp-rust
that run the pure pipeline: failures loud, WARP_FALLBACK honoured) and calls `pass_on` for a form it does not take,
which hands the whole command to warp-rust. A new kind of command needs one such function; a command that is a warp
program reuses one. Add an outcome check to tests/launcher/test_launcher.sh. `fix` and `beautify` are being written in
warp (Warp Code 2) and have no entry yet.

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
- warp-debug, warp-runtime, warp-page-runtime keep their names. scripts/llm/llm.py (`WARP`, default
  /Users/me/dev/bin/warp, `run <file>`) then runs generated programs through the pure pipeline: wanted or not is the
  user's call (WARP=/Users/me/dev/bin/warp-rust keeps Rust).

## Packaging that must follow later (not changed)

The released packages ship the Rust binary as `warp` with no compiler.wasm and no lib/, so they keep it that way until
they can carry the pure pipeline:
- release workflow and archives: build compiler.wasm / compiler-self.wasm (`warp rebuild pure|self`) and ship them, and
  the lib/ compiler.warp reads at run time (prelude.warp, compiler/runtime.warp …); an installed warp-rust's own lib/ is
  the build machine's (CARGO_MANIFEST_DIR) unless WARP_LIB names one.
- packaging/homebrew/Formula/warp.rb: `bin.install` the binary as warp-rust plus packaging/bin/warp, the compilers in
  bin/ beside warp-rust (or libexec/ with WARP_RUST), lib/ in share/; the `warp eval '6*7'` test passes through as is.
- packaging/linux/nfpm.yaml (`./warp*` → /usr/bin/): warp-rust, the launcher and the compilers; bash is a dependency.
- packaging/pypi/build_wheels.py (binaries into `.data/scripts/`): the same files; the launcher finds warp-rust beside
  itself in the environment's bin/.

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
