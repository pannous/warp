# numbers-n-z-lib

Branch `numbers-n-z-lib`. Not pushed.

`gcd(a: int, b: int): int` is the signature. A call is `wide_gcd` before an int parameter can wrap a value past 64 bits, and `lowest_terms` calls `wide_gcd` too. `euclidean_remainder` and `euclidean_quotient` rewrite to `%` and `/!` at the call, so a literal and a for-loop value each keep the operator written there.
