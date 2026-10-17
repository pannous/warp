# Signals as first-class citizens

Roadmap card: signals (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).
Spec: wiki/signal.md, wiki/thread.md, wiki/charged.md. System signals (OS, devices, page): notes/system_signals.md.

warp has push (listeners run after writes) and pull (getters compute on read): the push-pull model of Preact/Solid
signals, done at compile time like Svelte 5's compiler, with runtime objects only where a signal escapes. An event
signal is a node (`stop the machine{time:now reason:"…"}`), so it serializes across tasks, host, page and network; a
state signal is a variable, `x = 3` stays `x = 3`.

## Decisions
- P109: any variable can be watched; the compiler adds checks only for watched ones. No `signal` keyword.
- P163: `emit X` (and `send X` without `to`) sends events, `raise` / `throw` are errors only; an unhandled emit does
  nothing (P202 in a plain program). fire, trigger, signal and ask are aliases of emit with a note, also as an
  operand: `net * (1 + ask tax_rate)`.
- P111: writes through `global x` inside called functions run the listeners.
- P156: `whenever` is edge-triggered: it runs each time its condition becomes true, not at every write while it
  holds (`on change x { if … }`, `on set x { if … }` and a timer give the other behaviours).
- P112: no batching block; only a multi-assignment `a, b = 1, 2` notifies once.
- P148: `old` is the value before the write.

## The model
1. Every variable is observable; only observed ones cost anything. Unobserved programs compile as without signals.
2. `:=` is the derived signal: `total := a + b` recomputes at each use (P71); listeners on `total` watch what its
   definition reads, transitively. No cache, so never stale. Assigning a derived value is an error;
   `on set total` of a derived value means `on change total`.
3. Listeners: `whenever cond {…}` (flags `whenever_held_N` / `whenever_was_N`, `edge_check`), `once cond {…}`,
   `on set x {…}` (each write), `on change x {…}` (each write that changed the value), `after f`/`before f` (calls).
   Inside: `value` (alias `signal`, `event`) is the new value, `old` (aliases `previous`, `was`, `before`, with a note)
   the one before; a program variable of one of these names keeps its meaning. `on set p.age` watches one field: a
   write of it or of p.
4. Events: `emit name{data}` → `on name {…}` (`event` is the payload; `emit item 1` carries 1). `once name {…}` runs at
   the first emit only (flag `once_fired_N`). A write of x is the event `set x`.
5. Type: a variable keeps its value type; only a signal that escapes as a value has a runtime representation.

```warp
price = 3
count = 2
total := price * count               // derived
whenever total > 10 { print "big order: " total }
on change total { print "total is now " value }
count = 5                            // prints both
emit stop the machine{reason:"human nearby"}
on stop the machine { print event.reason }
price, count = 4, 1                  // one notification for both writes
```

Educate rather than refuse: `whenever x { … }` with a non-boolean `x` is "did you mean `on change x`?";
`on change f` of a function is "did you mean `after f`?".

Ported cases of Svelte, Vue, Solid, RxJS, C#, Node, Qt and the DOM: probes/reactive_ports.md,
tests/control/test_reactive_ports.rs.

## Semantics
- Writes push (checks run after the write statement), values are pulled. Listeners run synchronously, on the writing
  thread, in declaration order.
- Dependencies are tracked at compile time: the names a condition reads, closed over `:=` definitions and through
  the functions it calls (`whenever f() > 3`, function_reads). A module is closed, so static tracking is complete for
  main-level code.
- Remaining glitch: two writes in two statements show the intermediate state; a multi-assignment checks once.
- A listener sees writes in the statements after it, loops included. A listener body gets no listeners, so a
  listener writing its own variable doesn't loop.
- Effects: a write of a watched variable performs the listener bodies, so their effects belong to the writer;
  effects.rs runs on the lowered program, where a cell write is State and a closure call carries every lifted
  lambda's effects (`effects of f` includes its listeners' IO).

## Lowering
- Static (lowering/signals/variable_signals.rs): checks inlined after writes; `once` a flag global, `on change` a
  `change_last_N` variable. A variable only the main level writes keeps this static check
  (tests/control/test_variable_signals.rs).
- Events (event_signals.rs): `emit e{…}` with known handlers calls a generated `on·e(event)` running them in order.
  Handlers are program-wide (an emit before the `on` line reaches it too); the function declares `global` the
  main-level variables the bodies mention (tests/control/test_event_signals.rs).
- `on error of f {handler}`: f's body becomes `try body else handler` (with_function_error_handlers), so every call
  of f has the handler's value when it fails; forms `{…}`, `: …`, `do …`; an undefined f is "on error of g: the
  program defines no function g". `error` is not bound in the handler yet.
- Watched cells (decision watched-cells): a main-level variable written inside functions is a cell and its
  main-level listeners subscribe to it (signal_values::subscribe).
- Escaping signals (lowering/signals/signal_values.rs): a $Signal is a cell (wasm_emitter/cells.rs) with a second
  slot, the list of listener closures: `signal_new(v)`, `cell_get` / `cell_set`, `signal_listeners(s)` /
  `signal_listeners_set(s, list)`. A write is `signal·set(s, v)`: store, then `closure_call_2(listener, new, old)`
  for each listener. Only variables that escape get it (tests/control/test_signal_values.rs).

