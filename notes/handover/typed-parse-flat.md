# typed-parse flat slice

Branch `typed-parse-flat` isolates required flat class fields from the unfinished `typed-parse` branch, which retains the experiments and all original tests. The installed compiler refuses `parse("{id:7,name:Ada}", User)` with `the call parse(…) (no owner)`. The slice implements no coercion, rejects unknown or missing fields, and is intended to refuse constructor, optional, defaulted, nested and other non-flat shapes. Those guards and the copied outcomes are not yet verified. `scripts/smoke.sh /Volumes/Worktrees/typed-parse` is building the separate constructor guard; after it finishes, merge origin/main here, run `scripts/smoke.sh /Volumes/Worktrees/typed-parse-flat`, then run the focused test/types/test_typed_parse_* files present in this worktree. Fix any failures before landing. Follow-up cards: typed-parse-nested, typed-parse-optional, typed-parse-generic, typed-parse-constructor. No existing tests modified.

`warp format yes` left the two implementation files unchanged but replaced the required tab in each new `.pairs.warp` test with a space. Restored the authored tab-separated test content; card format-pairs tracks the formatter issue. Do not format pair files until that is fixed.

The original branch's constructor guard passed its focused outcome and smoke. The flat slice smoke is running in `data/typed-parse-flat/smoke.log`; a computed-source outcome was added while it builds, without changing compiler inputs.

Early feedback against the preserved branch compiler (not the flat slice build): 11 outcomes in the library, bool, falsy, root, computed-source and flat-diagnostics files passed via `WARP_NO_WINDOW=1 WARP_LIB=/Volumes/Worktrees/typed-parse/lib /Users/me/dev/bin/warp test` with the six absolute file paths. The flat slice and its new field-shape guards still require their own build and tests.

Flat slice smoke passed on compiler hash 4bcf9f67268231bb (`Hello 🌍2026`); all 14 outcomes in the eight `test_typed_parse_{library,bool,falsy,root,constructor_guard,flat_guards,flat_diagnostics,computed_source}.pairs.warp` files passed with this worktree's compiler. Next remove unreachable experimental list/default generation from the isolated slice, rebuild and repeat these outcomes before landing; the original branch retains that work.

Removed unreachable optional/default and recursive-list generator code from the flat slice; the original branch retains those experiments. This cleanup is unverified until a fresh merged-main smoke and the same 14 outcomes pass.
