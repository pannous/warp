# Dates at run time (card runtime-dates)

`now` reads the host clock when the program runs, not when it compiles (decision 7e9b01db4: dates run at run time).

## Representation
An instant is a `$Node` of `Kind::Time` (18): `data` = `$i64box` of nanoseconds since 1970 UTC, `value` = null.
The analyzer types it `Kind::Data`. Readers: `wasm_reader::struct_node` and `web.rs cell_node` both go through
`time::instant_node`, which gives `Node::Data(Time::Instant)`; it prints like the compile-time instant.

## Lowering
- `system_values::name` rewrites a free `now` (also the target of a dot, `now.hour`) into `instant_at(clock())`
  (`time::now_call`). A program that binds `now` keeps its own.
- `clock()` is the existing host word (ms since 1970): native `host_words::milliseconds_since_epoch`, browser
  `host.js clock: () => BigInt(Date.now())`. No new import; precision is milliseconds.
- `time::answer` (the compile-time folder) still runs, but only for programs with forms it alone knows: time literals,
  durations, `date(…)`, `t in "Zone"` with a known zone (`needs_folding`). There `instant_at(…)` folds to the
  compile-time `now()`.

## Emitter (src/wasm_emitter/times.rs)
- `instant_at(i64 ms) -> node`
- `instant_text(node) -> text`: RFC 3339 like calendar.rs `Display` (seconds left out when zero, fraction trimmed),
  Hinnant's civil_from_days in wasm; list_join calls it (print, `+` with a text, `str`).
- node_order compares two instants by nanoseconds (`emit_instant_order`); equality already worked (shared i64box).
- `t.hour` of an instant fails with `calendar::no_wall_clock` through `returned_error` + trap detail.

## Next stages
Durations and date arithmetic at run time, zones (`now in "Europe/Berlin"` needs tz rules at run time), dates and
local times as Kind::Time info bits, a sub-millisecond clock host word.
