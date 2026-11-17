# Signals as first-class citizens

Roadmap card: signals (board topic; its sub-cards are the steps and leftovers below).
Spec: wiki/signal.md (system signals of OS, devices and page too), wiki/thread.md, wiki/charged.md.

warp has push (listeners run after writes) and pull (getters compute on read): the push-pull model of Preact/Solid
signals, done at compile time like Svelte 5's compiler, with runtime objects only where a signal escapes. An event
signal is a node (`stop the machine{time:now reason:"…"}`), so it serializes across tasks, host, page and network; a
state signal is a variable, `x = 3` stays `x = 3`.

## Decisions
- P109: any variable can be watched; the compiler adds checks only for watched ones. No `signal` keyword.
- P163 (`emit` sends events, `raise` / `throw` errors only): wiki/signal.md.
- P111: writes through `global x` inside called functions run the listeners.
- P156 (`whenever` is edge-triggered): wiki/signal.md.
- P112: no batching block; only a multi-assignment `a, b = 1, 2` notifies once.
- P148: `old` is the value before the write.

## The model
1. Every variable is observable; only observed ones cost anything. Unobserved programs compile as without signals.
2. `:=` is the derived signal: `total := a + b` recomputes at each use (P71); listeners on `total` watch what its
   definition reads, transitively. No cache, so never stale. Assigning a derived value is an error;
   `on set total` of a derived value means `on change total`.
3. Listeners: `whenever cond {…}`, `once cond {…}`,
   `on set x {…}` (each write), `on change x {…}` (each write that changed the value), `after f`/`before f` (calls).
   Inside: `value` (alias `signal`, `event`) is the new value, `old` (aliases `previous`, `was`, `before`, with a note)
   the one before; a program variable of one of these names keeps its meaning. `on set p.age` watches one field: a
   write of it or of p.
4. Events: `emit name{data}` → `on name {…}` (`event` is the payload; `emit item 1` carries 1). `once name {…}` runs at
   the first emit only. A write of x is the event `set x`.
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

Ported cases of Svelte, Vue, Solid, RxJS, C#, Node, Qt and the DOM: test/control/test_reactive_ports.warp.

## Semantics
- Writes push (checks run after the write statement), values are pulled. Listeners run synchronously, on the writing
  thread, in declaration order.
- Dependencies are tracked at compile time: the names a condition reads, closed over `:=` definitions and through
  the functions it calls (`whenever f() > 3`, function_reads). A module is closed, so static tracking is complete for
  main-level code.
- Remaining glitch: two writes in two statements show the intermediate state; a multi-assignment checks once.
- A listener sees writes in the statements after it, loops included. A listener body gets no listeners, so a
  listener writing its own variable doesn't loop.
- Effects: a write of a watched variable performs the listener bodies, so their effects belong to the writer
  (`effects of f` includes its listeners' IO).

## Lowering
- Listeners: lib/signals.warp's rules turn `on set` / `on change` / `whenever` / `once` into `variable_listen` calls
  over the prelude's event listeners (lib/prelude.warp listen, emitted); a write of a watched name notifies them
  through the program rule `signaled` (extensions/rules/rule_words.warp "variable signals"): `x = 3` as
  `(signal·old = x; x = 3; signal_notify("x", x, signal·old))` (test/control/test_variable_signals.warp).
- Events: handlers are program-wide (an emit before the `on` line reaches it too)
  (test/control/test_event_signals.warp).
- `on error of f {handler}`: f's body becomes `try body else handler` (extensions/rules/rule_words.warp
  errors_handled), so every call of f has the handler's value when it fails; forms `{…}`, `: …`, `do …`; an undefined
  f is "on error of g: the program defines no function g". `error` is not bound in the handler yet.

## Signals as values
- A listener inside a function subscribes when the function runs and stays after it returns:
  `watch(s) := on change s {print value}; x = 1; watch(x); x = 2` prints 2. What subscribes: `on set` / `on change` /
  `whenever` / `once` in a function body watching a parameter, or a main-level variable the function does not assign.
  A listener on the function's own locals stays static.
