# Real threads for `go` (user decision P33, 2026-10-04)

Today (wiki/async.md, declarations::lower_tasks): a module runs on one thread, `go f(x)` runs the task to its end where
it starts, `await job` is its value, pause/stop handlers never run. The user wants real threads: WASM threads with
shared memory natively, Web Workers in the browser. This note is the design; the steps below land one at a time.

## The constraint that shapes everything
warp values are WASM GC objects ($Node structs, typed arrays). GC references cannot cross threads: the threads proposal
shares *linear memory* only, and "shared-everything threads" (shared GC types) is still a proposal that neither
wasmtime 49 nor any browser ships. So two tasks can never hold the same Node.

Consequence: **an instance per task** (an isolate, like a Worker or an Erlang process). Each task runs in its own
instance of the same compiled module, on its own thread; arguments and results are *copied* between instances. That
matches wasp's value semantics (index assignment already copies, aliases never change), so it costs no language rule.

## Semantics
- `job = go f(x)`: starts f(x) in a new instance on a new thread and returns at once with the task.
- `await job`, and `job` used as its value (the auto-cast): blocks until the task finishes, then is its result.
- Arguments and results are values: copied, never shared. A task sees the main-level variables as they were when the
  module started (its own instance), not later changes; a write to a `global` inside a task stays in that task.
  Sharing state goes through results, signals and (step 6) shared typed arrays.
- `stop job` / `cancel job`: the task traps at its next check point; `await` of a stopped task is the error
  `task stopped`, catchable with `try`. `pause job` / `resume job`: the task parks at its next check point.
- `once job finishes: …` and `on job.stop: …` run on the thread that started the task, at its next check point
  (an `await`, a loop back-edge, a call of a task word), never concurrently with that thread's own code.
