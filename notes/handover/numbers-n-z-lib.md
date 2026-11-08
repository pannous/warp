# numbers-n-z-lib

Branch `numbers-n-z-lib` from origin/main `5ae124e4d`. Not pushed. The integrator lands it.

One lib change: `gcd`, `lowest_terms`, `euclidean_remainder` and `euclidean_quotient` in lib/math.warp take `number`, so a value past 64 bits is not cut to an int. `gcd(2^70, 3 * 2^65)` is `2^65`.

Measured with the v1.7.0 Linux release (`warp.wasm` of `f6fd3db2cc`, tag commit `47049867b`), `WARP_LIB` this tree. No compiler rebuild. Time and units files were not taken. `test/numbers/test_ratio_gcd_warp.warp:14` still fails here because this compiler has no `$t is Unit` condition; the twenty pairs match when written out. Mac should re-run that line.