- What escapes: the subscribed parameters, and through every call the arguments given for them (`relay(t) {
  watch(t) }; relay(x)`); a main-level variable subscribed directly. `watch(3)` makes a signal nobody writes.
- Order: subscriptions run inside the write, in subscription order; the main level's static listeners on the same
  variable right after. Writes before the subscription are not seen.
- Field and item writes `p.age = 2`, `xs#1 = 9` are writes of their variable. Field signals are named by their path
  `p.age`: `watch(p) := on change p.age {…}` subscribes to that field; the object literal assigned to a variable
  passed there keeps a cell in the field, and a variable kept in it (`person = {age: a}`) escapes
  (test/control/test_signal_fields.warp, samples/signals/signal_fields.warp). Open: a field signal of an object built
  anywhere but a main-level literal.
- Lists of signals: a function subscribing to the loop variable of `for s in xs {…}` over its parameter xs takes a
  list of signals; a list literal of variables passed there holds the signals themselves (the one place a list keeps
  no copy): `watch_all([a, b])` passes `[Signal("a", a), Signal("b", b)]` (lib/signals.warp), the loop's listener
  listens to the item's variable, and a bare read of the item is the variable's value as last written
  (extensions/rules/rule_words.warp signal_list_takers; test/control/test_signal_lists.pairs.warp).

## Listener lists
- Variables: `listeners of x` is the list of functions listening to x (`count listeners of x`, `(listeners of x)#1`,
  `for f in listeners of x {f(new, old)}`), `clear listeners of x` unsubscribes them all; either makes x a $Signal
  whose main-level listeners subscribe at run time in declaration order (test/control/test_signal_reflection.warp). A
  named listener `alarm = whenever t > 30 {…}` is the function `alarm(value, old)`; `remove alarm from listeners of
  t` takes it out, anywhere, also inside functions
  (test/control/test_listener_removal*.warp).
- Events: a named handler `h = on alarm {…}` listens until `remove h from listeners of
  alarm` (or `listeners of alarm -= h`, also from its own body), `clear listeners of alarm`, `count listeners of
  alarm`. `listeners of alarm` lists the handlers still listening by their place in declaration order (`[1, 3]` once
  the second is removed); any handler is removed by its place (`remove 2 from listeners of alarm`). `remove h from
  alarm` is short for it unless alarm is also a variable. Removing an h that is neither a handler nor a variable is
  the compile error "h is no named listener of tick"; `listeners of x` for a name nothing listens to is loud
  (test/control/test_event_handler_lists.warp).
- A handler inside a block subscribes each time the block runs (`for i in 1 to 3 { on tick {…} }` adds three), run by
  one main-level handler. Open: those are not listed.

## Across tasks
- Events emitted in tasks reach the `on` handlers of the starting thread at its next await, loop start or the end of
  the run (test/control/test_task_signals.warp).
- A main-level listener watching a `shared` value becomes a check that compares the watched values with the ones
  seen last and runs the listener on a change, from its declaring line on. The runtime calls it at every check point: signal_poll (main's start and end, loop
  starts), every 10 ms of a sleep, and after the run's tasks ended. It never keeps a program alive. By polling,
  `on set` of a shared value is `on change`, several writes between two check points are one change
  (test/control/test_shared_signals.warp).

## Browser
- `on click {…}` / `on key {…}` are page events; a handler takes `event` only
  when it reads it. The worker keeps the run, the page sends clicks on the output (`{x, y}`, a
  canvas pixel on a canvas) and keys (`{key}`); the handler's prints and paintings appear, and the last line, when
  it is a name, is the output binding shown anew after each handler (test/control/test_page_events.warp,
  tour example `events`).
- An emit no handler receives, in a program compiled for a page, is `dispatch_event(name, data)` (lib/signals.warp): a
  `CustomEvent(name, {detail})` on the document (markup.js dispatchToPage; a Worker posts it to its page)
  (test/web/test_page.pairs.warp, samples/web/page_custom_event.warp). A handler body inside an element
  stays code.
- `broadcast` / channel listeners: natively (wiki/signal.md); the stdlib side is open.
