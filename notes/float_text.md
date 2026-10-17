# Float text at run time

Floats print their shortest digits that read back (Python/JS repr), in pure wasm: src/wasm_emitter/numbers/float_text.rs.

- float_digits(x): estimate the exponent, scale x to 17 digits with float_scaled (double-double: Dekker's exact
  product, one exact power ≤ 10^22 a step, rescaled by 2^∓128 beyond 2^±900), and the two halfways to the neighbours
  (inclusive when x's last bit is even). The multiple of the largest power of ten between them, nearest x (half to
  even, decided on the exact double-double, not on rounded 17 digits: double rounding broke 0.6% of floats), is the
  shortest. probes/floats/float_scaled_sim.py runs the same steps in Python: 0 differences from repr on 1M random bit
  patterns and 300k decimal-looking floats.
- float_text is emitted only when a program may show a float: analyzer::makes_floats (literals, `/`, roots, powers,
  float words, libm, foreign/host values) plus text_builtins FLOAT_MAKERS (text_as_float …); a `new_float` the emitter
  writes anyway without float_text reruns emission with it (WasmGcEmitter::emit_call). The hello-world site has no
  float_text (web::test_bundle_budget).
