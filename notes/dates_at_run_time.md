# Dates at run time

`now` reads the host clock when the program runs, not when it compiles (decision runtime-dates). Date, local time,
instant and zoned time literals and durations are run-time values too.

## Representation
A time is a `$Node` of `Kind::Time` (18): `data` = `$i64box` of its position, `value` = null. Its `TimeForm`
(time.rs) sits in the info bits above the kind (`kind = Time | form << 8`):
- Instant = 0: nanoseconds since 1970 UTC
- Date = 1: days since 1970
- Local = 2: wall-clock nanoseconds as if UTC
- Duration = 3: its exact nanoseconds; days (32 bits) above the form, months above them (`MONTHS_SHIFT`)
- Zoned = 4: nanoseconds since 1970 UTC; the index in `calendar::ZONES` (8 bits) above the form, the offset in minutes
  above it (`OFFSET_SHIFT`). Read back with the current rules; a prediction (after the builtin rules' publication)
  prints `[_tzdata=warp-2026a]`
- Clock = 5: a time of day (`calendar::Time::Clock`, type `time of day`), nanoseconds since midnight. Made by the
  parts `t.time` (whole seconds, prints `22:27:46`) and `t.date` (a Date) of any time with a clock or date
  (`Time::part`; at run time `time_field` indices 10 and 11 wrapped into a time node). `time.now` lowers to `now.time`
  (time/partial_comparisons.rs, user decision time-word). `+`/`-` an exact duration goes round the clock
  (23:00 + 2 hours = 01:00); months or days are refused. Not yet: a time of day compared with a duration (`now.time <
  17h`) and a clock literal (`22:27`).

`time::time_info` gives the info bits and position of a constant. The analyzer types a time `Kind::Data`. Readers
(`wasm_reader::struct_node`, `web.rs cell_node`) pass all info bits (`kind >> 8`) to `time::time_node(info, position)`.

## Lowering (`time::lower`, called by pipeline.rs)
- A free `now` (also `now.hour`) becomes `instant_at(clock())` (`time::now_call`, system_values).
- No program is answered whole at compile time: `fold_constants` folds the largest constant time
  expressions (time literals, unit and time words, field names after a dot) into `time_of(info, position)`, and the
  rest runs. A real error of a constant expression is the program's error.
- A duration in a unit of the units module (`1 day in h`) folds to its quantity, written as `24h` for the units'
  lowering; `(t in "Zone")` in parentheses is the placement's call itself, so its fields read.
- `t.tzdata` of a zoned time is the version of the rules of the compilation (`emit_tzdata_lookup`).
- Known edge: a variable named like a unit word (`days`) counts as a time word.

## The local wall clock of an instant
An instant's calendar fields (`now.hour`, `t.year`, …) are the wall clock of the environment's time zone, provided through the runtime; `epoch_seconds` needs none.
- Host word `local_offset(ms) -> seconds east of UTC` at that instant, DST included: natively libc `localtime_r`'s
  `tm_gmtoff` (TZ, else the system zone; UTC without one, `system_signals::local_offset_at`), in the browser
  `-Date.getTimezoneOffset() * 60` (host.js).
- `extract_host_words` imports it for a program that makes an instant (`time::makes_instant`); `time_field` adds the
  offset before splitting the position.
- The compile-time evaluator leaves instant fields to the run (the run's zone, not the compiler's); calendar.rs
  `Time::field` uses `time::local_offset` for the same result natively.
- Placing in a zone stays explicit for any other zone: `t in "Europe/Berlin"`.

## Emitter (src/wasm_emitter/time/times.rs)
- `instant_at(i64 ms) -> node`; `time_of` constants are emitted inline (`emit_time_constant`).
- `civil_from_days`, `days_from_civil`: Hinnant's civil calendar. `time_parts(form, position) -> (days, nanos)`.
- `time_field(node, index of TIME_FIELDS) -> i64`, a zoned time's on its own wall clock, `offset` its seconds east of
  UTC. The call site (`emit_time_field_lookup`) first refuses the fields a form lacks, with the message of
  `TimeForm::refusal(name)` (`date has no hour`, `duration has no field year`).
- `time_text(node) -> text`: RFC 3339 like calendar.rs `Display` (seconds left out when zero, fraction trimmed, `Z`
  only on an instant, a zoned time's offset and `[Zone]`); list_join calls it (print, `+` with a text, `str`).
  `duration_text` shows a duration as `Display for Duration` (`1 month 2 days 3 hours`, `-1.5 seconds`). The zone
  names enter linear memory only when a zoned constant requires `ZONE_NAMES` (analyzer/imports.rs).
- node_order compares two times of one form by position; two forms trap with "no implicit conversion between kinds
  of time", durations with "cannot order durations".
- warp twins (card duration-warp): lib/compiler/runtime.warp `## durations` has `duration_text(n)` and
  `duration_order(a, b)`, each unpacking the node (months above bit 43 of the kind word, days the signed 32 bits
  above bit 11, nanoseconds in the box) into `duration_parts_text` / `duration_parts_order` over the three amounts,
  checked against calendar.rs by tests/numbers/test_duration_words.rs. emitter.warp compiles no time yet, so the
  Rust functions stay until it does; then the call sites call the warp words, traps.rs maps
  `durations_compare_differently_on_different_dates` to the "cannot order durations" message, and the Rust goes.
  lib/time.warp `format_duration(ms)` is a different, compact form (`1h 30m 30s`) of plain milliseconds, kept apart.

## Arithmetic (wasm_emitter/time_arithmetic.rs)
node_add, node_sub and node_mul hand over to `time_arithmetic(a, b, operator)` when an operand is a time, as calendar.rs
does for constants: time ± duration (months first, then days, then nanoseconds; a day that does not exist in the target
month is a run-time error naming `add(…, overflow: clamp)`), duration ± duration, duration * whole number (either way
round), `a - b` of two times of one form (days between dates, else a duration). A date takes no time of day, an instant
no months or days. `1 month 2 days` is lexed as one duration
(literals.rs `with_time_unit`).

## Zones (wasm_emitter/time_zones.rs)
- `zone_rule(zone)` (winter offset, Dst) is a table of calendar::ZONES; `zone_offset_at(zone, seconds)` applies the Europe
  and US rules as `Zone::offset_at`; `zone_resolve(zone, wall nanos, choice) -> (instant, status)` as `Zone::resolve`
  (status 1 skipped, 2 repeated, both failing with the choice named unless `disambiguation: earlier|later`);
  `zoned_node(zone, instant)` builds the node with zone and offset in the info bits.
- The zone functions exist when a program makes a zoned time (ZONE_NAMES required, by a zoned constant or by
  `time_in_zone`).
- `t in "Zone"` and `zoned(t, "Zone", disambiguation: later)` of a time only the run knows lower (time.rs
  `zone_placement`, in `fold_constants`) to `time_in_zone(t, zone index, choice)`; the zone must be a literal name.
  An instant or zoned time keeps its instant, a local time is resolved, a date is refused.
- Zoned `+`/`-` a duration: exact units move the instant, months and days move the wall time which is resolved again
  (reject: a day landing on a skipped or repeated wall time fails). `a - b` of two zoned times is their elapsed time.
- Not yet: `add(t, d, disambiguation: …)` and a zone name held in a variable at run time.

