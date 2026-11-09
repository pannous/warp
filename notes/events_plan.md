# Events: one loop for timers, signals, input, sockets, windows and the page

Roadmap card: event-loop (board topic; its sub-cards are the steps of section 6, notes/roadmaps.md).
Companions: notes/signals.md (signals inside a program), notes/system_signals.md (the Rust era's system signals: the
record of what was decided, its paths are gone), notes/ffi_handles.md (callbacks), notes/roadmap_native_harness.md.
User answers 2026-11-09 (through warp-interviewer, section 7): cancel-timer A (`cancel t`), ctrl-c-runs-exit A (`on
exit` runs, then 130 / 143), event-delivery-points (Q1): waits and loop starts, one rule for native and the page,
and threaded-handlers: real threads planned too (section 7, "Decided").

## In one paragraph
A run has one event loop. Sources (timers, OS signals, readable descriptors, the window, file watches, callbacks from
C threads, finished tasks and fetches) register with it; it waits in exactly one place and runs handlers one at a
time, never inside running code (run-to-completion). Its policy is warp, written once in lib/events.warp: a **turn**
takes what happened and the time, runs the due handlers in a fixed order and says how long to wait. Natively one
blocking host word, `events_wait`, feeds it (poll() in the native host module, a library: no driver change); in the
page the browser's own loop calls the turn. C keeps only what warp must not do: the signal handler (async-signal-safe)
and callbacks arriving on foreign threads (never allowed into wasm); both only write a byte to a wake pipe.

## 1. Today

### Native
How it works:
- Named events: lib/prelude.warp listen / emitted / unlisten, a synchronous registry; `emit` calls the listeners in
  order inside the emit and gives the last one's value (P163). extensions/sugar/events.warp makes on, once, when,
  emit (fire, send, trigger), listeners of calls of it. Variable listeners (lib/signals.warp) use the same registry.
  Both stay as they are.
- Timers: lib/timers.warp keeps each handler and tells the host by `signal_listen(kind, id, argument)`; the driver
  keeps a second table (core/native/driver.warp TIMERS_SLOT, next_due, weekday_now) and, once main returned, sleeps
  (nanosleep) until the earliest is due and calls the export timer_fired(id) by name (events_run). WARP_EVENTS_FOR
  bounds it for tests and sweeps.
- Frames: a program with `on frame` ends in lib/frames.warp frames_end, `while window_open() { emitted(frame) }`,
  inside main; `on exit` ends main with exits_end (extensions/rules/rule_words.warp framed, exited).
- Keys: lib/keys.warp compares counters ("keys pressed") each frame; lib/host/native_host/window.warp counts SDL's
  presses.
- `warp app`: lib/markup.warp page_window_run loops on the web view's `next(ms)`, and lib/timers.warp runs the timers
  a second time itself on each "tick" (timers_ticked), since that loop never returns to the driver's.
- Tasks: lib/tasks.warp, a cooperative scheduler with its own queue of due times (tasks_run_while, tasks_end).
- Server: lib/web.warp serve, a blocking accept() loop (routes_end).
- Channels: lib/channels.warp, a 20 ms timer reading the channel's file in /tmp (driver channel_* words).
- C callbacks: lib/callbacks.warp and 16 trampolines in lib/native/foreign.c call the driver's c_callback_answered,
  which calls the export c_callback_fired, synchronously inside the C call (qsort works).

What is broken (probed 2026-11-09 with the installed warp, WARP_NO_WINDOW=1, WARP_EVENTS_FOR=350):
1. Five loops, none composes: driver events_run, frames_end, page_window_run, tasks_run_while, serve's accept.
   Whoever waits blocks the rest: no timer fires while a window is open or routes are served, a task's sleep fires
   no timer.
