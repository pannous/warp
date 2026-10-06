# System signals: hooking into the world outside the module (sketch, 2026-10-06)

User: "contemplate a more general concept where we can also hook into the signals of the system, which is obviously
not part of WebAssembly, so we need something in the runtime and connectors on many platforms; it can get a bit
dirty, just sketch it out". Companion of notes/signals.md (signals inside a program); this is the outside.

## The idea in one paragraph

The program says *what* it listens to with the same words as for its own signals: `on interrupt {…}`,
`on file "notes.txt" change {…}`, `whenever battery < 20% {…}`, `on dark mode {…}`. The compiler sees a name it does not
define, finds it in the **system signal catalog**, and emits an import `signal_subscribe(name, filter)` plus an exported
dispatcher `on·signal(id, event)`. The **runtime** (wasmtime host natively, host.js in the page) owns a **signal bus**:
platform **connectors** feed it, it queues events as wasp nodes, and delivers them to the module at **check points**.
WASM never sees a platform API; every dirty part lives in a connector.

```
 OS / browser / device                runtime (host)                         module (WASM GC)
 ─────────────────────               ───────────────────────────            ─────────────────────────
 SIGINT, SIGTERM          ─┐         connectors (one per source,  ─┐        on interrupt { … }
 FSEvents/inotify/RDCW     ├──────▶  feature-gated per platform)    │        on file "x" change { … }
 power, network, theme     │         │                              │        whenever battery < 20% { … }
 DOM events, visibility    │         ▼                              │
 MIDI, gamepad, D-Bus …   ─┘         signal bus: queue of           │  check point (await, loop back-edge,
                                     (subscription id, event node)  ├─▶ host call, idle event loop):
                                     filters, coalescing, rate      │   on·signal(id, event) → handler
 OS notification, kill,  ◀────────── outbound: broadcast / send  ◀──┘   raise / broadcast name{data}
 postMessage, webhook                                                    
```

## The catalog: names, not APIs

A signal is named in words, with a namespace for the ambiguous ones; the payload is a wasp node, so it reads as data.

| signal | payload (`event`) | native connector | browser connector |
|---|---|---|---|
| `interrupt` (ctrl-c), `terminate`, `hangup`, `user signal 1` | `{signal:"SIGINT"}` | signal-hook (unix), SetConsoleCtrlHandler (Windows) | `beforeunload`, `pagehide` |
| `resize` (terminal / window) | `{width height}` | SIGWINCH | `resize` |
| `file "p" change` / `created` / `deleted` | `{path kind}` | notify crate (FSEvents, inotify, ReadDirectoryChangesW) | File System Observer (where available), else none |
| `clock every 5s`, `at 9:00`, `cron "…"` | `{time}` | timer thread | `setInterval` |
| `network online` / `offline` | `{online}` | SCNetworkReachability / netlink / NLM | `online` / `offline` |
| `power sleep` / `wake`, `battery` | `{level charging}` | IOKit / UPower (D-Bus) / WM_POWERBROADCAST | Battery API (Chromium) |
| `dark mode`, `locale`, `clipboard change` | `{dark}` … | NSDistributedNotificationCenter, gsettings, WM_SETTINGCHANGE | `matchMedia(prefers-color-scheme)` |
| `visible` / `hidden` (app lifecycle) | `{visible}` | NSWorkspace, Android lifecycle | `visibilitychange` |
| `click`, `key`, `input` on an element | `{target value key}` | (GUI toolkit, later) | DOM events |
| `midi`, `gamepad`, `sensor` | device-specific | midir, gilrs | Web MIDI, Gamepad API |
| `message from "channel"` | the sent node | unix socket / named pipe / MQTT / D-Bus connector | `BroadcastChannel`, `postMessage`, WebSocket |
| `stdin line` | `{text}` | reader thread | (playground input box) |

Unknown names are not guessed (notes/welcoming.md): `on interupt {…}` is "did you mean `interrupt`?"; a signal
whose connector is missing on the platform is a loud compile-time warning and a runtime notice when it would subscribe,
never a silent no-op ("loud and routed to the owner": the connector table names who must add it).

## Runtime: the signal bus

- **Host interface** (two imports, one export, like the task words):
  - `signal_subscribe(name: text, filter: node) -> id`: the host starts the connector (lazily, once per source) and
    returns a subscription id. `signal_unsubscribe(id)`.
  - `signal_poll() -> i32`: pending count; the emitter calls it at check points, then pulls `signal_next()` events
    and calls the exported dispatcher `on·signal(id, event)`, which switches on id to the handler.
  - outbound `signal_send(name, node)`: `broadcast name{…}` / `send … to "channel"` / `notify "text"`.
