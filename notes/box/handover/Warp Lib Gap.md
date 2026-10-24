# Warp Lib Gap handover (2026-10-23 14:31 CEST, parked)
- pure-as-conversions (Now, mine): branch libgap-as-conversions tip aefe652be, pushed, NOT Ready.
  Last full measure on 5af404bea: /tmp/lg/act 503/1058 fail vs 522 base = +19, 0 regressions; last passing selfcheck on 5b710a86b.
  Since 8411245e3 every class-conversion program traps "call stack exhausted" (c1-c5,c7,c9,c13,c14, to_text probe).
  Next: run /tmp/lg/ac/run.sh on c1,c9 against /tmp/lg/x1/lib (param "it") and /tmp/lg/x2/lib (no renamed call); fold the winner
  into definitions.warp, full measure + rebuild pure + self hello, then `todo ready pure-as-conversions libgap-as-conversions --express`.
  Fallback: forward-fix back to the 5b710a86b behaviour and Ready the +19.
  Still refused: `x as text = ...`, `ys = ... as list of ...`, `xs: list of int`, as_number int/float, `as symbol`.
- pure-unit-literals (Soon): waits for pure-class-operators (Warp Signals); user decided Quantity class in lib/units + literal rule only.
  libgap-units-2 (50addcb2a) = reference only, does not land. Local tree libgap-as-units = scratch, unpushed.
- rust-strings-list-panic (Soon): bisect e40f04a50 vs 669d65d84 (showing_words map -> list).
- Ready earlier (express): lines-clash, def-heads, bench-emitter-apart, system-values, wasi-environment, is-pair,
  property-accessors, field-calls, exit-miscompile, nil-in-any-map. Log: /workspace/cow/libgap/inventory.md.
- Open: folder lib/units vs lib/extensions/units.warp (Supervisor confirming with user).
