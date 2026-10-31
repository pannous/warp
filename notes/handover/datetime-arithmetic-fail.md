# datetime-arithmetic-fail

Baseline on main: `WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'use time; use units; now + 1h'` reports `index out of range`. `warp test test/types/test_duration_units.pairs.warp` fails the two time-moving cases and an unrelated `min` lookup; the two other cases pass. Existing tests are unchanged. Added deterministic instant addition and subtraction outcomes. Next check how Instant and the runtime time literals receive quantities; preserve milliseconds for the existing numeric API and reject non-time dimensions.
