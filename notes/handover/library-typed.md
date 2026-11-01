# library-typed

Default: type one module at a time without narrowing its existing accepted inputs. Start with `lib/hash.warp`; SHA-256 returns text and CRC-32 returns an integer. Remaining library modules need the same audit.

Before changes, `WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'use hash; [sha256("abc"), crc32("abc")]'` returned `["ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", 891568578]`. Stored baseline for the existing hash tests is green. A fresh worktree initially has no `lib/warp.wasm`; run the prescribed smoke setup before worktree-specific tests.
