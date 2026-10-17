# System signals: hooking into the world outside the module

Companion of notes/signals.md (signals inside a program); this is the outside. The original sketch (catalog of
sources per platform, bus design, delivery rules, the dirty parts): notes/OLD/system_signals_sketch.md.

## The idea
The program says what it listens to with the same words as for its own signals: `on interrupt {…}`,
`on file "notes.txt" change {…}`, `whenever battery < 20% {…}`, `on every 5 minutes {…}`. The compiler finds a name it
does not define among the system signals and emits host calls plus an exported handler; the runtime
(crates/warp-runtime/src/system_signals.rs natively, so built executables keep them; host.js and host-timers.js in the
page) runs due handlers at check points. WASM never sees a platform API. Imports follow use: a program without system
signals imports nothing.

- Check points: every loop start (wasm_emitter control_flow, next to task_poll), sleeps (in 10 ms slices once
  watching; they wake for timers), main's start and end. A handler never runs in the middle of another.
- Resident programs: `warp run`, `warp <file>` and executables stay after main while a timer, watch or listener lives
  and say so once on stderr ("listening: every 5 seconds (ctrl-c to stop)"; frames are left out). In-process eval and
  tests never wait.
- Unknown names are not guessed (notes/welcoming.md); a source missing on a platform is loud, never a silent no-op.
- Sources not built yet (each a card when needed): resize, power sleep/wake, locale, visibility, gamepad, sensors,
  stdin lines, directories and separate `created` / `deleted` events, `stop listening`, event-driven connectors
  instead of polling, Windows beyond ctrl-c.

## Interrupt and exit
- `on interrupt {…}` (event_signals SYSTEM_EVENTS: the exported `on·interrupt`, no emit needed): a program with it
  imports the host word `signal_poll`. The first poll installs a SIGINT handler that only sets a flag (Windows:
  SetConsoleCtrlHandler for ctrl-c and ctrl-break, checked with `cargo check --target x86_64-pc-windows-gnu -p
  warp-runtime`, never run there); the poll then runs the handler (`event` ø). A second ctrl-c within a second, or
  before the handler ran, exits with 130 (`ctrl_c_ends_run`, tests/control/test_ctrl_c_rule.rs), so a handler never
  makes a program unstoppable. A ctrl-c ends a `sleep` early. Only a program with `on interrupt` watches ctrl-c.
  tests/control/test_system_signals.rs sends a real SIGINT. host.js has `signal_poll` as a no-op.
- `exit`, `exit(code)` (P121): a host word whose error unwinds the run (ExitRequest); the run's value is ø, the CLI
  and executables exit with the code, the page ends the run, `warp app` closes its window with the code (site.js
  endApp → webview.rs "warp-exit N"). A bare `exit` is `exit(0)`; `on key { exit 3 }` is that one call.
- `on exit {…}`: the exported `on·exit` runs once as the run ends (after main and the timers, or at `exit(code)`,
  whose code stays); never after a failure, and not at a hard second ctrl-c (system_signals::with_exit_handler,
  host.js withExitHandler). `event` is ø (tests/control/test_exit_signal.rs).

## Timers
One host word starts every timer: `signal_listen(kind, N, value)`, kind 0 every / 1 frame / 2 clock; the handler is
`on·every·N` (src/lowering/signals/system_signals.rs).
- `on every 50 ms {…}` (P120).
- Times of day (P135): `on every day at 9:00 {…}`, `at 9:00 {…}` once. Value `weekdays·10000 + minute_of_day`,
  weekdays a mask (bit 0 Sunday … bit 6 Saturday): `day` 127, `monday`/`mondays` 2, `weekday` 62, `weekend` 65, 0 once.
  Times `21:30`, `9:30pm`, `9 pm` (12am is 0:00, 12pm noon); `at 13pm`, `25:00` are "needs a time of day". Each next
  time is computed from the local clock anew (localtime_r), so summer time shifts nothing; a one-shot timer is
  dropped after it fired (tests/control/test_daily_timer.rs, test_time_of_day.rs). Not yet: several days (`on every
  monday and friday`), dates, cron strings.
- `on frame {…}` (Trigger::WindowShows): every 16 ms while the paint window shows, ending with the window; headless
  once (one paint = one PNG); in the page every 16 ms until the next run. Replaces `while window_open { …; sleep 16ms }`.
- A body after a colon (`on every 5 seconds: print n`) is the rest of the line (words_and_body).
- Playground: host.js records the timers while main runs, worker.js starts them after it (setInterval, setTimeout to
  the next local time) until the next run; each tick shows the output binding `page·value`.

