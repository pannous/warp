# Panic sweep (task F)

Rule: a program a user can write never panics the compiler. `panic!`/`unwrap()`/`expect()`/`unreachable!()` in
src/wasm_emitter, src/analyzer.rs and src/warp_parser.rs became error values (`emit_type_error`, `emit_malformed`,
`emit_undefined_variable`, `WasmGcEmitter::type_error`, `TypeManager::type_errors`), tests in tests/sweeps/test_panic_sweep.rs.

## Converted
| site | now |
|------|-----|
| `x = …`, `x++`, `x += …` with a non-variable target (5 copies) | `expected a variable to …, got …` at the source position |
| float compound assignment `x %= 2`, `x ^= 2` (was a silent `f64.mul` fallback) | the float operator; `and=`/`or=` on floats → error |
| ternary / if-then / while without their structure (7 sites, 2 copies of the if-then destructuring → `if_then_parts`) | `expected …, got …` |
| prefix `∛` in an exact context ("Unhandled prefix operator") | `… of an exact number is not supported yet` |
| range with a non-constant bound | `expected a constant integer as range bound` |
| user function called with a missing argument that has no default | `f needs a value for parameter b` |
| user function returning a float used as an exact Int | float-in-exact-context error |
| unknown / not yet compiled user function | `undefined function: f` |
| `global 3`, `global a.b = 1` | `expected global name …` |
| struct field of an unknown type | `unknown type: Vec3 of field x` |
| WASM validation failure of the emitted module (`finish`) | `try_finish` → `Node::Error("internal error: WASM validation failed …")`; `finish` (test helper API) still panics |
| `test.wasm` debug dump failing (read-only cwd) | logged, ignored |
| `new_list` missing (inline list) | internal error value |

## Converted (panics-remaining, 2026-10-06)
| site | now |
|------|-----|
| `Number` `+ - * /` of unsupported pairs (4× `panic!("unsupported types")`), `Quotient` with a NaN/∞ Float (`to_rational` panic) | `mixed`: complex if either operand is, an exact real stays exact with a ratio (else `Real::Approx`), ∞/NaN by IEEE |
| `f64::from(Complex)` (`unimplemented!`) | the real part without an imaginary one, else NaN |
| wisp_parser: a missing `)`/`]`, unclosed text, a number that is none (`(int x)` was 0, a big int was 0) | `Error("wisp: expected ')', got the end")`, …; big ints parse exactly |

## Kept (genuine internal invariants)
| site | why |
|------|-----|
| `func_index` "Unknown function", `emit_call` assert on required functions | names are compiler constants; a miss is a compiler bug the two-pass `discovered_needs` rerun otherwise repairs |
| `register_user_function_signature`, `compile_user_function_body` `.get(name).unwrap()`, `func_index.unwrap()` | `name` comes from the list of registered user functions, the index is set in pass 1 |
| `let … else { unreachable!() }` in `get_type` | the match guard just proved the shape |
| `unreachable!` in `emit_int_op`, `emit_machine_int_op`, `emit_int_compare`, `emit_float_comparison` | callers only pass arithmetic / comparison operators (guarded by `is_arithmetic`, `is_comparison`, `is_shift`) |
| `i64::try_from(number).expect("big literal needs the Int runtime")` | `analyze_required_functions` requires the Int runtime for every non-fixnum literal |
| `decimal_fraction` `expect`s | parse the output of Rust's own `{:e}` formatting of an f64 |
| analyzer `.expect("guarded")` (7×) and `call sites were collected from known functions` | the `match` arm guard calls the same helper that must return `Some` |
| `warp_parser` `panic!("Invalid bracket")` | `parse_bracketed` is only called with the literals `( [ { <` |
| `warp_parser` `body_items…next().unwrap()` / `precedences.last().unwrap()` | preceded by a `len() == 1` / `is_empty()` check |
| `node_emitter::EmitContext::emit_call` | the type is not used by any emitter path |
| `Number::to_bigint` / `to_rational` panics | every caller matches Int/BigInt (and Quotient/BigQuotient) first; the arithmetic arms check `is_rational` |
| wisp_parser `panic!("expected …")` (13) | all inside its `#[cfg(test)]` module: test assertions |
