# Handover: named events, part A of the `on` group (card event-handlers-unknown), on hold 2026-10-28

On hold by user decision (merge-then-reshuffle-then-celebrate): resume after lib-core-reshuffle and the warp-from-warp tag.
Branch event-handlers-unknown-2, WIP, not on main:
- core/rewrites.warp: sugar_module_names() is every lib/sugar/*.warp not placed in another step (supervisor's ask)
- lib/sugar/events.warp: on/once/emit/fire/send/trigger, listeners of, remove … from listeners of
- lib/prelude.warp: registry listen/emitted/listeners_of/unlisten, listeners numbered within their event
State: smoke green, but the events don't run yet:
- `on add {…}; emit add{…}` traps at prelude 7:25 (`entry#2(data)`: calling a function held in a list; the user-level
  prototype data/events/proto3.warp in the main checkout, a map of name → list of functions, works)
- `once alarm {…}` "fits no single variant": the `yes` argument vs `once: bool`
- `listeners of alarm`: the rule doesn't fire (phrase `listeners of $e`)
- `emit nobody listens`: two-word events (also `on stop the machine`) unhandled
Shape decided with the supervisor: pure lib (sugar + prelude registry); variable signals (part B) need a
"collect then rewrite" hook outside core, shape still to be reworked; system events (part C) later.
