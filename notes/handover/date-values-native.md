# date-values-native, instance-copy: blocked on core

Written 2026-11-02 by a cloud worker (lib/-only rules). Nothing of either card can land in lib/ alone: every piece
that fails in samples/numbers/dates.warp and samples/objects/references.warp sits in core/. The pieces, each with
its smallest core shape, for the Interviewer:

## date-values-native (samples/numbers/dates.warp)

1. Dates do not sort: `sort([2024-03-01, 2024-01-01])` → "no order between data … numbers order with numbers".
   core/runtime/ordering.warp node_order has no Kind.Time branch; `<`/`>` work (times.warp time_holds).
   Shape: in node_order, two times order by time_holds (relation 0, then equality), ~3 lines. Its error word
   "data" for a time comes from the kind word, show.warp/types.warp: a time should be named "time".
2. A zoned literal is "a data": `2024-07-01T10:30[Europe/Berlin]` → "outside the core: a data".
   core/emitter/times.warp moment_parts returns [] for `[Zone]`, and runtime time_text traps on TimeFormCode.Zoned
   (time_form_not_in_warp). The zone rules exist in plain warp now (lib/time.warp time_zones, zone_offset,
   zone_instant), so the shape is: the emitter makes a zoned time node (wall position, zone index into
   lib/time's time_zones, offset from zone_offset at compile time), time_text prints `…+02:00[Europe/Berlin]`,
   time_field_of takes the zone's seconds. Already-written red tests: test/types/test_time_values_at_run_time*,
   test_time_zones_at_run_time*, test_time_kinds_are_types:11, test_time_type_names:9.
3. `moment in "Europe/Berlin"` is read as containment and gives no; it should be the zoned time (and
   `zoned(t, zone, disambiguation: …)` is unknown). Needs (2) first, then `in` of a time and a text routed to a
   lib word (a typed infix operator `in(t: time, zone: text)` once the seed reads it).
4. `start.hour` of `now`: now is lib/time's Instant class (ms), which has no wall-clock fields. Adding them to
   Instant in lib (offset by TZ through zone_named, time_field_of on the wall clock: tried, works in every zone)
   breaks time literals: a class method one class declares becomes the global `hour(self)`
   (core/passes/class_methods.warp), so `"at " + meeting.hour` on a time literal calls Instant's method whenever
   lib/time is in scope (the sample uses `now`, so it always is). The same bug already hits
   `"w " + d.weekday` (lib/time's weekday(ms) function wins over the time's own field) — trap at
   lib/time.warp day_number. Shape: a value statically a time reads the time fields (emitter times.warp
   time_fields) as fields, before any function or method of that name (class_methods' routing by static type
   already does this for runtime words: add the time field names to that set). Then the Instant fields land
   in lib/ as tried. Also `local_offset`, the core host word for an instant's wall clock, is answered by no
   host (core/native/driver.warp).

## instance-copy (samples/objects/references.warp)

1. `copy(mine)` resolves to lib/memory's copy(bytes, at) by the auto-use hint ("copy is memory's") and fails
   "not a list". lib/prelude.warp declares `instance_copy(x, shallow): same as x from warp`, but no runtime word
   implements it and core/emitter/runtime_words.warp maps no `copy` call to it. Plain warp cannot copy an
   instance: it lives in a struct of its class (maps.warp instance_set), and no generic constructor is
   reachable from lib. Shape: runtime instance_copy (struct of the same class, each field copied deep unless
   shallow; lists by node_copy, maps entry by entry), and the prelude word
   `copy(x, shallow: bool = no) := instance_copy(x, shallow)` over memory's.
2. `mine === ours` → "outside the core: the operator === on int": === of two instances (identity, ref.eq of the
   structs) is missing in core/emitter/operators.warp.

## How to test
Smoke test only after the core change; the sample runs are the outcome tests (plus the red test/types files above).