- **Rust**: `trait Connector { fn source(&self) -> &str; fn start(&self, filter: &Node, sink: Sink) -> Result<Handle>; }`
  in `src/system_signals/` with one module per source family, each behind a cargo feature
  (`signals-fs`, `signals-power` …) and `#[cfg(target_os)]`. `Sink` is a channel into the bus; events are
  `TaskValue`s already (they cross threads, notes/threads.md), rebuilt as nodes in the instance.
- **Browser**: `registerConnector(name, start)` in host.js, mirroring `registerForeignRuntime`. The program runs in a
  Worker: connectors on DOM events live in the page and forward through the worker message loop.

## Delivery: when does a handler run (the hard part)

WASM cannot be interrupted at an arbitrary instruction to run a handler, and must never run two handlers at once on
one instance. So events are **queued and delivered at check points**, the same mechanism task handlers use
(task_poll, notes/threads.md step 4):

1. **Check points while main runs**: `await`, `sleep`, loop back-edges (only in programs that subscribe), every host
   call. A tight loop without them still sees the signal because native uses epoch interruption: the epoch callback
   sees the bus non-empty and makes the next back-edge poll (no emitted code change for programs without signals).
2. **Resident programs**: a program whose main ends with live subscriptions does not exit; the runtime keeps the
   instance and dispatches events from its event loop (natively a blocking wait on the bus, in the page the worker's
   message loop). `exit`, `stop listening`, or all subscriptions being `once` and fired end it. `warp run` prints
   "listening for interrupt, file notes.txt change (ctrl-c to stop)" so the waiting is never silent.
3. **ctrl-c itself**: without an `on interrupt` handler the default stays (exit). With one, the first ctrl-c runs the
   handler at the next check point; a second ctrl-c within 1 s exits hard, so a stuck program can still be stopped.
4. **Ordering and coalescing**: per subscription FIFO; bursty sources (resize, file change) coalesce to the last
   event per check point unless the handler asks for `every`; a full queue drops the oldest and counts the drops
   (`event.dropped`), loudly.

## Language surface (nothing new beyond notes/signals.md)

```wasp
on interrupt { print "saving…"; save(); exit }
on file "config.wasp" change { config = load "config.wasp" }   // hot reload
whenever battery < 20% { notify "plug me in" }                 // a system value as a derived signal
on every 5 minutes { backup() }
on message from "chat" { print event.text }
broadcast build finished{ok:true}                               // outbound, to whoever listens
```

`battery`, `dark mode`, `online` are **system values**: reading them is a host call (pull), and listening to them is a
subscription (push), the same push-pull split as `:=` values inside the program. So a system value composes with
derived signals: `low := battery < 20% and not charging` is a derived signal over two system values.

## Effects and capabilities

Subscribing is an effect: a new `Effect::Signal` (effects.rs) per program and per function, and a capability the
host grants: `eval_untrusted` grants none, the playground grants the browser ones, `warp run` grants all with
`--deny signals:file` to restrict. Imports follow effects (as WASI does today): a program without system signals
imports nothing and starts no connector thread.

## Where it gets dirty (honest list)

- **Platform matrix**: every source has 3+ native APIs; macOS-only first (FSEvents, IOKit, NSDistributedNotification
  via objc2), Linux second, Windows third; each missing pair is a board card, not a silent gap.
- **Unix signal handlers** may only set a flag (async-signal safety): signal-hook's pipe/flag, never wasm calls.
- **Threads**: connectors run on their own threads, the instance is single-threaded; the bus is the only crossing.
- **Tasks**: which instance gets an event? Rule: the one that subscribed; a task's subscriptions end with the task.
- **Browser limits**: no SIGINT, no file watching (mostly), battery only in Chromium, everything permission-gated;
  host.js reports unsupported sources once.
- **WASI**: preview 2 has `wasi:io/poll` pollables (clocks, sockets, streams) and nothing for signals or files;
  a component build (`warp build` to a component) could express subscriptions as a custom WIT interface
  `warp:signals/bus` with `subscribe`/`next` returning pollables, so other hosts can implement it later.
- **AOT executables** (warp-runtime stub, notes/aot.md): the connectors must live in crates/warp-runtime, not in the
  compiler, so a built executable keeps its signals; feature flags keep the stub small.

## Smallest useful first step

`on interrupt {…}` natively: signal-hook flag, `signal_subscribe("interrupt")` / `signal_poll` host words, the poll at
loop back-edges of subscribing programs, the double ctrl-c escape, and a resident main that waits for the next event.
Then `on file "…" change` (notify crate) and `on every 5 seconds`, then the playground's DOM and visibility
connectors. Board card: signals-system (phase 7 of notes/signals.md, split out).

