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
