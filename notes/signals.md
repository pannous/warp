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

## What a signal is in warp (proposal)

1. **Every variable is observable; only observed ones cost anything.** No `signal` keyword: a variable becomes a
   signal when something listens to it, and the compiler inserts the notification at its writes. Unobserved programs
   compile exactly as today. (Alternative: an explicit `signal x = 0` marker, as Svelte's `$state`. Question Q1.)
2. **`:=` is the derived signal.** `total := a + b` already recomputes at each use (P71); listeners on `total` watch
   `a` and `b` (phase 1, done). Glitch-free by construction: a derived value has no cache, so it can never be stale.
   A cache with a dirty flag (memo) is an optimization the compiler may add later when the body is pure (effects.rs).
3. **Listeners are the effects**: `whenever cond {…}` (each time it holds after a change), `once cond {…}`,
   `on set x {…}` (each write), `on change x {…}` (each write that changed the value, phase 1, done), `after f`/`before f`.
   Inside: `value` (alias `signal`, `event`) is the new value.
4. **Events**: `raise name{data}` sends the event, `on name {…}` receives it (`event` is the payload node). A write
   of x is the event `set x`, so `on set x` is one case of the general rule.
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

together { price = 4; count = 1 }    // one notification for both writes (phase 4, name: Q4)
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
  `a, b = 1, 2` checks once after both, and `together { … }` defers all checks to its end, each listener once (phase 4).
- **Listener scope** (P38, decided keep): a listener sees writes in the statements after it, loops included, not
  before it and not inside called functions. Phase 3 asks whether writes to a watched variable inside functions count.
- **Recursion guard**: a listener that writes the variable it watches would loop; the check inside the listener body
  is not inserted (today: bodies are lowered with no listeners, so it already holds).
- **Interaction with `:=`**: assigning a derived value is an error (P71). `on set total` of a derived value is
  `on change total` (it has no write of its own); today it fires never: phase 2 makes it the change listener with a
  got-it note.
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
- **Escaping signals (phase 5)**: WASM GC `(struct $Signal (field $value (mut anyref)) (field $listeners (mut (ref null
  $ClosureList))))`; a write is `signal_set` (store, then `call_ref` each listener closure); a read is `struct.get`.
  Only variables that escape get it; the rest stay plain locals/globals.
- **Tasks (phase 6)**: an event crossing threads is a TaskValue (already a node); host words `signal_send(name,
  value)` / handled in task_poll.
- **Browser (phase 7)**: the playground host.js maps `raise` of an unknown-to-the-program event to a DOM
  `CustomEvent` on the page (and `broadcast` to `postMessage`), and page events (`on click`, `on input`) to exported
  handlers `on·click`: the page calls them through the worker message loop. Output bindings (an element showing a
  derived value) are listeners the playground adds: `on change total` → update the element.

## Phases (each its own board card, smallest useful first)

1. **Derived signals and change detection** (done on branch signals): a listener on a `:=` value watches what its
   definition reads, transitively; `on change x {…}` runs only when the value differs, `value` is the new value.
   tests/control/test_variable_signals.rs.
2. **Named event signals in one module**: `raise name{data}` → `on name {…}` (payload `event`), static dispatch;
   `on set` of a derived value means `on change`.
3. **Program-wide listeners**: writes to a watched main-level variable inside functions (via `global x`) run the
   checks; `whenever f() > 3` follows the variables f reads; effects.rs counts listener bodies at the writes.
4. **Batching**: multi-assignment checks once; `together { … }` block defers the checks.
5. **Signals as values**: `$Signal` cells for escaping variables and object fields (`on change person.age`),
   subscription inside functions.
6. **Signals across tasks**: events raised in tasks reach `on` handlers of the starting thread; listeners on `shared`
   values poll at check points (with warp-d9, card task-signals).
7. **Browser and outside**: DOM events and output bindings in the playground, `broadcast` / channel listeners
   (stdlib, wiki/signal.md).

## Open questions (sent to the Interviewer warp-eb, defaults assumed)

- Q1 implicit (every variable observable, recommended) or an explicit `signal x = 0` declaration.
- Q2 `raise X` with an `on X` handler in the program: a signal to the handlers (recommended), else an exception as
  today; or a separate word (`send X` / `emit X`) for signals and `raise` stays the exception.
- Q3 revisit P38: should writes inside functions trigger main-level listeners (recommended, phase 3).
- Q4 the batching block's name: `together { … }` (recommended), `batch { … }`, `atomically { … }`.