## Signals as values
- A listener inside a function subscribes when the function runs and stays after it returns:
  `watch(s) := on change s {print value}; x = 1; watch(x); x = 2` prints 2. What subscribes: `on set` / `on change` /
  `whenever` / `once` in a function body watching a parameter, or a main-level variable the function does not assign.
  A listener on the function's own locals stays static.
- What escapes: the subscribed parameters, and through every call the arguments given for them (`relay(t) {
  watch(t) }; relay(x)`); a main-level variable subscribed directly. `watch(3)` makes a signal nobody writes.
- Two passes around variable_signals: `subscribe` turns the listener into `signal_listeners_set(s,
  signal_listeners(s) + [(value, signal·old) => {check; 0}])` (`once`: a flag cell `signal·fired·N`); `lower` makes
  the escaping variable's first main-level write `x = signal_new(e)`, every other write `signal·set(x, e)`, every
  read `cell_get(x)`, and passes the cell itself to a signal parameter.
- Order: subscriptions run inside the write, in subscription order; the main level's static listeners on the same
  variable right after. Writes before the subscription are not seen.
- Field and item writes `p.age = 2`, `xs#1 = 9` are writes of their variable. Field signals are named by their path
  `p.age`: `watch(p) := on change p.age {…}` subscribes to that field; the object literal assigned to a variable
  passed there keeps a cell in the field, and a variable kept in it (`person = {age: a}`) escapes
  (tests/control/test_signal_fields.rs, samples/signals/signal_fields.warp). Open: a field signal of an object built
  anywhere but a main-level literal.
- Lists of signals: a function subscribing to the loop variable of `for s in xs {…}` over its parameter xs takes a
  list of signals; a list literal of variables passed there holds the signals themselves (the one place a list keeps
  no copy; tests/control/test_signal_lists.rs).

## Listener lists
- Variables: `listeners of x` is the list of functions listening to x (`count listeners of x`, `(listeners of x)#1`,
  `for f in listeners of x {f(new, old)}`), `clear listeners of x` unsubscribes them all; either makes x a $Signal
  whose main-level listeners subscribe at run time in declaration order (tests/control/test_signal_reflection.rs). A
  named listener `alarm = whenever t > 30 {…}` is the function `alarm(value, old)`; `remove alarm from listeners of
  t` takes it out (place `alarm_index_t`, -1 once removed), anywhere, also inside functions
  (tests/control/test_listener_removal*.rs).
- Events: a named handler `h = on alarm {…}` runs while its flag `h_listening` holds: `remove h from listeners of
  alarm` (or `listeners of alarm -= h`, also from its own body), `clear listeners of alarm`, `count listeners of
  alarm`. `listeners of alarm` lists the handlers still listening by their place in declaration order (`[1, 3]` once
  the second is removed); any handler is removed by its place (`remove 2 from listeners of alarm`). `remove h from
  alarm` is short for it unless alarm is also a variable. Removing an h that is neither a handler nor a variable is
  the compile error "h is no named listener of tick"; `listeners of x` for a name nothing listens to is loud
  (tests/control/test_event_handler_lists.rs).
- A handler inside a block subscribes each time the block runs (`for i in 1 to 3 { on tick {…} }` adds three), run by
  one main-level handler (subscribed_in_blocks). Open: those are not listed.

## Across tasks
- Events emitted in tasks reach the `on` handlers of the starting thread at its next await, loop start or the end of
  the run (task_inside / signal_send; tests/control/test_task_signals.rs).
- A main-level listener watching a `shared` value becomes a check in the exported `on·shared()`
  (signal_values::poll_shared): it compares the watched values with `signal_seen_N` and runs the listener on a change,
  from its declaring line on. The runtime calls it at every check point: signal_poll (main's start and end, loop
  starts), every 10 ms of a sleep, and after the run's tasks ended. It never keeps a program alive. By polling,
  `on set` of a shared value is `on change`, several writes between two check points are one change
  (tests/control/test_shared_signals.rs).

## Browser
- `on click {…}` / `on key {…}` are PAGE_EVENTS, exported as `on·click·node([event])`; a handler takes `event` only
  when it reads it. The worker keeps the run (host.js runPageEvent), the page sends clicks on the output (`{x, y}`, a
  canvas pixel on a canvas) and keys (`{key}`); the handler's prints and paintings appear, and the last line, when
  it is a name, is the output binding `page·value()` shown anew after each handler (tests/control/test_page_events.rs,
  tour example `events`).
- An emit no handler receives, in a program compiled for a page, is `dispatch_event(name, data)`: a
  `CustomEvent(name, {detail})` on the document (markup.js dispatchToPage; a Worker posts it to its page); natively
  the headless Page keeps it (`Page::dispatched`; tests/web/test_page_custom_events.rs,
  samples/web/page_custom_event.warp). A handler body inside an element stays code (markup_tags::handlers_kept).
- `broadcast` / channel listeners: natively (notes/system_signals.md, Channels); the stdlib side is open.
