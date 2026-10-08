# Effect handlers (card effect-handlers, user: "yes, steps 1+2")

Events are wasp's effects: `emit ask` performs the effect `ask`, an `on ask {…}` handler answers it. Handlers are
tail-resumptive only: the handler's value is what `emit` gives, and the emitting code goes on (P163). No captured
continuations, everything is plain calls.

## Step 1: block-scoped handlers
`on ask { 42 } in { compute() }` handles `ask` while the block runs, in it and in every function it calls; the block's
value is the expression's value. The innermost handler wins. An emit nobody handles does nothing (P163) and gives a
got-it warning "no handler for event X" (P202).
Program-wide `on ask {…}` (no `in`) works as before; it answers when no block handler is active.

Lowering (src/lowering/scoped_handlers.rs, before event_signals):
- each block handler i of event `ask` becomes `ask·handler·i(event) := { global …; body }`, as program handlers do
- the global `effect_handler_active_ask` (0 = none) is the active handler; `effect_handler_outer_ask_i` the one active
  when block i was entered; effects.rs ignores these plumbing globals, so a function using a block handler stays pure
- the block: `saved = ask·enter(i); value = block; ask·leave(saved); value`
- `emit ask{…}`: `if ask·active() == 1 then ask·handler·1(data) else … else emit ask{…}` (the last is the program-wide
  emit, lowered by event_signals as before); reads go through `ask·active()` so a function sees the current value
- while handler i runs, the active handler is block i's outer one: an emit inside a handler goes to the next handler out
- Limits: a handler body reads main-level variables and `event`, not the locals of the function it is written in;
  an error escaping the block leaves the handler active (no unwinding yet)

## Step 2: events as named effects (src/effects.rs)
- a function's effects gain event names next to the closed set (FunctionEffects.events): `emit ask` in f adds `ask`, so `effects of f` is `(IO ask)`
- a call inside `on ask {…} in {…}` does not pass `ask` on: the block handles it
- program-wide handlers do not discharge it: f still emits ask, main handles it
- `f := … ! Pure` reports an unhandled emitted event like any other effect; a declared `! ask` allows it
- Lean phase 4 (warp-94) models the same: effect rows of names, handled = removed

## Emit as an operand (card emit-operand)
- `x = 1 + emit ask`, `3 * emit ask + 1`: the parser (try_parse_emit) makes `emit <words>{data}` one operand, the
  same Space list a statement `emit ask` is; its words bind like a unary minus operand, so `+ 1` stays outside
- `emit` and `send` only; `emit = 3`, `emit(x)` and `emit + 1` keep `emit` a name (P165 soft keyword); a word operator after the event (`send alarm{} to "x"`) keeps the old
  shape, the operator's operands inside the phrase
- a program-wide handler's value is its last statement's (body statements spliced, not a one-item block `{42}`)

## Step 3: aborting handlers (card effect-handlers-abort, undoable default by warp-dc, user not asked yet)
- `break value` (or bare `break`, ø) at the top level of a block handler's body does not resume: the emit never
  returns, the `on … in {…}` block ends at once and its value is `value`:
  `on fail { break 0 } in { compute() }` is 0 when compute emits fail, however deep. No new word: `break` already
  leaves the innermost enclosing construct; a `break` inside a loop of the handler body still belongs to that loop.
  `return` keeps meaning the handler's value (resume), `raise`/`stop` stay errors (P163).
- Only block handlers abort; a program-wide `on fail {…}` has no block to unwind to (`break` there is an error as before).
- Mechanism: wasm exceptions, a tag `wasp_abort(i32)` of its own, so a user's `try … else` (tag `wasp_error`) never
  catches an abort. The handler stores the value in the global `effect_handler_aborted_<event>_<i>` and throws i;
  the block runs inside `ran_without_abort(i, {value = block})` (try_guard.rs), which catches i (another number is
  rethrown to the next block out), puts `try_depth` back to what it was at the block's start (tries the abort jumped
  out of never ended) and gives 0; then `if finished then value else aborted value`. The block leaves its handler on
  both paths.
- The effect row is unchanged: an aborting handler still handles (removes) its event.
- on error of f (Error.md) is the same shape (a handler that gives the call's value instead of resuming); it keeps
  its own lowering for now.