2. Timers never run during main: `n = 0; on every 100 ms {…}; sleep 350 ms` saw 0 ticks in the sleep, then the three
   missed ticks fired back to back after main (the driver's next due is due + period: no "one call however late").
3. `on exit` runs before the timers: `on every 100 ms { print "tick" }; on exit { print "exit handler" }` prints
   "exit handler", then the ticks.
4. `on interrupt {…}` is an ordinary named event nobody emits: the program ends at once, ctrl-c always kills. No
   terminate, hangup or resize either.
5. The schedule of a timer is computed three times (driver next_due, timers.warp timer_due, web/playground
   host-timers.js millisecondsUntil), and tasks keep a fourth queue of due times.
6. Periods run on the wall clock (CLOCK_REALTIME): a clock step or a machine waking from sleep shifts or bursts them.
7. `on` is three modules' word: `on file "notes.txt" change {…}` gives "on(key, body) is called with 3 arguments"
   (lib/keys.warp's on takes it), `broadcast {text: "hi"} on "x"` gives "on is defined by keys and timers and
   tasks" (test_broadcast:4), `use timers; every(200 ms, f)` gives "the call all(…)" (card timers-every-refusal),
   `on alarm {…}` links the keys enum (card on-alarm-keys).
8. Polling where the system could tell: channels every 20 ms through a file, listened system values every 250 ms,
   key presses once a frame.
9. C callbacks work only on the calling thread inside the call: a library calling back from its own thread
   (CoreAudio, CoreMIDI, GCD, SDL audio) would enter wasmtime from a foreign thread, undefined behaviour. 16 slots,
   integers only, never given back.
10. A program ending in a registration prints its number: `…; on every 100 ms {…}` prints 1.
11. `on file … change` (Rust: a modification-time stamp) is not ported.

### Page (web/playground)
How it works: the worker runs main synchronously; after it host-timers.js starts one setInterval or setTimeout per
timer and calls the export timer_fired(id); a click comes by postMessage to driver.js pageEventHandled and the export
page_event(n, detail), then the page renders again; held keys and the pointer are SharedArrayBuffer slots
system_value reads (canvas.js, host.js); a fetch answered later is one more timer calling fetch_answered; channels
are BroadcastChannels (host-tasks.js); an emit nobody handles becomes a CustomEvent on the document.
Broken: `sleep` busy-waits (host.js holder.sleep), spinning a core and handling no click meanwhile; a JS promise
cannot be awaited (card promise-await-page); the timer schedule is JS's own copy; `on interrupt` passes silently.

## 2. The Rust version
Sources: tag saved/archive/rust-history, crates/warp-runtime/src/system_signals.rs, src/lowering/signals/*.rs,
main's history before 2026-10-29. The C++ wasp had only an optional requestAnimationFrame import (frames pulled).

How it worked: the compiler made each system handler an export named by convention (on·every·N, on·file·N,
on·interrupt, on·exit, on·shared) and put `signal_poll()` at main's start and end and at every loop start. The host
kept thread-local timers (Repeat Every / Clock, Trigger Always / FileChange / Ready / WindowShows), ran the due ones
at those check points and inside `sleep` (woken in 10 ms slices), and after main stayed while one lived, saying
"listening: every 5 seconds (ctrl-c to stop)". SIGINT (Windows: SetConsoleCtrlHandler) only set an atomic flag; a
second ctrl-c within a second, or before the handler ran, exited 130. exit(code) was an error unwinding the run;
`on exit` ran after main and the timers or at exit(code), never after a failure, `event` the code.

Keep:
- one call however late: the next due is (due + period) or now, whichever is later;
- a handler is never re-entered (RUNNING_HANDLERS, fc3781f409, card timer-handler);
- clock times recomputed from the local clock after each firing (summer time);
- the flag-only signal handler and the double ctrl-c rule, so a handler never makes a program unstoppable;
- `on exit` last, after the timers, and at exit(code), never after a failure;
- the "listening: …" line once on stderr while a program stays, frames silent;
- a file changed = its modification time or size changed (coarse clocks);
- a source a platform lacks is loud.

Drop:
- handlers found by export name: the compiler and the host had to agree on each new kind of event;
- check points compiled into every loop: a cost in hot loops, handlers running in the middle of an update, and the
  re-entry bug;
- polling as the mechanism: ctrl-c in 10 ms slices, files every 100 ms, shared values every 10 ms, awaits every 10 ms;
- the timer semantics in the host (Repeat::Clock in Rust beside the JS copy).

## 3. The model
Theory, briefly. A reactor (libuv, the browser, Node) waits in one place for readiness (kqueue, epoll, poll), keeps
timers by due time and runs callbacks to completion; the browser adds a microtask checkpoint after each task and a
rendering step per frame. Fine-grained signals (Solid, Preact, the TC39 proposal) push invalidation and pull values:
warp's variables and `:=` already do (notes/signals.md). FRP tells behaviours (values over time: a variable, mouse_x,
battery) from events (occurrences: a click, a tick): a behaviour is read (pull), an event is delivered (push), and a
behaviour becomes a source only while something listens to its change. Structured concurrency: a registration
belongs to a scope and ends with it. WASI 0.3 (released 2026-06-11, wasmtime 46 on) is completion-based with one host
loop for all components; warp emits core modules on WASI preview 1, so that is a later target (section 7).

### Words
- **source**: what produces occurrences: Timer (every, at a clock time, once), Frame (each frame while a window
  shows), Signal (interrupt, terminate, hangup, resize), Readable (stdin, a socket, a pipe), FileWatch, window input
  (key, click), Message (a machine-wide channel), Completion (a task done, a fetch answered, a child exited),
  Callback (a C callback from another thread). An in-program `emit` is a call, not a source.
- **registration**: source, handler, once; it gives a handle, which cancels it (Q2).
- **turn**: one pass of the loop.
- **wait point**: where a turn may run while the program's code is on the stack.

### The turn
`events_turn(happened: list, now: int): int` (lib/events.warp) runs, in this order, and gives the milliseconds to wait
(0: turn again, -1: nothing keeps the run going):
1. signals, interrupt first: the user's intent beats queued work;
2. timers due by now, by due time, then registration order, each at most once a turn (`every` keeps its beat, or
   goes on from now once more than a period behind);
3. the rest of what happened (readable, messages, window input, file changes, callbacks, completions) in arrival
   order, at most a few per source a turn (fairness);
4. the frame: frame listeners while a window shows and a frame is due, then a markup program's render (the browser's
   rendering step);
5. keep-alive: the earliest due time, 0 when a source has more, -1 when no source keeps the run.

Natively the loop is plain warp around one blocking word, `wait = events_turn(events_wait(deadline, watched), now)`
until wait is -1 or the deadline passed.

### Delivery and re-entrancy (Q1 decided: event-delivery-points, section 7)
- Occurrences from outside are handled at wait points: after main's last line (a listening program ends in
  `events_end`, one ending in place of exits_end, frames_end, tasks_end and routes_end), inside `sleep`, `await` and a
  channel's `receive` (each turns the loop until its deadline or its completion), and after each painted frame
  (paint is a frame's wait). The user rejected wait points alone: a busy loop must see its timers too. Recommended
  for that (section 7, D): natively, wasmtime's epoch interruption turns the loop at wasmtime's own checks (each loop
  header and function entry) when a source is due, on main's thread; the page keeps the wait points.
- A handler runs to its end before another starts. Its own wait points turn the loop nested, but a source whose
  handler runs is held: never re-entered, its repeats coalesce (Rust's timer-handler rule, for every source).
- `emit` inside a program stays a synchronous call (P163: `1 + emit ask` uses the handler's value), as the DOM's
  dispatchEvent; only occurrences from outside are queued.
- An error raised in a handler ends the run with it, as in main; a `try` in the handler keeps it.

### Order
Each source in arrival order; timers by due time, ties by registration; the listeners of one occurrence in
registration order (today's registry); between sources the order of the turn's phases. A given sequence of happenings
gives one result, so the page and the native warp agree.

### Cancellation and lifetime
- Every registration gives its handle: `remove h from listeners of e` for named events (P128), `cancel h` for timers
  and the other sources (Q2, decided), `clear listeners of e`. `exit(code)` ends the loop, then `on exit` runs (P134).
- A registration made inside a task belongs to it and ends with it (structured; with tasks-parallel, each thread's
  own loop).
- Keep-alive (P120): the run stays while a repeating or pending timer, an open window, a listened descriptor, socket
  or channel, a file watch, a pending task or fetch, or a deferred C callback keeps it. `on interrupt`, `on exit`,
  named events, `on key` without a window and variable listeners never do. Staying says so once on stderr:
  `listening: every 5 seconds, message from "chat" (ctrl-c to stop)`, a text warp computes, which the page's chips
  show too.

### Back-pressure
- What is state coalesces: a timer due several times is one call; file changes, pointer motion and window size
  between two turns are one (the latest).
- Data is read lazily: one message or line per source a turn, the rest stays in the kernel's buffer, so a fast
  sender slows down at its socket or pipe (the readiness model gives this for free).
- The queues warp owns are bounded and loud: window key presses (64) and deferred C callbacks (1024) beyond that are
  dropped with one warning naming the source and the count, never silently.
- In-program channels keep their send / receive, now waiting through turns.

### Time
Periods (every, sleep, within, frames) on the monotonic clock; clock times (at 9:00, every day at) on the local wall
clock, recomputed after each firing: summer time and a machine waking from sleep give one call, no burst.

### Surface
Nothing new except `cancel h` (Q2, decided). on, once, when, whenever, emit and its aliases, on every …, at 9:00, on every day
at, on frame, on key, on click, on exit, on interrupt, on file … change, on message from, listeners of, remove … from
listeners of, sleep, await: as decided.
`on X {…}` reads one way: X a value is a source, and its type's `listened(X, handler)` runs (one-word dispatch,
notes/dispatch.md: Key, Timer, Signal, FileWatch, Channel, each module adding the method for its own type, so `on` is
no longer three modules' word); X a word the program binds to nothing is a named event, as `click $who` already
decides by `is unbound`. The signal names (interrupt, terminate, hangup, resize) are an enum in lib/events.warp
(vocabulary is library; an FYI to the Interviewer).

## 4. The native harness
```
program (GC wasm in wasmtime): lib/events.warp, the turn and the loop
  │ events_wait(deadline, watched)      → happened: descriptors, signal numbers, window input, "callbacks"
  ▼
driver (primitive warp, C by wasm2c): a word it lacks goes on (host_word_answered → host_export), unchanged
  ▼
native host module (lib, GC wasm + FFI): host_events_wait
  poll() on the watched descriptors + the wake pipe's read end, timeout the deadline;
  while a window shows: SDL_WaitEventTimeout or the web view's run loop instead
  ▲ wake pipe
foreign.c (C, the native binary): the signal handler writes the signal's number;
  a trampoline called on a foreign thread queues its arguments in the ring and writes "callbacks"
```
- One host word waits: `events_wait(deadline_ms, watched: list): list`, answered by a new part of the native host
  module (lib/host/native_host/events.warp) through the driver's existing forwarding of words it lacks
  (driver.warp host_word_answered → host_<word>): poll() by FFI over the watched descriptors and the wake pipe's read
  end, its timeout the deadline. No driver change and no export the driver must know: the program pulls, the host
  never calls the program for an event. The driver's call of timer_fired by name goes.
- A window open: the native host module owns it (window.warp), so the same word waits in SDL_WaitEventTimeout or the
  web view's run loop instead and returns the window's input as happenings (presses, clicks, close), not counters
  read each frame. Until gui-wake, the descriptors are polled at least once a frame (16 ms) while a window shows.
- C only where warp cannot run, in lib/native/foreign.c (the native binary's C, so a core OK first):
  1. the signal handler, async-signal-safe: write() one byte to the wake pipe; a second ctrl-c within a second or
     before the first was taken: _exit(130). About 20 lines.
  2. trampolines that look at the thread: on the program's thread they call in at once as today (the C call gets its
     result: qsort, sqlite); on any other thread they copy their six integers into a bounded ring (a mutex), write
     the wake byte and give C 0. The turn drains the ring and runs the handlers on the program's thread. About 30
     lines.
  3. later (gui-wake), a waker thread that on a wake byte calls a C function the window registered (SDL_PushEvent,
     the web view's run loop), so a signal or a socket wakes a window's wait at once.
  Everything else is warp: the loop, the timers, the order, keep-alive, the listening line.
- Why not the alternatives:
  - kqueue's EVFILT_SIGNAL needs no C handler on macOS, but it cannot stop a program busy outside a wait point (no
    double ctrl-c), and Linux's signalfd needs the signal blocked, with the same flaw;
  - a host loop calling exports by name is the Rust coupling again;
  - wasmtime's async C API (fibers) allows one suspended call per store, so it adds nothing over a blocking host
    word;
  - wasm stack switching would give real coroutines, but wasmtime runs it only on x86_64 Linux (not this Mac's
    aarch64), and its C API switch is a pull request of 2026-11-04 (wasmtime #15148).
- The driver loses its timer table and loop (TIMERS_SLOT, signal_listen, next_due, weekday_now, events_run,
  earliest_timer, timer_fired, timers_forgotten), about 130 lines of core/native, once the page no longer needs
  signal_listen either (card driver-timers-gone).
- Re-entry with wasmtime: a nested turn (a sleep inside a handler) is ordinary warp calls on one stack, no host
  re-entry; a synchronous C callback re-enters the store from the C call's host frame as today, which wasmtime
  allows; nothing calls into a store from another thread.
- Threads later (decision tasks-parallel, card natively-go): a loop per thread's instance; an emit to another
  thread's listeners is a packed node (runtime/packed_nodes.warp) posted to that loop's queue with its wake byte.

## 5. The page
The browser's loop is the loop; the same lib/events.warp turn decides.
- The host drives: the page answers `events_wait` with -1 ("the host drives"), so events_end returns after main, and
  the worker calls the export `events_turn(happened, now)` for each JS task: its one setTimeout to the deadline the
  turn gave, a click or key the page posts, a BroadcastChannel message, a fetch's answer. After each turn the page
  shows the program's value or renders its markup (timerShown and showPageEvent become one path). host-timers.js
  keeps one timeout handle and no schedule (millisecondsUntil and timerLabel go: the label is warp's listening text).
- Wait points: with JSPI (Chrome 137, Firefox 153, Safari 27: every current browser) `events_wait` inside a sleep or
  await is a WebAssembly.Suspending import and the entry exports are wrapped by WebAssembly.promising. The program's
  stack suspends, the worker handles clicks and timers meanwhile, nothing spins; the same import awaits a JS promise
  (card promise-await-page). A browser without JSPI keeps the busy sleep, which then turns the loop at its check
  points, and says once that it blocks the page.
- Behaviours stay samples: held keys and the pointer in SharedArrayBuffer slots, read by system_value (pull); key
  presses and clicks are events through the turn, one path for the canvas and the markup.
- `on interrupt`, `on terminate`, `on file … change` in a page say once that a page has none of them.
- `warp app` (the native web view) runs the native loop with the web view as the window's wait: page_window_run and
  the timers' tick tables go into it.

## 6. Steps
Smallest first. Layers: lib (a module; prelude.warp is baked, so a rebuild), ext (extensions/, a rebuild; generic and
small: an FYI), core (core/ or the native binary's C: the user's OK first, through the Supervisor and the
Interviewer). Tests are outcome tests (code ⇥ value, or printed output for the CLI ones); the timing ones leave at
least 50 ms of slack and never bound a count from above by a few milliseconds.

1. **listener-value** (lib, mechanical). A program ending in a registration has no value to print.
   Files: lib/timers.warp timer_started, lib/prelude.warp listen's callers, or the shown-value rule. Test:
   `'on every day at 21:15 {}'	ø` (prints 1 now).
2. **events-loop** (lib + prelude, careful). lib/events.warp with the turn and the loop; lib/host/native_host/
   events.warp answers `events_wait` (poll() with no descriptors yet, the timeout the deadline); natively lib/timers.warp
   registers with the loop instead of signal_listen; prelude `slept` turns the loop while anything listens; one
   ending `events_end` (extensions/rules/rule_words.warp exited and framed's endings join it) runs the loop, then
   `on exit`; one call however late; monotonic periods; WARP_EVENTS_FOR read in warp; the listening line. The page
   keeps signal_listen and timer_fired until page-turns. Out of scope: signals, descriptors, windows, tasks.
   Tests (test/control/test_event_loop.pairs.warp):
   `'n = 0; on every 100 ms { n += 1 }; sleep 350 ms; n'	3` (0 now);
   `'d = 0; most = 0; on every 10 ms { d += 1; most = max(most, d); sleep 30 ms; d -= 1 }; sleep 200 ms; most'	1`;
   `'n = 0; on every 50 ms { n += 1 }; t = clock(); while clock() - t < 300 {}; sleep 5 ms; n'` gives 1 under Q1 A
   (one call however late), 5 or more under D (the busy loop ticks): written once Q1 is decided;
   test/native/test_events.sh: `on every 100 ms { print "tick"; exit(0) }; on exit { print "bye" }` prints tick,
   then bye (bye first now).
3. **one-on** (lib + ext, careful). `on X {…}`: a value calls `listened(X, handler)` by X's type, a free word is a
   named event; keys' `on(key: Key, …)` becomes Key's `listened`; `every(period)` and `at(time)` give Timer values.
   Files: extensions/sugar/events.warp, lib/keys.warp, lib/timers.warp, lib/channels.warp, lib/tasks.warp (its
   `on $x.stop`). Takes cards on-alarm-keys and timers-every-refusal. Tests: `'broadcast {text: "hi"} on
   "test-nobody"; 5'	5` (test_broadcast:4, red now); `'use timers; n = 0; every(100 ms, () => { n += 1 }); sleep
   350 ms; n'	3`.
4. **os-signals** (core: foreign.c + lib, careful). The wake pipe made by lib (pipe(), O_NONBLOCK, by FFI), its write
   end given to foreign.c's `signal_watch(number, fd)`; `on interrupt`, `on terminate`, `on hangup` as Signal values;
   with `on exit` and no `on interrupt`, ctrl-c and terminate end the loop through `on exit` (Q3, decided). Under Q1
   D a ctrl-c reaches a busy loop through the epoch check too. Tests,
   test/native/test_signals.sh with real signals as Rust's test_system_signals.rs did: `on interrupt { print "bye";
   exit(0) }; print "ready"; sleep 10 s` and a SIGINT after "ready" prints bye; `on exit { print "saved" }; print
   "ready"; sleep 10 s` and a SIGTERM prints saved (Q3); `on interrupt { print "again" }; print "ready"; while yes {}`
   with two SIGINTs half a second apart ends.
5. **tasks-on-loop** (lib, careful). lib/tasks.warp's queue becomes the loop's: a task's sleep is a once timer,
   `await` turns the loop until done, tasks_end joins events_end. Test: `'n = 0; on every 100 ms { n += 1 }; job = go
   { sleep 350 ms; 7 }; [await job, n]'	[7, 3]`.
6. **frames-in-loop** (lib, careful). Frames are the turn's frame phase; host_events_wait waits in the window's
   SDL_WaitEventTimeout or the web view's run loop and returns presses and clicks; paint is a wait point; frames_end,
   page_window_run and timers' tick tables (timers_ticked, TIMER_TICK) go. Files: lib/frames.warp, lib/keys.warp,
   lib/markup.warp, lib/timers.warp, lib/host/native_host/window.warp and webview.warp. Test: test/native/
   test_events.sh `on frame { print "frame" }; on every 100 ms { print "tick"; exit(0) }` prints frame, then tick
   (headless; with a window the user's try line `/Users/me/dev/bin/warp samples/events/ticks.warp` with a frame
   added ticks while the window shows).
7. **routes-in-loop** (lib, careful). The server's listening socket is a watched descriptor; a request is answered
   in a turn, so timers and channels run beside it. File lib/web.warp. Test, in test/native/test_server_builds.sh's
   manner: a program serving `/n` with `on every 100 ms { n += 1 }` answers two GETs 300 ms apart with a larger n.
8. **file-watch** (lib, careful). `on file "x" change {…}` as a FileWatch source: kqueue EVFILT_VNODE (macOS) or an
   inotify descriptor (Linux) in the wait, the stamp as fallback; `on message from` rides on it instead of the 20 ms
   timer. Files: lib/events.warp, lib/host/native_host/events.warp, lib/channels.warp. Tests: `'n = 0; write("w.txt",
   "a"); on file "w.txt" change { n += 1 }; write("w.txt", "b"); sleep 200 ms; delete("w.txt"); n'	1`;
   test_named_broadcast and test_broadcast stay green.
9. **page-turns** (lib + web/playground, careful). The page drives the turn (section 5): host-timers.js keeps one
   timeout, worker.js calls events_turn for timeouts, clicks, keys, messages and fetch answers; the busy sleep turns
   the loop at its check points. Takes card once-event. Tests in the page lane: test_event_loop's first line gives 3
   there too; test_page_events, test_clock_timers stay green.
10. **page-jspi-waits** (web/playground, careful). events_wait as a Suspending import, entries wrapped by promising;
    the busy sleep only without JSPI. Takes card promise-await-page. Test: test_js_callbacks:4 and :7 green in the
    page lane.
11. **deferred-callbacks** (core: foreign.c + lib/callbacks.warp, careful). Thread-aware trampolines and the ring;
    `cancel cb` gives a slot back; overflow warns once. Test (test/ffi/test_c_callbacks.pairs.warp): a callback passed
    to pthread_create as its start routine runs in the loop: `n = 0; f = c_callback(p => { n += 1; 0 }, ["handle"]);
    pthread_create(calloc(1, 8), 0, f, 0); sleep 100 ms; n` gives 1 (pthread_create declared from 'c').
12. **cancel-handles** (lib, mechanical; Q2 decided). `cancel h` for timers and every source. Test: `'n = 0; t = on
    every 20 ms { n += 1; if n == 2 { cancel t } }; sleep 300 ms; n'	2`.
13. **driver-timers-gone** (core removal, mechanical; after events-loop and page-turns). The driver's timer table and
    loop go, signal_listen leaves the host words. Test: smoke, self-check, the tests above stay green.
14. **gui-wake** (core: foreign.c, careful; later). The waker thread (section 4, C part 3): a SIGINT or a socket
    wakes a window's wait at once. Try line only (a window).
15. **busy-loop-events** (core: driver + foreign.c, careful; if Q1 picks D). First a probe: may the store's epoch
    callback (wasmtime_store_epoch_deadline_callback) call into the store? Then: epoch interruption on for programs
    that listen, a C timer thread bumping the epoch at the next due time (wasm_engine_increment_epoch), the callback
    running one turn on main's thread. Test: step 2's busy-loop line gives 5 or more.

Later, cards that exist: natively-go and stop-endless-spin (a loop per thread); load-time-linking (components: map
Readable to stream<u8> and Completion to future<T>, and let wasmtime's component-model-async own the wait once its C
API is complete; the turn stays the policy).

## 7. Risks and open questions
Risks:
- Timing tests under a loaded Mac (five builds at once): generous slack, counts that do not hang on wake-up times.
- A handler that sleeps now lets other handlers run in between, so what it read before the sleep may have changed
  (the interleaving every await has); the held source stops only its own re-entry. The wiki says so with the step.
- One sleep: prelude's `slept`, tasks' main sleep and sound's waits (lib/sound.warp wait_sound, functions.warp retry
  call sleep_ms directly) must all wait through the loop, or a timer starves while they block.
- The program runs on the process's main thread, which SDL and the web view need; the ring keeps foreign threads out
  of wasm. SDL_HINT_NO_SIGNAL_HANDLERS stays "1" (native_host/platform.warp), so SIGINT stays ours.
- wasmtime's own signal handlers (SIGSEGV, SIGILL for traps) leave SIGINT and SIGTERM alone; sigaction with
  SA_RESTART, and a poll() ended by EINTR counts as a wake.
- poll() is right for tens of descriptors; a server with thousands of connections wants kqueue or epoll behind the
  same host word, without a change to the program side.
- page-turns before JSPI re-enters the instance from the busy sleep's import: legal in JS, and the held-source rule
  applies the same way.
- JSPI makes the worker's run path asynchronous (driver.js programRan): the largest page change; a program that never
  waits keeps the synchronous path.
- A deferred callback gives C 0 at once: a foreign thread that needs a real answer (audio rendering) cannot be an
  event; real-time audio stays a ring of samples warp fills ahead (lib/sound.warp), never a call into wasm.
- macOS's modal window resize blocks the main loop while dragging: frames pause during a resize (SDL's event watch
  could paint, later).

Decided by the user 2026-11-09 (through warp-interviewer; these overrule the options below):
- event-delivery-points: one rule for native and the page. A handler runs on the program's thread at its waits
  (sleep, await, receive, a painted frame) and at loop starts, through one compiled flag check, only in programs that
  listen (~15 emitter lines, core OK given). A host thread does the waiting and only queues events. Nothing is
  delivered while a handler runs. Test: `n = 0; on every 50 ms { n += 1 }; sleep 175 ms; n` → 3. Wasmtime epochs may
  replace the native check later, if a probe shows the epoch callback may call into the store (busy-loop-events).
- threaded-handlers: real threads are planned too, from what is decided already (go tasks truly parallel natively,
  `shared` atomic values): `shared n = 0; on every 50 ms go { n += 1 }`. A threaded handler sees only shared values.
  Simple native scenarios first; the page stays cooperative.

Questions for the user, through warp-interviewer 2026-11-09:
- cancel-timer (Q2), decided A: `t = on every 1 second {…}; cancel t`.
- ctrl-c-runs-exit (Q3), decided A: with `on exit` and no `on interrupt`, ctrl-c and a kill run `on exit`, then the
  run ends with 130 (143 for a kill); a second ctrl-c within a second ends it at once.
- event-delivery-points (Q1), open (the user rejected A and asked about a second thread; being discussed):
  - A: wait points only (after main, sleep, await, receive, painted frames). The JS / libuv model.
  - B: also a check compiled at every loop start (the Rust check points): an emitter change, a cost per iteration.
  - C: handlers on a second thread. A wasmtime store is single-threaded and GC objects cannot cross threads
    (shared-everything threads is an early proposal, not in wasmtime), so the handler runs a second instance with
    its own globals: `on every 1 s { n += 1 }` would not change main's n unless n is `shared`, and every handler
    touching main's data races with it.
  - D, recommended: epoch safe points on main's thread. A C timer thread bumps wasmtime's epoch when a source is due;
    wasmtime's compiled code checks it at each loop header and function entry, and the store's epoch callback runs a
    turn before main goes on (preemption at safe points, as in Go and Erlang). No emitter change, a cost only in
    programs that listen, ctrl-c reaches busy loops; a handler may run in the middle of one of main's updates
    (interrupt semantics). Native only: the page's worker cannot be preempted, so it keeps A. To verify by a probe:
    that the epoch callback may call into the store.

Decided here by taste and evidence (an objection goes to the Interviewer): emit stays synchronous (P163); the turn's
phase order; the keep-alive list; back-pressure by coalescing, lazy reads and bounded loud queues; monotonic periods;
signal names as a library enum; `on X` a value's source or a free word's named event; JSPI for the page's waits; no
WASI 0.3 yet; C only for the signal handler, foreign-thread callbacks and the GUI waker.
Still parked from before: event-handlers (notes/open_decisions.md: `it` or a chosen name for the payload, declared
events); this plan keeps `event` and any word.
