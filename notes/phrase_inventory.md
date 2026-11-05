# Phrase rules → plain functions with named arguments (decision named-argument-phrases, inventory 2026-11-05)

`give(v, to) := v + to; give 2 to: 40` gives 42 today. Proposed function and call per lib phrase rule; test/sample
lines counted by grep over test/ and samples/ (*.warp; a count may include a comment or prose line).

Probed on main: `twice_do(body: block)` with `twice_do { … }` works, `when_true { c } { body }` works,
`every_do every: 2 { 9 }` works; **`react 1 finishes: { 7 }` fails ("not a function")** — a labeled block after a
positional argument; `then:` as a label is refused (keyword).

## Multi-word phrases (the conversions)
| lib | rule spelling | plain function | named-argument call | test/sample lines (files) |
|---|---|---|---|---|
| channels | `send v to c` | `send(value, to: channel)` | `send v to: c` | 4 (3) test_channels, test_named_broadcast |
| english | `add x to xs` | `add(item, to: list)` | `add x to: xs` | 12 (5) test_add_to, test_add_to_text, test_preposition_calls, test_add_subtract_words, test_sweep_fixes |
| english | `set field of object to value` | `set(field, of: object, to: value)` | `set name of: p to: 3` (field is a name: needs a quoted/symbol parameter) | 3 (3) test_english_phrases ×2, test_window_size |
| tasks | `await x within t` / `… or d` | `await(x, within: duration, or: fallback: block)` | `await x within: 2s or: { 0 }` | 6 (2) test_async_ports |
| tasks | `await all xs` / `any` / `first` | `await(all: xs)`, `await(any: xs)`, `await(first: xs)` | `await all: xs` | ~35 (17), test/control task tests, samples/tasks/async, threads, kitchensink |
| tasks | `once x finishes {…}` / `pauses` | `once(task, finishes: block)` / `once(task, pauses: block)` | `once x finishes: {…}` (**fails today**, see above) | 6 (2) test_tasks, test_threads |
| tasks | `on x.stop {…}` / `on x.cancel {…}` | `on(stop: task, body: block)` / `on(cancel: task, …)` | `on stop: x {…}` | 2 (2) test_threads, test_tasks |
| tasks | `after c return x` | `after(condition: block, return: block)` | `after c return: x` | 4 (2) test_after, samples/tasks/async |
| timers | `on every period {…}`, `on every n unit {…}` | `on(every: duration, body: block)` | `on every: 1 second {…}` | ~17 (11) test_timers, test_clock_timers, test_daily_timer, test_colon_handlers, test_empty_bodies, test_unit_spellings, test_time_of_day, test_slow_handler, test_system_signals, test_prefix_keyword_condition, test_implicit_rule_words; samples/advanced/mandelbrot_canvas |
| timers | `on every day at t {…}` | `on(every: day, at: time, body: block)` | `on every: day at: 9:30 {…}` | 3 (2) test_daily_timer, test_time_of_day |
| timers | `at h:m {…}` | `at(time, body: block)` | `at 9:30 {…}` (already one word + argument) | 2 (2) |
| signals | `on change x {…}` / `on set x {…}` | `on(change: name, body: block)` / `on(set: name, …)` (x is a variable's name, not its value) | `on change: x {…}` | **~125 (28)**: most of test/control signal tests, samples/signals/signal_fields, kitchensink |
| signals | `whenever c {…}` | `whenever(condition: block, body: block)` | unchanged: `whenever c {…}` (one word) | 0 to change (~35 lines use it) |
| signals | `once c {…}` | `once(condition: block, body: block)` | unchanged (one word); clashes with tasks' `once x finishes` | 0 to change (~16 lines) |
| channels | `on message from name {…}` | `on(message_from: name, body: block)` | `on message_from: n {…}` | 0 |

## One-word prefixes (no phrase: stay rules or become one-argument functions)
`go x`, `go {…}`, `await x`, `await go {…}`, `finish x`, `cancel x`, `stop x`, `end x` (refused), `pause x`,
`resume x`, `shared d`, `channel name`: one word and one argument; `cancel`/`stop` need the variable's name
(spelled), `go`/`await go` a lazy block — these stay rules unless a `block`/name parameter covers them.
`go for x in xs {…}` and `for item in c {…}` (channels) are loop forms, stay rules.

## Open for the Interviewer
1. `on change x {…}` → `on change: x {…}` changes ~125 test lines; keep `on change x` as the one rule?
2. `await all xs` → `await all: xs` (~35 lines).
3. A labeled block after a positional (`once x finishes: {…}`) fails today: fix in the call reader first.
4. `set name of p to 3`: field as a name parameter (symbol), or keep the rule.
