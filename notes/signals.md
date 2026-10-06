# Signals as first-class citizens (investigation 2026-10-06, branch signals)

User: "investigate how we could make signals first-class citizens in Warp". Spec: wiki/signal.md, wiki/thread.md,
wiki/charged.md. Board card signals-first.

## What exists on main (the pieces are there, unconnected)

| piece | where | what it is in signal terms |
|---|---|---|
| `z := y*y` (P71) | lowering/getters.rs | a **derived value** (Solid `createMemo`, Vue `computed`, Svelte `$derived`) without a cache: pulled, recomputed at every use |
| `once cond {…}`, `whenever cond {…}`, `on set x {…}` | lowering/variable_signals.rs | **effects** (Solid `createEffect`, Vue `watch`): a check inserted after each later write of a variable the condition reads, decided at compile time |
| `after tested: …`, `before save: …` | variable_signals.rs | listeners on calls (aspect oriented), static |
| `once job finishes: …`, `on job.stop: …` | declarations.rs TaskHandlers | task events, checked after each later statement through task_status |
| `shared done = false` (P106), `after done return x` | shared_arrays.rs, go_blocks.rs | the only state tasks share; `after` waits by polling it in a task |
| `raise` / `throw` | pipeline.rs RAISE_WORDS | errors (exceptions), not yet signals in the wiki's sense |
| effects analysis | effects.rs | knows State/IO/…; listener bodies are not yet counted at the writes that trigger them |

So warp already has push (listeners run after writes) and pull (getters compute on read): the push-pull model that
Preact/Solid signals use, done at compile time like Svelte 5's compiler, with zero runtime objects. What is missing
for "first class": listeners on derived values, change detection, batching, writes inside functions, signals as values
that can be passed around, named event signals (`raise stop the machine` → `on stop the machine`), and crossing tasks
and the browser page.

## How others do it, and what fits wasp

| system | model | fits wasp? |
|---|---|---|
| Solid / Preact signals | runtime cells, auto-tracked dependencies (read inside an effect subscribes), push dirty + pull values, glitch-free by topological order, `batch()` | the semantics yes; the runtime tracking only where static analysis cannot see |
| Svelte 5 runes | `$state`, `$derived`, `$effect`, compiled to signals | closest: a compiler that knows the reactive names; wasp can do without the `$` marks because the whole module is known |
| Vue refs | `ref(x)` boxes, `.value`, `computed`, `watch` | the box is what an *escaping* signal needs (`$Signal` below), never written by the user |
| Elm / FRP | values over time, pure update functions, no mutation | inspiration for `whenever` being declarative; full FRP clashes with wasp's plain assignment |
| Qt signals/slots | named events with payload, `connect(sender, signal, receiver, slot)` | the wiki's `raise name{data}` / `on name {…}`: an event is a wasp node, so its payload is data |
| Go channels | values sent between goroutines, `select` | the task side: an event raised in a task is a message to the starting thread (copied like every task value, P33) |

Data-format-first consequence: an event signal is a **node**: `stop the machine{time:now reason:"…"}` is a tag with
data, so it serializes across tasks, the host, the page and the network with the existing machinery (TaskValue,
JSON, wasp text). A state signal is a **variable**: nothing new to write, `x = 3` stays `x = 3`.

## What a signal is in warp

1. **Every variable is observable; only observed ones cost anything.** No `signal` keyword: a variable becomes a
   signal when something listens to it, and the compiler inserts the notification at its writes. Unobserved programs
   compile exactly as today (P109).
2. **`:=` is the derived signal.** `total := a + b` already recomputes at each use (P71); listeners on `total` watch
   `a` and `b` (phase 1, done). Glitch-free by construction: a derived value has no cache, so it can never be stale.
   A cache with a dirty flag (memo) is an optimization the compiler may add later when the body is pure (effects.rs).