## Files
`on file "notes.txt" change {…}` (or `changes`): `on·file·N()` and `signal_watch(N, "notes.txt")`, a 100 ms timer
running the handler when the modification time or size changed (written, appeared, deleted). Polling on the check
points needs no dependency or thread; the notify crate can replace the stat if many files or directories are
watched. The playground warns (a page has no files).

## Keys and clicks over the paint window
- `on key {…}` natively: system_values.rs queues each press from the viewer's input lines and `system_value
  ("key_pressed")` pops one; a frame handler after the first `on key` emits `key{key: …}` per press, named as the
  page's KeyboardEvent.key (the character, ArrowUp…, Enter, Escape, Backspace, Tab: KEY_NAMES). Not for a page (the
  page raises key itself). Not yet: keys in a terminal, the `warp app` web view.
- `on click {…}` natively: button presses with their place (`click_pressed`, `click_x`, `click_y`) emitted as
  `click{x, y}` in frame pixels, as the page's canvas raises it.
- Key names (lowering/collections/key_names.rs): inside `on key {…}` a bare `key` is `event.key` (a program variable
  `key` wins); Escape/ESC, Enter/RETURN, Tab, Backspace, Space, Delete, UP/DOWN/LEFT/RIGHT, F1 … F12 name keys, letters
  stay texts (`key == "a"`). A comparison with a word naming nothing (`key == q`) warns: it never holds.
- `key_down`: whether a key is held now (for loops, like mouse_down).
- `held_key` names the key held now ("" for none); a comparison `held_key == LEFT` asks whether that key is among
  those held (host word `key_held(code)`), so diagonals work (samples/events/arrow_keys.warp,
  tests/control/test_held_key.rs). Natively the viewer's input line carries every held code (paint_window.rs, cleared
  on focus loss); the playground keeps them in shared slots "held 0" … "held 7" (canvas.js KEY_CODES).

## System values (P136, tests/control/test_system_values.rs)
`battery` (percent), `charging`, `online`, `dark mode` read the host word `system_value(name)` unless the program
binds the name. Readers in crates/warp-runtime/src/system_values.rs: macOS `pmset -g batt` and `defaults read -g
AppleInterfaceStyle`, Linux /sys/class/power_supply and `gsettings`; `online` asks whether a UDP socket can connect
to 1.1.1.1 (nothing is sent). A reading is kept for a second. A value the machine cannot give is a loud run-time
error ("battery: this Mac has no battery").
- Listening: lowering/signals/system_values.rs marks the reads (`system·battery`) before
  signal_values::poll_shared, which checks them like shared values in `on·shared`, adds a 1000 ms timer after the
  first listener, then makes the marks host calls. `whenever battery < 20% {…}`: `20%` is 20.
- Clipboard (tests/control/test_clipboard.rs): `on clipboard change {…}` is `on change` of `clipboard count`
  (macOS NSPasteboard changeCount through the Objective-C runtime, AppKit dlopened on first use; it reads no content,
  so no paste prompt; elsewhere the content's hash). `clipboard` is its text (pbpaste, wl-paste or xclip), never
  polled.
- The page reads `online` and `dark mode` (the page tells its worker, again on each change) and says what it cannot
  read (battery, the clipboard: the browser API is asynchronous).
- Not yet: `on dark mode {…}` (use `whenever dark mode`), percent literals elsewhere.

## Channels (tests/control/test_broadcast.rs, test_named_broadcast.rs)
- `broadcast value on "chat"` sends any value as warp text to every program on this machine listening with
  `on message from "chat" {…}` (`event` the value); without a name the channel is "warp". `send value to "chat"`
  is the same.
- Natively (unix, src/host/channels.rs) a channel is the directory `/tmp/warp-channels-<user>/<channel>` of Unix
  datagram sockets, one per listener, removed when its run ends (a socket path has at most 104 bytes, macOS's temp
  directory takes about 50). A listener is a 20 ms timer handler pulling `channel_pending` / `channel_next`. The
  playground uses a `BroadcastChannel`, so programs in two tabs talk. Windows has no channels yet.
- Named events (P129): `broadcast stop the machine{reason: "heat"} on "chat"` sends to the sub-channel `chat/stop the
  machine` (directory `stop_the_machine`), heard only by `on stop the machine from "chat" {…}`. `send "file system
  full"` is a named event without data on "warp", heard by `on "file system full" {…}`. A named event is two or more
  plain words, or words ending in `name{data}`; one word without data stays a value. Without `from` or a text name,
  `on stop the machine {…}` is the in-program handler of `emit`, so no program listens by accident.
