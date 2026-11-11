# Binary gcd (Stein's algorithm)

`gcd` is unchanged. It is the core's Euclid (`core/integers.warp`), of any whole numbers, values past 64 bits
included, and `gcd_euclid` is that function's name. `gcd_binary` (lib/math.warp) is Stein's trailing-zero form
for a signed machine word: strip the common power of two, then subtract and shift. The idea is the one in the
Linux kernel's `lib/math/gcd.c` (https://github.com/torvalds/linux/blob/master/lib/math/gcd.c, GPL-2.0). That
file was the algorithm reference only. No Linux code was copied.

A value past a signed machine word is not counted with `trailing_zeros`. `gcd_binary` gives it to `gcd`, so
`gcd(2^70, 6)` and the other past-64-bit results stay exactly what they were. `gcd` itself was not replaced.

## Trailing zeros

`lib/bits.warp` already has `trailing_zeros(n: int) := wasm i64.ctz`. `gcd_binary_word` calls it. No new
compiler intrinsic, and no core change. A machine that had no such word would count the zeros with a shift
loop; this one has the word. The native bench cannot emit that wasm body (`primitive code calls only
primitive code`), so it calls libc `ffsll`, the same bit counted from one.

## Why Euclid stays `gcd`

The native bench is `test/bench/binary_gcd.warp`, an `@primitive` file built with `warp build native` (wasm2c
and `cc -O2`, not the wasm runner). `clock()` is C's, and the figure is that count divided by 1000
(microseconds to milliseconds). Three runs, the median. Binary is slower on every class, so it stays opt-in
and `gcd` stays the core's Euclid.

| class | what is timed | Euclid | binary |
|---|---|---|---|
| small | 2000000 × 8 pairs under 1000 | 106 ms | 235 ms |
| random | 80000 × 32 values across a signed word | 100 ms | 277 ms |
| powers | 2000000 × 8 power-of-two-heavy pairs | 78 ms | 143 ms |
| large | 1500000 × 8 values near the top of a signed word | 100 ms | 453 ms |

The three runs were 106 106 98 vs 235 240 232 (small), 100 104 99 vs 277 281 273 (random), 78 82 76 vs
143 150 143 (powers), 100 103 96 vs 453 457 446 (large). Each class printed `1` first: the two algorithms
agreed on every pair.
