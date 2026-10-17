# go blocks and sleep durations

`def hi: {sleep(1000 ms);print('hi')}; go { hi() }; print('faster')` prints `faster`, then `hi` a second
later; the program waits for its tasks before it ends (tasks::TaskTable::join_all).

- `go { … }` (src/lowering/control/go_blocks.rs, first source pass): the block becomes `go·block·N(v…) := { … }`, where v… are
  the variables it reads that are assigned before it (or parameters of the enclosing function), and the statement is
  `go go·block·N(v…)`. The task gets their values at its start, copied: no sharing (notes/threads.md). After that it is
  an ordinary `go f(x)`: lower_tasks / resolve_tasks decide the thread path. `job = go { … }; await job` works too.
  A program defining its own `go` keeps it.
- Tasks of no arguments: tasks.rs builds the wrapper `f·node(arguments)`'s empty `[]` as ø.
- `def hi: {…}` / `fun f(x) {…}` start as tasks: lower_tasks takes the function names from extract_user_functions too.
- `sleep(1000 ms)`, `sleep 1s`, `sleep(2 seconds)` (units::lower_sleep_durations): a constant duration is its
  milliseconds; quantities do not reach run time otherwise (notes/units_runtime.md).
- `sleep(…)` is a statement like `print`, so `{ sleep(10); "hi" }` runs and gives "hi" (analyzer is_statement).
- `after C return V` (wiki/thread.md, go_blocks::task_phrases): `go { while not (C) { sleep(1) }; V }`, a task polling
  every millisecond; C must read shared values (P106), a copied variable is a loud error. `cancel last` stops it.
  `await job or y` is `try await job else y`. Parser precedence: `x = after n == 5 return v` needs parentheses for now
  (card after-precedence).
- A go block's inputs come in renamed, `n` as the parameter `n·in`: a parameter named like a function of the program
  read as that function otherwise.
- `@parallel xs.map(f)` (and `xs.map(f) @parallel` once the parser annotates the expression before a trailing
  attribute): `(jobs = []; for item in xs { jobs.add(go { f(item) }) }; await all jobs)`, f a function or a function
  value; results in order.
- A task that fails while nobody awaits it ends the run with its error (tasks::TaskTable::join_all, host.js joinTasks);
  a stopped task does not.
- Data parallelism (the second use of `go`, src/lowering/control/parallel.rs): `go xs.map(f)`, `xs.map(f)
  @parallel` and `go for x in xs { … }` split xs into 8 slices (PARALLEL_CHUNKS), each a go block; a map joins its
  results in order, a loop ends when every slice is done. A loop body updating a non-shared outer variable warns.
  `@parallel square all xs` is the same map (the parser reads it as `(square all) xs`, so the annotation is found on
  the head's head); `@parallel` on any other form (`@parallel square xs`) warns that it runs in sequence:
  go_blocks runs before broadcasting, so it cannot know whether square broadcasts.
  Later backends (SIMD, GPU via WebGPU/wgpu for pure numeric f over typed arrays) can take the same forms.
- `await all [go f(1), go f(2)]` (Promise.all, asyncio.gather; declarations::awaited_starts): every task starts first,
  then each is awaited, `(go·job·1 = go f(1); go·job·2 = go f(2); [await go·job·1, await go·job·2])`. `await any [go a(), go b()]` (Promise.race): every task
  starts, a loop polls task_status every millisecond (Tasks::race) and awaits the first finished or failed one; the
  others keep running. Ported async cases: probes/async_ports.md, tests/control/test_async_ports.rs. Not yet:
  `await any [a, b]` of task variables (card await-any-vars), Go channels (P155).
- `await first [go a(), go b()]` (P153) is the first task's result, with a got-it note naming
  `await any`. `await job within 100 ms or 0` (P154, asyncio.wait_for): polls the job's status until the deadline, then
  stops it and gives the `or` value (`raise timeout` without one, which `try` catches); a got-it note says so where it
  is written. As `x = await job within … or v` it arrives as go_blocks' try·else (Tasks::within_or).

## Parallel let (notes/bend.md)
- `a b = f(x) g(y)` arrives as `a (b = f(x)) g(y)`; go_blocks.rs hands it to parallel.rs `let_in_tasks` when every
  value is a call by name and names and calls are as many (two or more): `if task_forks() { j0 = go f(x); b = g(y);
  a = await j0 } else { a = f(x); b = g(y) }`, the last call on the current thread as in BendRT.
- `task_forks()` (host.rs) is the cutoff: natively 1 while the run's TaskTable has fewer running tasks (nested ones
  too) than `available_parallelism`, so a recursive split forks only its top levels; in the browser 1 in the program
  and 0 inside a task (a Worker does not see the others).
- `go` inside an if-expression (`j = if c { go f(3) } else { f(3) }; await j`) gives a wrong value (1), so the cutoff
  duplicates the whole statement into both branches instead.
- Timing (probes/parallel/tree_sum.warp vs tree_sum_sequential.warp, 16 leaves of 30M steps, 10 cores, loaded machine):
  0.86 s vs 1.72 s.