3. **Listeners are the effects**: `whenever cond {…}` (each time it holds after a change), `once cond {…}`,
   `on set x {…}` (each write), `on change x {…}` (each write that changed the value, phase 1, done), `after f`/`before f`.
   Inside: `value` (alias `signal`, `event`) is the new value.
4. **Events**: `raise name{data}` sends the event, `on name {…}` receives it (`event` is the payload node). With no
   `on name` handler in the program, `raise` stays today's catchable exception (P110). A write of x is the event
   `set x`, so `on set x` is one case of the general rule.
5. **Type**: a variable keeps its value type `T`. Only a signal that *escapes* as a value (passed to a function that
   subscribes to it, kept in a list) has the type `signal T` and a runtime representation (phase 5).

### Syntax that reads naturally (notes/welcoming.md)

```wasp
price = 3
count = 2
total := price * count               // derived
whenever total > 10 { print "big order: " total }
on change total { print "total is now " value }
count = 5                            // prints both

raise stop the machine{reason:"human nearby"}
on stop the machine { print event.reason }

price, count = 4, 1                  // one notification for both writes (P112)
```

Educate rather than refuse: `whenever x { … }` with a non-boolean `x` is "did you mean `on change x`?";
`on change f` of a function is "did you mean `after f`?".

## Semantics

- **Push-pull**: writes push (the checks run after the write statement), values are pulled (a derived value is
  computed when a listener reads it). Listeners run synchronously, on the writing thread, in declaration order.
- **Dependency tracking at compile time**: the names a condition reads, closed over `:=` definitions. A module is
  closed (no eval of unknown code in a compiled module), so static tracking is complete for main-level code.
  Runtime tracking is needed only for: writes inside functions (phase 3, or static too: every write site of a watched
  global gets the check), signals passed as values (phase 5), dynamic dependencies through function calls
  (`whenever f() > 3` where f reads globals: phase 3 walks f's body statically via effects.rs' call graph).
- **Glitch-freedom**: no cached intermediates, so no stale derived value is ever seen. Remaining glitch: two writes
  in two statements show the intermediate state (`a=1; b=2` with `whenever a+b==2`). Fix: a multi-assignment
  `a, b = 1, 2` checks once after both (phase 4). There is no batching block (P112: the user chose this over
  `together { … }`).
- **Listener scope**: a listener sees writes in the statements after it, loops included, not before it. Writes through
  `global x` inside called functions run the listeners too (P111, revises P38; phase 3).
- **Recursion guard**: a listener that writes the variable it watches would loop; the check inside the listener body
  is not inserted (today: bodies are lowered with no listeners, so it already holds).
- **Interaction with `:=`**: assigning a derived value is an error (P71). `on set total` of a derived value is
  `on change total` (it has no write of its own), done in phase 2.
- **Interaction with `after` / `go` / tasks**: tasks are isolates (P33); only `shared` values (P106) cross. A listener
  on a shared value cannot be a check after local writes (the writes happen in another instance): it becomes a poll
  at the task check points (task_poll, `await`, loop back-edges), which is what `after done return x` does inside its
  own task today. Events raised in a task reach the `on` handlers of the starting thread at its next check point
  (TaskHandlers hook in declarations.rs; warp-d9 offered the task side, card task-signals).
- **Interaction with effects.rs**: a write of a watched variable performs the listener bodies, so their effects
  belong to the writer (`f ! Pure` that writes x with an IO listener on x is a violation). Phase 3.

## Lowering

- **Static (phases 1-4)**: no runtime objects. Checks are inlined after writes (today) or, once there are many write
  sites, a generated function `x·changed()` per watched variable that runs its listeners, called after each write.
  `once` gets a flag global, `on change` a `change_last_N` variable with the last seen value.
- **Events (phase 2)**: `raise e{…}` with handlers known at compile time is a direct call of a generated `on·e(event)`
  that runs the handlers in order. A raise with no handler stays today's error (exception), caught by `try`.
  Done: lowering/event_signals.rs. Handlers are program-wide (a raise before the `on` line reaches it too, as a
  function defined later is callable); the function declares `global` the main-level variables the bodies mention.
