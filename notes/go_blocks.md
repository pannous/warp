# go blocks and sleep durations (async agent, 2026-10-06)

User program: `def hi: {sleep(1000 ms);print('hi')}; go { hi() }; print('faster')` prints `faster`, then `hi` a second
later; the program waits for its tasks before it ends (tasks::TaskTable::join_all).

- `go { … }` (src/lowering/go_blocks.rs, first source pass): the block becomes `go·block·N(v…) := { … }`, where v… are
  the variables it reads that are assigned before it (or parameters of the enclosing function), and the statement is
  `go go·block·N(v…)`. The task gets their values at its start, copied: no sharing (notes/threads.md). After that it is
  an ordinary `go f(x)`: lower_tasks / resolve_tasks decide the thread path. `job = go { … }; await job` works too.
  A program defining its own `go` keeps it.
- Tasks of no arguments: the wrapper `f·node(arguments)` got a null for the empty list; tasks.rs builds `[]` as ø.
- `def hi: {…}` / `fun f(x) {…}` start as tasks: lower_tasks takes the function names from extract_user_functions too.
- `sleep(1000 ms)`, `sleep 1s`, `sleep(2 seconds)` (units::lower_sleep_durations): a constant duration is its
  milliseconds; quantities do not reach run time otherwise (notes/units_runtime.md).
- `sleep(…)` is a statement like `print`, so `{ sleep(10); "hi" }` runs and gives "hi" (analyzer is_statement).
- `after C return V` (wiki/thread.md, go_blocks::task_phrases): `go { while not (C) { sleep(1) }; V }`, a task polling
  every millisecond; C must read shared values (P106), a copied variable is a loud error. `cancel last` stops it.
  `await job or y` is `try await job else y`. Parser precedence: `x = after n == 5 return v` needs parentheses for now
  (card after-precedence).
- A go block's inputs come in renamed, `n` as the parameter `n·in`: a parameter named like a function of the program
  read as that function (card param-shadows).
- `@parallel xs.map(f)` (and `xs.map(f) @parallel` once the parser annotates the expression before a trailing
  attribute): `(jobs = []; for item in xs { jobs.add(go { f(item) }) }; await all jobs)`, f a function or a function
  value; results in order.
- A task that fails while nobody awaits it ends the run with its error (tasks::TaskTable::join_all, host.js joinTasks);
  a stopped task does not.
- Data parallelism (user, 2026-10-06: dual use of `go`, src/lowering/parallel.rs): `go xs.map(f)`, `xs.map(f)
  @parallel` and `go for x in xs { … }` split xs into 8 slices (PARALLEL_CHUNKS), each a go block; a map joins its
  results in order, a loop ends when every slice is done. A loop body updating a non-shared outer variable warns.
  Later backends (SIMD, GPU via WebGPU/wgpu for pure numeric f over typed arrays) can take the same forms.
- `await all [go f(1), go f(2)]` (Promise.all, asyncio.gather; declarations::awaited_starts): every task starts first,
  then each is awaited, `(go·job·1 = go f(1); go·job·2 = go f(2); [await go·job·1, await go·job·2])`. It gave the task
  handles [1 2] before. Ported async cases: probes/async_ports.md, tests/control/test_async_ports.rs. Not yet:
  `await any` (race), deadlines (`within`), Go channels.
