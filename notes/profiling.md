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
