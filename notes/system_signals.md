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