## Status
- `on interrupt {…}` natively (unix): event_signals SYSTEM_EVENTS makes the exported handler `on·interrupt` without a
  raise; a program with it imports the host word `signal_poll` (crates/warp-runtime host_words.rs), called at every
  loop start (wasm_emitter control_flow, next to task_poll). The first poll installs a libc SIGINT handler that only
  sets a flag; the poll then runs the handler (with ø as `event` when it reads one). A second ctrl-c within a second,
  or before the handler ran, exits with 130, so a handler that ignores ctrl-c never makes a program unstoppable.
  main polls at its start (which installs the handler) and before it returns; a ctrl-c also ends a `sleep` early
  (it sleeps in 10 ms slices once watching) and runs the handler. Test: tests/control/test_system_signals.rs
  (a real `warp run` gets a real SIGINT). The playground's host.js has `signal_poll` as a no-op.
- Timers (P120, 2026-10-06): `on every 50 ms {…}` (src/lowering/system_signals.rs) is the handler `on·every·N()` and
  `signal_every(N, ms)` where it is written; the runtime (crates/warp-runtime/src/system_signals.rs) runs due handlers
  at every check point (loop starts, sleeps, which wake for them, main's start and end). `warp run`, `warp <file>` and
  executables stay after main while a timer lives and say so once on stderr ("listening: every 5 seconds (ctrl-c to
  stop)"); in-process eval and tests never wait. Only a program with `on interrupt` watches ctrl-c: any other ends at
  one ctrl-c as usual.
- `exit`, `exit(code)` (P121): a host word whose error unwinds the run (ExitRequest); the run's value is ø, the CLI and
  executables exit with the code, the page ends the run. A bare `exit` / `exit()` statement is `exit(0)`.
- File change (2026-10-06): `on file "notes.txt" change {…}` (or `changes`) is the handler `on·file·N()` and
  `signal_watch(N, "notes.txt")`: a timer of 100 ms that runs the handler when the file's modification time or size
  changed since the last check (written, appeared, deleted). Polling on the existing check points instead of the
  notify crate: no dependency, no thread, and a handler can only run at a check point anyway; notify (8.2 is in the
  registry) can replace the stat later if many files or directories are watched. Programs stay while a watch lives.
  The playground warns (a page has no files). Test: a_file_change_runs_its_handler.
- `on exit {…}` (card g-3Gdo; syntax an assumption queued with the Interviewer): the exported handler `on·exit`
  (event_signals SYSTEM_EVENTS) runs once as the run ends: after main returns and the timers stop, or at `exit(code)`,
  whose code stays (an `exit` inside the handler sets its own); never after a failure, and not at a hard second
  ctrl-c. warp-runtime system_signals::with_exit_handler wraps the run natively (wasm_reader, standalone executables),
  host.js withExitHandler in the page. `event` is ø for now (the exit code would need a host-built Int).
  Test: tests/control/test_exit_signal.rs.
- Times of day (P135, user: `on every day at 9:00 {…}`, and `at 9:00 {…}` runs once; card time-day): the timer
  handler `on·every·N` started by `signal_daily(N, minute_of_day, weekdays)` or, for `at 9:00 {…}`,
  `signal_at(N, minute_of_day)`. `weekdays` is a mask, bit 0 Sunday … bit 6 Saturday (C's tm_wday): `day` 127,
  `monday` (or `mondays`) 2, `weekday` Monday to Friday 62, `weekend` 65. Times are `21:30`, `9:30pm`, `9:30 pm`,
  `9pm`, `9 pm` (12am is 0:00, 12pm noon); `at 13pm`, `25:00` are "needs a time of day", an unknown day word names the
  day words. Each next time is computed from the local clock anew (libc localtime_r, UTC without it), so summer time
  shifts nothing; a one-shot timer is dropped after it fired, so `warp run` ends then ("listening: at 9:00",
  "listening: every monday at 9:00"). Tests: tests/control/test_daily_timer.rs, test_time_of_day.rs.
  Not yet: several days (`on every monday and friday`), dates (`at 2026-12-24 18:00`), cron strings; the colon body
  (`at 9pm: print n`) works only as a whole program (card colon-body).
- Not yet: Windows (SetConsoleCtrlHandler), directories and `created` / `deleted` as separate events, timers in the
  playground (a warning says so), `stop listening`.
- Channels (branch signals-broadcast, warp-3a; tests/control/test_broadcast.rs; syntax an assumption queued with the
  Interviewer): `broadcast value on "chat"` sends any value (`{text: "hi"}`, `21`) as wasp text to every program on this
  machine listening with `on message from "chat" {…}`, where `event` is the value; without a name both use the channel
  "warp". Natively (unix, src/channels.rs) a channel is the directory `<temp>/warp-channels/<channel>` of Unix datagram
  sockets, one per listener, removed when its run ends; a broadcast sends to each and drops dead ones. A listener is
  a timer handler (`on·every·N`, every 20 ms, lowering/system_signals.rs) pulling `channel_pending` / `channel_next`,
  so a listening program stays after main in `warp run` like one with a timer. The playground sends on a
  `BroadcastChannel`, but does not receive yet (no timers there); Windows has no channels yet (named pipes).
  Open: named events across programs (`broadcast stop the machine{…}` → `on stop the machine from "chat"`).
