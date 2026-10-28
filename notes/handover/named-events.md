# Handover: named events, part A of the `on` group (card event-handlers-unknown), on hold 2026-10-28

On hold by user decision (merge-then-reshuffle-then-celebrate): resume after lib-core-reshuffle and the warp-from-warp tag.
Branch event-handlers-unknown-2, WIP, not on main:
- core/rewrites.warp: sugar_module_names() is every lib/sugar/*.warp not placed in another step (supervisor's ask)
- lib/sugar/events.warp: on/once/emit/fire/send/trigger, listeners of, remove … from listeners of
- lib/prelude.warp: registry listen/emitted/listeners_of/unlisten, listeners numbered within their event
State (merged with the reshuffled main): smoke green; working: on/emit with data and results, once, fire/trigger,
`listeners of e`, `count listeners of e`, `remove n from listeners of e`. The registry keeps three parallel lists
(calling a function taken out of a list entry, `entry#2(data)`, trapped). The rules sit in the early sugar step
(early_sugar_module_names): `once` is also a prelude function and `of`/`remove … from` are read by passes before the
default sugar step.
Open: `listeners of e -= h` (parses as `listeners of (-= e h)`; the rule `listeners of ($e -= $h)` doesn't fire yet),
two-word events (`emit nobody listens`, `on stop the machine`), hoisting (`emit ping; on ping {…}` expects 1), a
copy of the event per listener. Next after landing: list-sugar-methods, the leading-minus text cut, signals shape (B).
