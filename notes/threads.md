# Real threads for `go` (P33)

Roadmap card: threads (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

Native `go` tasks run on a fixed pool of host threads; browser tasks run in Web Workers. Each task has a separate
WebAssembly instance; shared values use host-owned atomic arrays.

## The constraint that shapes everything
warp values are WASM GC objects. GC references cannot cross threads: the threads proposal shares linear memory only,
and shared-everything threads (shared GC types) ship nowhere. So two tasks never hold the same Node: an instance per
task (an isolate, like a Worker or an Erlang process), arguments and results copied between instances. That matches
warp's value semantics, so it costs no language rule.

## Semantics
- `job = go f(x)`: queues f(x) in a new instance on the run's pool and returns at once with the task.
- `await job`, and `job` used as its value: blocks until the task finishes, then is its result. `await` binds like
  a unary minus.
- Arguments and results are copied. A task sees the main-level variables as the module started them, not later
  changes; a write to a `global` inside a task stays in that task. Sharing goes through results, signals and
  `shared` values.
- `stop job` / `cancel job`: the task traps at its next check point; `await` of a stopped task is the error
  `task f: task stopped`, catchable with `try`. `pause job` / `resume job`: the task parks at its next check point.
- Errors inside a task are its result: `await` raises its message as a `returned_error`, which `try` catches
  (`task·check(task_join(job), task_failure(job)); task_await(job)`).
- `once job finishes: …` / `on job.stop: …` run on the starting thread, as a check after every later statement of
  the block (task_status); the block's end waits for the task (before its last statement, which keeps the block's
  value).
- Job lists (P47): `jobs.add(go f(i))` keeps the started task unawaited (declarations::job_lists); a read of the list
  awaits every result (`task·list`), `jobs#i` that job only (`task·element`), `await all xs` reads xs once.
  `count(jobs)`, `#jobs` count without awaiting.
- Shared values (P106): `shared done = false`, `shared n = 0`, `shared x = 0.5` (and `atomic …`) are one-cell shared
  arrays: reads `shared_get(n, 1)`, `n = v` / `n += v` atomic set and add. `shared xs = int[n]` / `float[n]` (P44,
  lowering/collections/shared_arrays.rs): host words shared_new/get/set/add/count (+ …f for floats, bits in the same
  cells) over arrays the host holds (src/host/shared.rs: Arc<[AtomicI64]>; host.js: BigInt64Array over a
  SharedArrayBuffer). Not wasm shared memory: an imported memory would take index 0 ahead of the module's own. A go
  block reading a shared name gets it as a parameter; only `shared` names cross between tasks.

## Native (wasmtime)
- One Engine, one compiled Module; a run has a fixed pool of twice the available parallelism, since task host calls
  may block (one per core missed the 300-sleeping-task bound). Each task gets its own Store, instance and fuel budget.
  A worker awaiting a nested task runs queued work itself, so a full pool of parent tasks cannot deadlock.
- declarations::lower_tasks marks, declarations::resolve_tasks decides (inferred kinds), src/host/tasks/mod.rs runs.
  A task of values starts through `f·node(arguments·node)` (declarations::with_node_wrappers): one argument list, so
  neither host needs parameter types; the Int words carry only surely-Int arguments.
- Values cross as tasks::TaskValue (numbers, texts, characters, lists, keys, closures, exact numbers), rebuilt in the
  target instance through its exported constructors (new_int … new_list), read with wasm_reader::node_in. Floats passed
  to a float parameter are converted at the start.
- Stop and pause: epoch interruption (util::task_engine, only for modules importing the task words, checked by
  wasmparser before compiling); the store's epoch callback reads the task's flags. No change to the emitted code.
- Split float maps: `go xs.map(x => kernel)` over a float array starts no tasks: task_split_mapf maps one slice per
  core in fresh instances on scoped threads (src/host/tasks/mod.rs split_map, notes/simd.md). Other `go xs.map(f)`
  use map_in_tasks.

## Browser
- host.js runs tasks on a pool of Workers (task-worker.js) made while the program's worker is idle (a Worker starts
  only once its creator returns to the event loop); results as JSON in a growable SharedArrayBuffer awaited with
  Atomics.wait. That needs cross-origin isolation: the test server sends COOP/COEP, the playground gets them from
  coi-serviceworker.js (one reload); without it a task runs at once in a fresh instance.
- Pause/resume: each task Worker gets a control word (Int32Array on a SharedArrayBuffer); `task_poll`, called where
  every loop starts in a program that controls tasks, waits on it while paused. Stop: `worker.terminate()`.
- Stalled tasks: finishedTask waits in 100 ms slices and in between runs the run's stalled tasks itself: an unstarted
  one (its Worker could not instantiate it, e.g. out of Wasm memory) or one not taken within 15 s (the task Worker
  sets the control buffer's second word when it takes a task), replacing that Worker and saying why on stderr and the
  console. task-worker.js writes a failure record for any exception of its own. Channel waits (channelWait) run the
  stalled tasks too.
