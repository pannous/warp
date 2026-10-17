# No compiler panics

Rule: a program a user can write never panics the compiler. `panic!`/`unwrap()`/`expect()`/`unreachable!()` reachable
from user programs become error values (`emit_type_error`, `emit_malformed`, `emit_undefined_variable`,
`WasmGcEmitter::type_error`, `TypeManager::type_errors`); tests in tests/sweeps/test_panic_sweep.rs. A WASM validation
failure of the emitted module is `try_finish` → `Node::Error("internal error: WASM validation failed …")`; `finish`
(test helper API) still panics.

## Kept (genuine internal invariants)
| site | why |
|------|-----|
| `func_index` "Unknown function", `emit_call` assert on required functions | names are compiler constants; a miss is a compiler bug the two-pass `discovered_needs` rerun otherwise repairs |
| `register_user_function_signature`, `compile_user_function_body` `.get(name).unwrap()`, `func_index.unwrap()` | `name` comes from the list of registered user functions, the index is set in pass 1 |
| `let … else { unreachable!() }` in `get_type` | the match guard just proved the shape |
| `unreachable!` in `emit_int_op`, `emit_machine_int_op`, `emit_int_compare`, `emit_float_comparison` | callers only pass arithmetic / comparison operators (guarded by `is_arithmetic`, `is_comparison`, `is_shift`) |
| `i64::try_from(number).expect("big literal needs the Int runtime")` | `analyze_required_functions` requires the Int runtime for every non-fixnum literal |
| `decimal_fraction` `expect`s | parse the output of Rust's own `{:e}` formatting of an f64 |
| analyzer `.expect("guarded")` and `call sites were collected from known functions` | the `match` arm guard calls the same helper that must return `Some` |
| `warp_parser` `panic!("Invalid bracket")` | `parse_bracketed` is only called with the literals `( [ { <` |
| `warp_parser` `body_items…next().unwrap()` / `precedences.last().unwrap()` | preceded by a `len() == 1` / `is_empty()` check |
| `Number::to_bigint` / `to_rational` panics | every caller matches Int/BigInt (and Quotient/BigQuotient) first; the arithmetic arms check `is_rational` |
| wisp_parser `panic!("expected …")` | all inside its `#[cfg(test)]` module: test assertions |
