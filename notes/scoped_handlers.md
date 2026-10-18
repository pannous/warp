# Block-scoped handlers: dropped

`on e {h} in {block}` answered `emit e` only while the block ran, also deep in the functions it called; `break v` in
the handler ended the block with v. Dropped (cleanup cuts); the check that named the program-wide `on e {…}` went
too (decision rust-cleanup-drops), so the form now reads as any other `{…} in {…}`.

Only `warp_error` (`try … else`, `catch e`) is left of the abort machinery; a call's events always pass on. Its tests
are cleanup-edge ignores in tests/control (test_scoped_handlers, test_aborting_handlers, test_handler_unwinding,
test_emit_operands, test_handler_globals, test_unhandled_emits) and tests/functions (test_effect_block_operand,
test_event_effects).

Kept: the Lean model (lean/WarpTypes Syntax/Semantics/Soundness `handle`, `abort`) states the rules and proves them;
the type model still uses `.handle`/`.abort` for an early `return`. Card under type-theory (Later): drop or keep the
block-handler rules in Lean.
