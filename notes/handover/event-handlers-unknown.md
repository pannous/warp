# Handover: event handlers (card event-handlers-unknown), parked 2026-10-27

User decision (via warp-interviewer): not landed. Handlers wait for block-parameters-captures; then the shape is
discussed again, likely `on(event, body: block)` as a plain prelude function taking a sharing block, no handler code in
core. System values (key_pressed, click_x… and the whole system_value_kinds table) leave the emitter for lib
declarations (card whole-table).

Reference: a4d0dde771 (branch event-handlers-unknown, merged with main as 56f4eec299, green on smoke, self-check and
gate tier 1). What it has, reusable for the new shape:
- core/extensions/handlers.warp: `on frame/key/click {…}` as the exported on·every·N started by signal_listen(1, N, 16);
  key and click pumps; `emit e{data}`; key word texts (ESC, UP, F1…); `held_key == UP` through key_held.
- lib/prelude.warp key_name / key_code (KeyboardEvent.key names <-> host codes, arrows 0xF700+).
- core/passes/main_globals.warp made_global: the main-level names a handler function assigns are declared `global` in it
  (decision handler-blocks-share: blocks share, `def f` still needs `global x`).
- core/passes/rewrites.warp: a parenthesized glued head `(on··e)(x) := …` (head_part), numbers kept as words beside `··`,
  statement-place results not made values, prelude words a template names reached (reaching_template_words).
Results: data/t1 key/frame tests 23/29 (main 4), event tests 25/74 (main 2). Open there: emit without handler,
`on every 10 ms`, fire/send, listeners of.
