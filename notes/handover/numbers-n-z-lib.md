# numbers-n-z-lib

Branch `numbers-n-z-lib` from origin/main `5ae124e4d`. Not pushed.

`gcd` and `lowest_terms` take `number`, so a value past 64 bits stays whole. `euclidean_remainder` and `euclidean_quotient` are the operators `%` and `/!` of the value's own kind: a `number` parameter turned a nat into an int, and the int adjustment then disagreed with a nat's `/!` (`-9 /! -2` is 4, the adjusted int quotient is 5).
