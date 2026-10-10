# Dates at run time (cards runtime-dates, run-time-dates)

`now` reads the host clock when the program runs, not when it compiles (decision 7e9b01db4: dates run at run time).
Date, local time and instant literals are run-time values too: a program that prints, loops or keeps them in
variables runs (stage B1a of run-time-dates).

## Representation
A time is a `$Node` of `Kind::Time` (18): `data` = `$i64box` of its position, `value` = null. Its `TimeForm`
(time.rs) sits in the info bits above the kind (`kind = Time | form << 8`):
- Instant = 0: nanoseconds since 1970 UTC
- Date = 1: days since 1970
- Local = 2: wall-clock nanoseconds as if UTC

The position is calendar.rs `Time::position`. The analyzer types a time `Kind::Data`. Readers
(`wasm_reader::struct_node`, `web.rs cell_node`) go through `time::time_node(info, position)`.
Zoned times and durations have no run-time form yet (stage B1b): a program that runs with one fails with
"…: durations/zoned times in a program that runs are not supported yet (card run-time-dates)".

## Lowering (`time::lower`, called by pipeline.rs)
- A free `now` (also `now.hour`) becomes `instant_at(clock())` (`time::now_call`, system_values).
- A program with forms only the compile-time evaluator knows (literals, durations, `date(…)`, `t in "Zone"`) is
  first evaluated whole. If that succeeds, the program is its answer. A real error is the program's error.
- If the evaluator meets something it does not know (a variable, print, a loop: an `UNSUPPORTED` error),
  `fold_constants` folds the largest constant time expressions (time literals, unit and time words, field names after
  a dot) into `time_of(form, position)`, and the rest runs.
- Known edge: a variable named like a unit word (`days`) counts as a time word.

## The local wall clock of an instant
An instant's calendar fields (`now.hour`, `t.year`, …) are the wall clock of the environment's time zone (user,
2026-10-10: "provide the default local from the environment through the runtime"); `epoch_seconds` needs none.
- Host word `local_offset(ms) -> seconds east of UTC` at that instant, DST included: natively libc `localtime_r`'s
  `tm_gmtoff` (TZ, else the system zone; UTC without one, `system_signals::local_offset_at`), in the browser
  `-Date.getTimezoneOffset() * 60` (host.js).
- `extract_host_words` imports it for a program that makes an instant (`time::makes_instant`); `time_field` adds the
  offset before splitting the position.
- The compile-time evaluator leaves instant fields to the run (the run's zone, not the compiler's); calendar.rs
  `Time::field` uses `time::local_offset` for the same result natively.
- Placing in a zone stays explicit for any other zone: `t in "Europe/Berlin"`.

## Emitter (src/wasm_emitter/times.rs)
- `instant_at(i64 ms) -> node`; `time_of` constants are emitted inline (`emit_time_constant`).
- `civil_from_days`, `days_from_civil`: Hinnant's civil calendar. `time_parts(form, position) -> (days, nanos)`.
- `time_field(node, index of TIME_FIELDS) -> i64`. The call site (`emit_time_field_lookup`) first refuses the fields
  a form lacks, with the message of `TimeForm::example().field(name)` (`date has no hour`).
- `time_text(node) -> text`: RFC 3339 like calendar.rs `Display` (seconds left out when zero, fraction trimmed, `Z`
  only on an instant); list_join calls it (print, `+` with a text, `str`).
- node_order compares two times of one form by position; two forms trap with "no implicit conversion between kinds
  of time".

## Next stages
- B1b: run-time forms of durations and zoned times.
- B2: arithmetic at run time (`d + 1 day` with variables).
- B3: zones at run time (offset_at with the Europe/US DST rules, resolve with disambiguation, zoned text).
- B4: remove the whole-program folder.
