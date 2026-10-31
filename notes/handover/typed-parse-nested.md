# typed-parse-nested

The flat typed JSON validator on main refuses list and nested fields. The previous `typed-parse` branch contains an experimental recursive list checker, verified there for numeric lists and one-based mismatch paths. This card first isolates that list slice; nested classes remain to be implemented. Defaults: no coercion, one-based diagnostic paths, reject unsupported forms loudly. Existing tests were not modified.

Baseline: `WARP_NO_WINDOW=1 WARP_LIB=/Volumes/Worktrees/typed-parse-flat/lib /Users/me/dev/bin/warp test test/types/test_typed_parse_nested_lists.pairs.warp` failed 2/2 with `typed JSON supports only required flat fields yet`. Next adapt the prior recursive checker to the flat generator, smoke and run these outcomes.