- Errors inside a task are its result: `await` gives the error value (errors are values, decision #1).

## Native (wasmtime)
- One `Engine`, one compiled `Module` (both Send + Sync); a task is `std::thread::spawn` with its own `Store`, instance
  and fuel budget. The host keeps the task table: id → JoinHandle / result slot / control flags.
- Host imports: `task.spawn(function, arguments) -> id`, `task.await(id) -> result`, `task.control(id, op)`.
  The function goes by name through an exported entry `task_entry(name, arguments)` generated per program for the
  functions a `go` starts (the program knows them statically: lower_tasks already collects them).
- Crossing values: a value leaves an instance as bytes and enters the other one rebuilt. Step 1 carries numbers and
  texts (i64, f64, UTF-8 bytes: no parser needed in wasm). Lists and objects need a serializer out (list_text exists)
  and a reader in: either a runtime wasp reader in wasm, or the host reads the text (wasp_parser) and builds the Node in
  the target instance through exported constructors (new_int, new_text, list cons) — the second reuses the host parser
  and gc_traits, and is step 3.
- Stop and pause: epoch interruption. `Config::epoch_interruption(true)`, each task store gets a deadline; a ticker
  thread increments the epoch; the store's epoch callback reads the task's flags: stop → trap with `task stopped`,
  pause → block on a condvar until resume, else continue. No change to the emitted code.
- Check points for handlers on the starting thread: the host imports above, plus a cheap `task.poll()` the emitter adds
  at loop back-edges only when the program uses `once`/`on` with tasks.

## Browser (web/playground)
- Each task is a Worker running the same module bytes with the same host.js imports.
- A blocking `await` needs `Atomics.wait` on a SharedArrayBuffer, which needs cross-origin isolation (COOP/COEP
  headers). GitHub Pages cannot set headers: use a service worker that adds them (the coi-serviceworker approach), or
  JSPI (JS promise integration) to suspend the calling instance instead of blocking it. Without either, the playground
  keeps today's behaviour (the task runs where it starts) and says so once.
- Stop: `worker.terminate()`. Pause: a flag in shared memory checked at the poll points.

## Status
- Step 1 done (2026-10-04, 487dd2f1): functions of up to four Ints that give an Int; declarations::lower_tasks marks,
  declarations::resolve_tasks decides (inferred kinds), src/tasks.rs runs. `await` binds like a unary minus.
- Step 2 done (2026-10-04): stop/cancel/pause/resume through epoch interruption (util::task_engine, only for modules
  importing the task words, checked by wasmparser before compiling). Awaiting a stopped task is the error
  `task f: task stopped`, but not yet catchable with `try`: a host function cannot throw the `wasp_error` tag, so the
  wasm side would have to check a failure flag after task_await and call a runtime error function.

- Step 3 done (2026-10-04): numbers, texts, characters, lists and keys of them cross (tasks::TaskValue, rebuilt in
  each instance through its exported constructors new_int … new_list; read back with wasm_reader::node_in). Floats
  passed to a float parameter are converted at the start (`x as float`), as a call converts them. Still inline: list
  parameters (the list ABI passes arrays, not Nodes), function values; exact numbers beyond the fixnums (ratios, big
  integers) are a clear error, being handles into the memory of their instance.

- Catchable (2026-10-04): `await` is `task·check(task_join(job), task_failure(job)); task_await(job)`: a failed task
  raises its message from wasm as a `returned_error` with trap detail, which `try` catches.
- Lists (2026-10-04): a task of values starts through `f·node(arguments·node)` (declarations::with_node_wrappers): one
  argument list, so neither host needs parameter types, and the emitter converts into f's own convention (arrays);
  the Int words carry only surely-Int arguments (literals, variables assigned Int literals).
- Step 4 done (2026-10-04): `once job finishes: …` / `on job.stop: …` of a task on a thread: a check after every later
  statement of the block (task_status), the handler runs once; the block's end waits for the task (before its last
  statement, which keeps the block's value).
- Step 5 done (2026-10-04): host.js runs tasks on a pool of Workers (task-worker.js) made while the program's worker is
  idle (a Worker starts only once its creator returns to the event loop), results as JSON in a growable
  SharedArrayBuffer awaited with Atomics.wait; without cross-origin isolation a task runs at once in a fresh instance.
  The test server sends COOP/COEP; the playground gets them from coi-serviceworker.js (one reload). Measured: two
  one-second tasks take 1.0 s in the playground. Pause/resume in the browser: not yet (no epoch checks there).

- Step 6 done (2026-10-04, P44): `shared xs = int[n]` (lowering/shared_arrays.rs): host words shared_new/get/set/add/
  count over arrays the host holds for the run (src/shared.rs: Arc<[AtomicI64]>, host.js: BigInt64Array over a
  SharedArrayBuffer handed to the task Workers). Not wasm shared memory: an imported memory would take index 0 ahead of
  the module's own, and every memory instruction assumes 0; the host call per access is the price.

## Steps (each its own commit with tests)
1. Native spawn/await for functions of numbers and texts: lowering `go f(x)` → `task_spawn`, `await` → `task_await`
   typed by f's result kind; the runner's task table; tests that two tasks really overlap (each sleeps 200 ms, total
   well under 400 ms) and that results are right. Programs without `go` emit no task imports.
2. stop / cancel / pause / resume through epoch interruption; `await` of a stopped task is the catchable error.
3. Lists and objects across tasks (host-side reading into the target instance).
4. `once job finishes` / `on job.stop` handlers at check points on the starting thread.
5. Browser: Workers + cross-origin isolation service worker; fallback to inline tasks with a notice.
6. Shared typed arrays: `shared xs = int[1000]` in shared linear memory (the threads proposal) with atomic adds, for
   real data parallelism; GC values stay copied.

## Open questions (for open_decisions.md when a step needs them)
- Does a task see main-level variables assigned before the `go` (snapshot by copying them in) or only the module's
  initial values? Proposed: the values at `go` time, copied in, like arguments.
- How many threads: one per task (simple, step 1) or a pool sized to the cores.
