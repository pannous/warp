# Binary gcd (Stein's algorithm)

`gcd` is Euclid's algorithm, `gcd_euclid`. `gcd_binary` is Stein's binary gcd in the trailing-zero form: strip the
common power of two, then subtract and shift until the odd parts meet. The idea is the one in the Linux kernel's
`lib/math/gcd.c` (https://github.com/torvalds/linux/blob/master/lib/math/gcd.c, GPL-2.0). That file was the
algorithm reference only. No Linux code was copied. Both functions live in `core/integers.warp` (`lib/integers.warp`
links there), which is the one gcd of the compiler and of every program (`use math` or `use integers`).

## Why the wide path divides

A count of trailing zeros for a signed machine word is `wasm i64.ctz` (`gcd_word_zeros`), the same instruction
`lib/bits.warp`'s `trailing_zeros` already uses. No new compiler intrinsic.

Past a signed machine word the bitwise operators are not the number's bits, and a shift is not a power of two.
`test/results.txt` records it: `2^70 | 1` is 1, `(2^100) >> 98` is 0. Fixing that is a compiler change, and this
work does not make one. `gcd_binary` therefore keeps the machine-word path on shifts, and for a whole number of
`2^63` or more it halves by exact division (`/!`), 32 bits at a time and then one bit. The results match Euclid,
including `gcd(0, 0)`, negatives, and `2^70`.

## Why Euclid stays the default

Measured by `test/bench/binary_gcd.warp` on the callable functions (`clock()` around the calls only, three passes,
the median). `gcd_binary` is slower on every class, so Euclid stays `gcd` and `gcd_binary` stays opt-in.

| class | what is timed | Euclid | binary |
|---|---|---|---|
| small | 20000 × 8 pairs under 1000 | 73 ms | 122 ms |
| random | 400 × 32 values below 2^63 | 40 ms | 97 ms |
| powers | 15000 × 8 power-of-two-heavy pairs | 32 ms | 42 ms |
| wide | 200 × 8 pairs past 2^63 | 13 ms | 189 ms |

The three passes, on the compiler built with this file, were 74 73 73 vs 128 122 120 (small), 40 40 40 vs 97 96 97
(random), 33 32 32 vs 42 42 42 (powers), 13 14 13 vs 189 190 189 (wide). Wide is the slow class because each
"shift" is an exact division. Re-run `warp test/bench/binary_gcd.warp` to read them again.
