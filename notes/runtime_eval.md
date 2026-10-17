# Run-time `!`

`x!` on a quote whose value is known only at run time (wiki/charged.md §5): data read at run time, a `block`
parameter called with different blocks, `xs#2!`. The feasibility study (options interpreter / run-time compile /
hybrid, the shared-store design, optimisation invariants, effort table): notes/OLD/runtime_eval_study.md.

## As built: copying through the host
- Step 0: `x : a+b` over names defined nowhere warns that `x!` can never run it
  (src/lowering/control/run_time_blocks.rs).
- `interpret e` (the spec's word for `x!!`): of a constant block it is `x!!` (blocks.rs inlines it); of anything else
  the host word `run_block(e, "a b", [a, b])`, the variables assigned before it as a snapshot (the block reads them,
  never writes them).
- Step 2: the host (natively src/host.rs `run_block`, in the browser host.js → `web_eval_block` of a second compiler
  instance loaded on first use) reads the Nodes, binds the names (`pipeline::eval_block`), compiles and runs the
  block in a module of its own, and builds the value back in the program through its constructors (TaskValue). A
  failure is the program's error "the block q+1 failed: undefined variable: q".
- The block is pure: no capability but libm. Running it is the effect `Eval`; a function containing a run-time `!`
  has it too. `warp compile` warns that a module calling run_block needs a warp host.
- The program's function definitions travel along as data, so the block may call them; exact ratios and big Ints
  come back (EXACT_BUILDERS); pairs come back in the browser too.
- `e!` of an expression known only at run time (`xs#2!`) and of a name holding an element of a list of data runs
  through run_block (`lower_run_time_bangs`, between blocks.rs and mutation.rs); ø is "unwrapped ø", a value is
  itself. `!` also decides at run time where the kind is not known statically: an untyped parameter (`f(b) := b!`)
  and a result of the program's functions (`g()#1!`). A block run in a body sees the parameters and every main-level
  variable.

## Measured
Copying is not the cost, compiling is: 100 × `interpret(xs#1)` took ~200 ms per run_block with or without a
20 000-element list in scope. So the compiled blocks are cached: pipeline::eval_block binds only the names the block
or the definitions mention; natively each number among them becomes `block·value(i) as int|float`, a host word
reading this run's values (host.rs with_block_values), so the same block with other numbers is the same program:
compiled once per thread (BLOCK_MODULES, by its text; `pipeline::compiled_blocks()` counts the misses), machine code
kept by run/module_cache.rs. Each thread keeps one linker per engine and import families and clones it.
probes/bang_cache/loop.warp (100 × `xs#1!`, debug): 0.24 s cold, 0.09 s warm (was 1.68 s / 0.24 s).
tests/control/test_run_time_block_cache.rs pins it.

## Not yet
- Sharing GC objects instead of copying (child instance in the program's store, importing its memory, text heap and
  globals; V8 probed in probes/runtime_eval/, wasmtime not).
- Texts and lists as block values: written in, so a changing text compiles again. The browser writes all values in.
- Assigning the `!` site's locals (recommended: never; only declared globals), grants narrower than the run's, and
  taint for data read at run time.