- **Escaping signals (phase 5, done: lowering/signal_values.rs)**: a $Signal is a cell (wasm_emitter/cells.rs: a
  `$Node` of Kind::Data over a `$node_array`) with a second slot, the list of listener closures: `signal_new(v)`,
  `cell_get` / `cell_set` its value, `signal_listeners(s)` / `signal_listeners_set(s, list)`. A write is the generated
  `signal·set(s, v)`: store, then `closure_call_2(listener, new, old)` for each listener. Only variables that escape
  get it; the rest stay plain locals/globals.
- **Tasks (phase 6)**: an event crossing threads is a TaskValue (already a node); host words `signal_send(name,
  value)` / handled in task_poll.
- **Browser (phase 7)**: the playground host.js maps `raise` of an unknown-to-the-program event to a DOM
  `CustomEvent` on the page (and `broadcast` to `postMessage`), and page events (`on click`, `on input`) to exported
  handlers `on·click`: the page calls them through the worker message loop. Output bindings (an element showing a
  derived value) are listeners the playground adds: `on change total` → update the element.

## Phases (each its own board card, smallest useful first)

1. **Derived signals and change detection** (done, 812144532): a listener on a `:=` value watches what its
   definition reads, transitively; `on change x {…}` runs only when the value differs, `value` is the new value.
   tests/control/test_variable_signals.rs.
2. **Named event signals in one module** (done, branch signals-events): `raise name{data}` → `on name {…}` (payload
   `event`), static dispatch; `on set` of a derived value means `on change`. tests/control/test_event_signals.rs.
3. **Program-wide listeners** (done): writes to a watched main-level variable inside
   functions (via `global x`) run the checks: the listener's check is the function `signal·check·N()`, guarded by
   `signal_listening_N` (set where the listener is declared), called after each such write in a function body.
   Done too (card signals-phase-rest): `whenever f() > 3` follows the variables f reads, transitively through the
   functions it calls (function_reads); effects need nothing new: effects.rs runs on the lowered program, where the
   check is a call inside the writing function, so `effects of f` includes its listeners' IO.
4. **Batching** (done, branch signals-events): a multi-assignment `a, b = 1, 2` checks once, after all its writes
   (P112: no batching block); tuples::destructured_names names its targets.
5. **Signals as values** (done, branch signals-values, warp-3a; tests/control/test_signal_values.rs): static part
   (branch signals-events): a field or item write `p.age = 2`, `xs#1 = 9` is a write of its variable, `on change p.age`
   / `whenever xs#1 > 5` listen to it. Dynamic part: **a listener inside a function subscribes when the function runs
   and stays after it returns**: `watch(s) := on change s {print value}; x = 1; watch(x); x = 2` prints 2.
   - What subscribes: `on set` / `on change` / `whenever` / `once` in a function body watching a parameter, or a
     main-level variable the function does not assign (`def log() { on change count {…} }`). A listener on the
     function's own locals stays the static check of phase 1.
   - What escapes: the subscribed parameters, and through every call the arguments given for them (`relay(t) {
     watch(t) }; relay(x)` makes x a signal); a main-level variable subscribed directly. A value that is no variable
     (`watch(3)`) becomes a signal nobody writes.
   - Lowering, two passes around variable_signals: `subscribe` turns the listener into `signal_listeners_set(s,
     signal_listeners(s) + [(value, signal·old) => {check; 0}])` (`on change`: `value != signal·old`; `once`: a flag
     cell `signal·fired·N`); `lower` makes the escaping variable's first main-level write `x = signal_new(e)`, every
     other write `signal·set(x, e)` (`x += e` reads the cell), every read `cell_get(x)`, and passes the cell itself to
     a signal parameter. A function's `global x` of an escaping x goes (it changes the cell, never x).
   - Order: subscriptions run inside the write, in the order they were made; the main level's own static listeners
     on the same variable run right after the write (test a_static_listener_and_a_subscription_on_one_variable).
   - Writes before the subscription are not seen; `clear listeners of x` removes all subscriptions (no single one yet).
   - Reflection (card g-3HmY, branch signals-reflect; tests/control/test_signal_reflection.rs; syntax an assumption
     queued with the Interviewer): `listeners of x` is the list of the functions listening to x (`count listeners of
     x`, `(listeners of x)#1`, `for f in listeners of x {f(new, old)}`), `clear listeners of x` unsubscribes them all.
     Either makes x a $Signal, and x's main-level listeners then subscribe at run time (in declaration order), so
     the list holds every listener. A listener is a function of (new value, old value).
     P128 (branch signals-remove, tests/control/test_listener_removal.rs): a main-level listener named where it is
     declared, `alarm = whenever t > 30 {…}`, is the function `alarm(value, old)`; `remove alarm from listeners of t`
     takes it out of t's list (its place is `alarm_index_t`, -1 once removed, the later named listeners move up; a
     second removal does nothing). Removal is a main-level statement.
   - Lists of signals (branch signals-lists, tests/control/test_signal_lists.rs): a function subscribing to the loop
     variable of `for s in xs {…}` over its parameter xs takes a list of signals; the variables of a list literal
     passed there (`watch_all([a, b])`, or `watched = [a, b]; watch_all(watched)`) become signals and the list holds
     the signals themselves (the one place a list keeps no copy).
   - Open: escaping into an object field (objects copy values, so nothing there keeps the cell yet); `on change p.age` inside a function on a parameter p (only plain names subscribe);
     reflecting over listeners (g-3HmY).
