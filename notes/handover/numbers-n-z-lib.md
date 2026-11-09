# numbers-n-z-lib

Branch `numbers-n-z-lib`. Not pushed. Not done.

`gcd(a: int, b: int): int` is the Euclid steps. Arguments past 64 bits are unsupported, blocked on the compiler because an int parameter is 64 bits. `wide_gcd` is the same steps for a caller that needs the wider value, and `lowest_terms` calls it. `euclidean_remainder` and `euclidean_quotient` rewrite to `%` and `/!` at the call.
