# Tasks: go blocks, await, channels

Decision N29 (2026-11-02, card go-cooperative): `go {…}` is a cooperative task, no thread, natively and in the page
alike. lib/tasks.warp (a std module: "tasks" in core/passes/rewrites.warp std_module_names, so its `where program` rule
runs) holds the rules and the scheduler.

- `go { a; sleep(t); b }` is `task_go(() => { a; task_sleep(t, () => { b }) })`: the block is split at its own
  top-level sleeps (rule word task_steps in lib/rule_words.warp). Each step runs to the next sleep, which queues the rest,
  due t later.
- A task starts when main yields: main's sleep (the prelude's sleep goes through the hook slept(ms), which tasks
  replaces; units' sleep too), an await, a channel's receive or `for v in ch`, and main's end (program rule tasked:
  the last statement becomes tasks_end(…), which runs the queue dry and keeps the program's value).
- The scheduler (tasks_run_while) runs the earliest due step first and really sleeps until it is due.
- A sleep inside a function a task calls is not split: it holds every task until it wakes, said once as a warning.
  Not yet split either: a sleep inside an `if` or a loop of the block.
- `go f(x)` is `go { f(x) }` (decision go-unified): a task like a block's, started when main yields. A task is read as
  its value only by `await` (`await job + 1`, `await all [a, b]`), never implicitly.
- `await job` runs the tasks until it is done and gives its value, raising its error again; `await` takes the operand
  before an operator (`await a + await b`, rule word awaited_operand). `await all xs` gives the values, `await any`
  and `await first` the first done's, and `await job within t or d` gives the value, or d after t. `stop job` and
  `cancel job` take it out of the queue. A value that is no task passes through every await.
- A task's error that nobody awaited ends the program at main's end ("a task failed and nobody awaited it").
- A task keeps what a module holds per task (task_context: sound's voice), its starter's at `go`, swapped in for
  each step.
- Channels (lib/channels.warp `use tasks`): receive() lets the tasks run until a value arrives, and refuses loudly
  once none is left to send.

Tests: test/control/test_cooperative_tasks.pairs.warp (ordering), test_go_blocks, test_async_ports, test_channels.
The Rust-era thread notes (TaskTable, Workers, parallel let, `go xs.map(f)` slices) are on branch rust-bootstrap.
