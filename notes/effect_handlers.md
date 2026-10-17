# Effect handlers (card effect-handlers, user: "yes, steps 1+2")

Roadmap card: effects (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

Events are warp's effects: `emit ask` performs the effect `ask`, an `on ask {…}` handler answers it. Handlers are
tail-resumptive only: the handler's value is what `emit` gives, and the emitting code goes on (P163). No captured
continuations, everything is plain calls.

## Step 1: block-scoped handlers (dropped)
`on ask { 42 } in { compute() }` was dropped (user, cleanup-cuts; card cleanup-drop-scoped-handlers): the form is an
error naming the program-wide `on ask {…}`, which answers every emit. An emit nobody handles does nothing (P163) and
gives a got-it warning "no handler for event X" (P202). Summary and history: notes/scoped_handlers.md.

## Step 2: events as named effects (src/analyzer/effects/mod.rs)
- a function's effects gain event names next to the closed set (FunctionEffects.events): `emit ask` in f adds `ask`, so `effects of f` is `(IO ask)`
- program-wide handlers do not discharge it: f still emits ask, main handles it
- `f := … ! Pure` reports an unhandled emitted event like any other effect; a declared `! ask` allows it
- Lean phase 4 (warp-94) models effect rows of names, handled = removed; its block handlers (`.handle`, `.abort`) stay
  as the model of an early `return` (src/law/type_model/expressions.rs)

## Emit as an operand (card emit-operand)
- `x = 1 + emit ask`, `3 * emit ask + 1`: the parser (try_parse_emit) makes `emit <words>{data}` one operand, the
  same Space list a statement `emit ask` is; its words bind like a unary minus operand, so `+ 1` stays outside
- `emit` and `send` only; `emit = 3`, `emit(x)` and `emit + 1` keep `emit` a name (P165 soft keyword); a word operator after the event (`send alarm{} to "x"`) keeps the old
  shape, the operator's operands inside the phrase
- a program-wide handler's value is its last statement's (body statements spliced, not a one-item block `{42}`)

## Step 3: aborting handlers (dropped with step 1)
`break value` in a block handler ended its block; it went with block handlers, and with it the tag `warp_abort` and
`ran_without_abort` / `abort_to` (try_guard.rs). `on error of f` (Error.md) keeps its own lowering.

Why block handlers were dropped and what they did: notes/scoped_handlers.md
