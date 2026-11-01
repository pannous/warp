# library-typed

Default: type one module at a time without narrowing its existing accepted inputs. Start with `lib/hash.warp`; SHA-256 returns text and CRC-32 returns an integer. Remaining library modules need the same audit.

Before changes, `WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'use hash; [sha256("abc"), crc32("abc")]'` returned `["ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", 891568578]`. Stored baseline for the existing hash tests is green. A fresh worktree initially has no `lib/warp.wasm`; run the prescribed smoke setup before worktree-specific tests.

`lib/hash.warp` now declares `sha256` returns `text` and `crc32` returns `int`; both retain `text or list` inputs. Direct real text and byte-list calls returned the same three expected results before and after the change. The new outcome test passed 3/3 before the signatures. Next: finish the focused test run, format, merge main and smoke; land this module as a slice, then audit remaining modules. No existing test was modified.

Focused post-change run: `WARP_NO_WINDOW=1 WARP_LIB=/Volumes/Worktrees/library-typed/lib /Users/me/dev/bin/warp test test/modules/test_hash_typed.pairs.warp test/modules/test_std_hash.warp test/modules/test_hash_bytes.pairs.warp` passed 9/9. `scripts/smoke.sh /Volumes/Worktrees/library-typed` passed. Formatting `lib/hash.warp` changed nothing; formatting the pairs file replaced its required tabs with spaces, so those formatter changes were restored. Next: audit another module, starting with `lib/random.warp`; preserve generic element types rather than narrowing them without evidence.
