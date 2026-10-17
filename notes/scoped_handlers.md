# Block-scoped handlers: dropped

`on e {h} in {block}` answered `emit e` only while the block ran, also deep in the functions it called; `break v` in
the handler ended the block with v. Dropped (cleanup cuts): the form
is an error naming the program-wide `on e {…}` (src/lowering/control/scoped_handlers.rs keeps only that check).

Gone with it:
- the lowering (handler functions `e·handler·i`, the globals `effect_handler_active_e`, enter/leave, unwinding of
  `try` around active handlers)
- the abort machinery in try_guard.rs: tag `warp_abort`, `ran_without_abort(i, {…})`, `abort_to(i)`; only
  `warp_error` (`try … else`, `catch e`) is left
- effects.rs: the unreported plumbing State (`reported` effects) and handled events; a call's events always pass on
- the type model's translation of `on e {h} in {…}` and `break` (src/law); the 15 CORPUS programs of the form
- tests: cleanup-edge ignores in tests/control (test_scoped_handlers, test_aborting_handlers,
  test_handler_unwinding, test_emit_operands, test_handler_globals, test_unhandled_emits) and tests/functions
  (test_effect_block_operand, test_event_effects)

Kept: the Lean model (lean/WarpTypes Syntax/Semantics/Soundness `handle`, `abort`) states the rules and proves them;
the type model still uses `.handle`/`.abort` for an early `return`. Card under type-theory (Later): drop or keep the
block-handler rules in Lean.
