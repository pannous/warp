# What starting the compiler costs (card version-retires)

`warp samples/hello/hello.warp` is a compile and a run behind one startup: the native warp loads wasmtime, warp.wasm's
machine code and starts the compiler before any compile. `warp version` goes the same way (it runs the compiler's own
`version` command), so it is the startup alone. scripts/vitals.py measures both per install, and the Vitals page shows
the difference as its own line: **the compile alone**, hello's instructions minus `warp version`'s (about 1.42 G of
hello's 1.56 G). It still holds hello's own run, a few M.

The startup, measured by a C harness linking the same libwarp_wasmtime.dylib, each step stopped after
(data/version-retires/stages.c; instructions retired by `/usr/bin/time -l`, the cheapest of five runs):

| step | M instructions | ours? |
|--|--|--|
| process start, dyld (an empty C program; `/bin/echo` 9 M) | 11.4 | the OS floor |
| libwarp_wasmtime.dylib (17 MB) loaded and initialized: linked at launch, the only dylib beside libSystem | 14.4 | yes |
| engine: config, engine, cache config | 1.6 | yes |
| warp.wasm (2.5 MB) read whole and fingerprinted (FNV, byte by byte) to check its machine code's key | 14.7 | yes |
| warp.wasm.driver.cwasm (38.8 MB) deserialized | 38 | yes |
| instantiated | 1 | yes |
| the compiler's `_start` to its version line: its global initializers | 39 | yes |
| the native's rest: its own read and FNV (slower than the harness's), then compiler_stamp reading warp.wasm whole again for its 120-byte build stamp at the end | ~20 | yes |
| `warp version` in all | 141 | |

So about 130 M of the 141 M is ours. Fixable on their own:
- the second whole read for the stamp (compiler_stamp): the stamp is the file's last bytes, one read of the tail
- the fingerprint, a read and a byte loop over 2.5 MB each start: a key of the file's size and time would cost nothing
- the compiler's `_start` at 39 M: what its global initializers build before any command
- the eager dylib: loaded on first use, a command that runs no module skips 14 M

Inside the compile (`compile_written` → `compile`), hello's 1.42 G: see compile_speed.md.
