# Profiling a warp run (card warp-profile)

`WARP_PROFILE=<file> warp <program>` samples where the wasm spends its time and writes Firefox Profiler JSON when the
run ends; open it at https://profiler.firefox.com (Load a profile from file). The function names come from the
module's name section.

- Sampling: wasmtime's GuestProfiler at each epoch tick (10 ms, `warp_runtime::engine::EPOCH_TICK`), so samples fall
  on function entries and loop headers; host calls show as markers. Code: crates/warp-runtime/src/profile.rs, the store
  of every `run_main` (src/run/wasm_reader.rs) through `profiled_store`.
- Several runs in one process (the self-hosted stage programs, `WARP_SELF_HOSTED=…`): the first writes `<file>`, the
  next `<stem>-2.json`, `<stem>-3.json`, … beside it; stderr names each file written.
- Runs counting fuel (`use fuel`, WARP_FUEL) don't tick and give no samples.
- Feature `profiling` of warp-runtime: warp has it (native), the standalone stubs don't (a build without it says so
  on stderr when WARP_PROFILE is set).

## Where a compile goes (snake, warm caches, 2026-10-28, clock probes in passes.warp; card runtime-prepended)
`warp pure build samples/advanced/snake.warp` ~1.1 s: checked_program ~0.8 s, emitter 0.23 s (0.11 s prepended behind
runtime.wasm), analyzer 0.03 s, parse 0.05 s. Inside checked_program:
- with_used_modules ~235 ms: the used library modules compiled from source (card modules-prebuilt)
- lowered_node ~240 ms, of it the five sugar-rule passes ~145 ms (first_sugar 38, sugar 44, late 23, iteration 21,
  comprehension 17; card sugar-rule-passes)
- with_expected_type_uses ~130 ms: the prelude and every bundled module sliced into definitions each compile (card takes-ms)
Every compile is prepended behind runtime.wasm (runtime-lowering-scope, 2026-10-28): it lowers and compiles no
runtime word, so the kept-runtime-words miss (4.5-7 s) is gone with the cache; it costs reading runtime.wasm (~40 ms)
and ~0.05 s more machine code: hello 0.22 s vs 0.17 s before, snake 1.04 s vs 1.14 s.
