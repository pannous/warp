# Multi-value results (branch multi-value, 2026-10-03)

User functions return the general Node, so multi-value pays off only inside the runtime helpers, where a helper
naturally produces two numbers. Tests: tests/test_multi_value.rs (result counts read from the name section, results of
big division and exact division, an optimized module through wasm-opt).

## Survey: where two values travel today
| place | before | now |
|---|---|---|
| `mag_divmod` (big_int.rs) | quotient as result, remainder in the global `int_remainder_global` (comment: "wasm-opt runs without multivalue") | returns `(quotient, remainder)`; the global is gone |
| `big_quot` / `big_rem` | two functions, each one long division, reading the global | one `big_divmod(a, b) -> (q, r)` |
| `int_quot_slow` / `int_rem_slow` | one long division each | thin wrappers over `int_divmod_slow(a, b) -> (q, r)` that drop one result |
| `exact_div` on big integers | cross products, then `ratio_new` → `int_gcd` (an Euclid loop of big divisions) and two quotients, even when b divides a | one `int_divmod_slow`; zero remainder → the quotient, else the old ratio path |
| host imports `fetch`, `fetch_within`, `read` | already `(ptr, len)` as two i32 results | unchanged |
| `wasm_optimizer.rs` BINARYEN_FEATURES | no multivalue | `--enable-multivalue` |

Left as is, and why:
- `int_heap_global` / `int_count_global`, the text heap and list heap globals, `try_guard`'s depth/caught globals:
  allocator and exception state, not a smuggled second result.
- big_int.rs scratch locals (`scratch(0..2)`): operands of the inline fast path, not results.
- `exact_numerator` / `exact_denominator`: called separately, often on different values (`cross_product` takes a's
  numerator and b's denominator). An `exact_parts(x) -> (n, d)` would save one `is_ratio` + heap lookup in
  `exact_is_nan`, `exact_infinity_rank`, `exact_neg`, `exact_pow`; small win, not done.
- smart pointers (src/smarty.rs, `WaspSmartPointers`): not used by the emitter; wiki/multi-value.md already calls them
  obsolete.
- Language level (`return a, b`, `x, y = f()`): no decided syntax in wiki/ or notes/open_decisions.md, and user
  functions return a Node; open question for the supervisor, nothing invented.

## Measurements
Program (probes/multi_value/bench.rs, copy to examples/ to run it): digit sum of 1000!
`fac(n) := n<2 ? 1 : n*fac(n-1); f = fac(1000); s = 0; while f > 0 { s += f % 10; f = f//10 }; s` → 10539.
Release build, 9 alternating rounds of wasmtime compile + run (`wasm_reader::read_bytes`), machine under heavy load:

| module | bytes | bytes after wasm-opt -O3 | min | median |
|---|---|---|---|---|
| before (origin/main a01e06ec) | 7719 | 5036 | 891 ms | 953 ms |
| mag_divmod multi-value only | 7716 | 5053 | 892 ms | 941 ms |
| + big_divmod, exact_div via divmod | 7799 | – | 664 ms | 700 ms |

- Replacing the global by a second result alone is neutral (a global.set/get against the stack, lost in noise) and
  3 bytes smaller; it mainly removes a global and lets wasm-opt see the data flow.
- The win comes from what a two-result division enables: `f//10` is `(f - f%10)/10`, and the exact division of a big
  integer used to run a gcd loop of big divisions before ratio_new; now it is one divmod. About -26 % on this program.
- Next step (not done): `a//b` still costs two long divisions (`%`, then `/`), because the parser rewrites it to
  `(a - a%b)/b` (wasp_parser.rs `floor_division`). A Euclidean `exact_floor_div` built on `int_divmod_slow` would make
  it one; it needs the rewrite to become an operator of its own.
