# Running the warp compiler under plain wasmtime

The self-compiled compiler, compiler-self.wasm, is an ordinary WASI preview 1 command: wasmtime runs it with no warp
host, it compiles a .warp file to a .wasm file, and that module runs under wasmtime too (card standalone, decision
wasm-standalone; design: notes/standalone_compiler.md).

## How to

```bash
# 1. build compiler-self.wasm (prints `built <path>`: next to the warp binary, or <checkout>/data/ for a worktree)
/Users/me/dev/bin/warp rebuild self

# 2. compile: <source> <output> <folder of the source's uses> <lib folder>
wasmtime run -W gc=y,function-references=y,exceptions=y,unknown-imports-default=y --dir / --dir .::. \
  /Users/me/dev/bin/compiler-self.wasm hello.warp hello.wasm . /Users/me/dev/angles/warp/lib

# 3. run what it wrote
wasmtime run -W gc=y,function-references=y,exceptions=y hello.wasm
```

- `--dir /` lets it read lib/ and the source by absolute path, `--dir .::.` gives it the working folder.
- `unknown-imports-default=y` answers the few host words it may import with defaults: `kept`/`keep` (the native
  warp's module cache) keep nothing, so under plain wasmtime every run is cold: it lowers runtime.warp anew each time
  (seconds to a minute), where `warp self` reuses the kept words; `local_offset` is 0 (UTC).
- The GC, function-references and exceptions proposals are needed for both modules.
- The modules import only the WASI calls they reach (args, environ, path_open, fd_read, fd_write, fd_close …);
  `print`, files, `use os` (args, env) all run over WASI. Float `^` is musl's pow ported to warp
  (lib/compiler/runtime/extended/pow.warp), so no libm import either.
- Under plain WASI there is no network (`read` of a URL fails loudly) and no module cache (`kept` is always ø).
  A WASI file cache would make it warm (card wasi-module, Later).

## Commands

The same compiler answers warp's commands (lib/compiler/commands.warp, card wasm-commands), under plain wasmtime and
as `warp pure <command>` / `warp self <command>`:

```bash
W="wasmtime run -W gc=y,function-references=y,exceptions=y --dir / --dir .::. /Users/me/dev/bin/compiler-self.wasm"
$W help                  # the commands it has
$W parse "1+2*x"         # (+ 1 (* 2 x)), as `warp parse` shows it
$W lower "x=3;x+1"       # the program after the lowering passes
$W repl                  # each line compiled with the kept declarations
warp self repl           # the same REPL, each line's module run by the native warp
```

- Plain WASI has no way to instantiate a module, so under wasmtime `repl` compiles each line and says that running it
  needs the native warp; `run`, `eval`, `test` refuse the same way. `warp pure|self repl` runs each line.
- Commands needing a host (serve, fetch, app, lsp …) and those not yet written in warp (format, lint, types, verify)
  refuse naming the native warp.

Outcome tests: tests/parser/test_warp_commands.rs; tests/parser/test_warp_wasm_alone.rs (ignored as slow, it builds compiler-self.wasm; run it with
`--ignored` at opt-level 2: `scripts/cloud-test.sh --optimized <branch> <tip> -- compiler_wasm_runs_under_plain_wasmtime --ignored`).