6. **Signals across tasks** (done): events raised in tasks reach `on` handlers of the starting thread (b8da08417:
   task_inside / signal_send, run at the program's next await, loop start or the end of the run;
   tests/control/test_task_signals.rs). Listeners on `shared` values (branch signals-shared, warp-3a;
   tests/control/test_shared_signals.rs): a main-level `whenever` / `once` / `on change` / `on set` watching a shared
   value becomes a check in the exported `on·shared()` (signal_values::poll_shared, before shared_arrays): it compares
   the watched values with `signal_seen_N` and runs the listener on a change, from the line that declares it on
   (`signal_polling_N`). The runtime calls it at every check point: signal_poll (main's start and end, loop starts),
   every 10 ms of a sleep, and after the run's tasks ended (native system_signals.rs, playground host.js). It never
   keeps a program alive (timers do). By polling, `on set` of a shared value is `on change` (a write of the same value
   is invisible), several writes between two check points are one change, and the program's own write is seen at its
   next check point, not right after it.
7. **Browser and outside**: DOM events and output bindings in the playground (done, branch signals-browser, warp-4a:
   `on click {…}` / `on key {…}` are PAGE_EVENTS, kept without a raise and exported as `on·click·node([event])`; a
   handler takes `event` only when it reads it. The worker keeps the run (host.js runPageEvent), the page sends clicks
   on the output (`{x, y}`, a canvas pixel on a canvas) and keys typed there (`{key}`); the handler's prints and
   paintings appear, and the last line, when it is a name, is the output binding `page·value()` shown anew after each
   handler (else the handler's value). An unhandled raise stays the exception (P110), no CustomEvent yet. Tour
   example `events`; tests/control/test_page_events.rs). Still open: `broadcast` / channel listeners
   (stdlib, wiki/signal.md). System signals (OS, devices, page): notes/system_signals.md.

## Decisions (user, 2026-10-06, via the Interviewer)

- P109 implicit: any variable can be watched; the compiler adds checks only for watched ones. No `signal` keyword.
- P110 `raise X` goes to the `on X` handlers first; with no handler it stays today's catchable exception.
- P111 writes through `global x` inside called functions run the listeners (phase 3). Revises P38: tests pinning
  "never fires" may be changed, each in one commit naming P111.
- P112 no batching block (the user chose this over `together { … }`); only a multi-assignment `a, b = 1, 2`
  notifies once.
