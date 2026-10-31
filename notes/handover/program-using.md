# program-using

Reproduced the card on installed main and on this worktree: `WARP_NO_WINDOW=1 WARP_LIB=/Volumes/Worktrees/program-using/lib /Users/me/dev/bin/warp test test/types/test_units_free_function.pairs.warp` fails with `unknown name pattern·clause·shown` in `quantity_text`. The new outcome expects the program's `shown(5)` to produce text `5`; units-as-values is the source. No existing test changed. Next investigate a general library solution preserving the public `Quantity.shown()` method and free function; do not silently rename either API. Smoke on the unmodified library passed.
