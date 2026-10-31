# typed-parse-nested

The flat typed JSON validator on main refuses list and nested fields. The previous `typed-parse` branch contains an experimental recursive list checker, verified there for numeric lists and one-based mismatch paths. This card first isolates that list slice; nested classes remain to be implemented. Defaults: no coercion, one-based diagnostic paths, reject unsupported forms loudly. Existing tests were not modified.

Baseline: `WARP_NO_WINDOW=1 WARP_LIB=/Volumes/Worktrees/typed-parse-flat/lib /Users/me/dev/bin/warp test test/types/test_typed_parse_nested_lists.pairs.warp` failed 2/2 with `typed JSON supports only required flat fields yet`. Next adapt the prior recursive checker to the flat generator, smoke and run these outcomes.

Adapted the old recursive list checker to the flat validator and retained a recursive whitelist of supported primitive element types. This is unverified; nested class support is still outstanding. Restored the prior cleanup plan after inadvertently replacing it in the first commit.

Smoke was started on commit a9fd2bd69b with `WARP_NO_WINDOW=1 scripts/smoke.sh /Volumes/Worktrees/typed-parse-nested`; build store hash is `a58d1abf47233363`, log `/Volumes/Build/compilers/a58d1abf47233363/warp.building/log`. It was still running when recorded. Do not start a duplicate build: inspect the store result first. On success run `WARP_NO_WINDOW=1 WARP_LIB=/Volumes/Worktrees/typed-parse-nested/lib /Users/me/dev/bin/warp test test/types/test_typed_parse_nested_lists.pairs.warp` and the existing flat typed-parse outcome files. On failure read the retained log and fix forward. Neither list implementation nor nested classes are verified yet.
