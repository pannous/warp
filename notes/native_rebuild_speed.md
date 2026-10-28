# The native driver's GC: copying collector and initial heap (card wasmtime-native-rebuild)

core/native/driver.warp on brew's libwasmtime 50 rebuilds warp.wasm in 16+ min against ~2 min for the Rust harness
(crates/warp-runtime/src/engine.rs: wasmtime 49, copying collector, 512 MB initial GC heap). Investigation, 2026-10-28.

## What the C API offers
- brew's and the released `*-c-api` builds use the c-api crate's default features: gc, gc-drc, gc-null, **no
  gc-copying** (include/wasmtime/conf.h lists WASMTIME_FEATURE_GC_DRC and _GC_NULL only), so they run DRC.
- wasmtime 50's C API has no collector setter and no gc_heap_initial_size setter (crates/c-api/src/config.rs:
  only wasmtime_config_gc_support_set; memory_reservation is for linear memory). Nothing reads GC settings from a file
  or the environment (wasmtime_config_cache_config_load is the code cache only).
- The c-api crate does have a `gc-copying` feature, and `Collector::Auto` picks copying first whenever it is compiled
  in (wasmtime config.rs not_auto: copying, then drc, then null). A build with it switches the collector with no driver
  change; the initial heap still needs a setter.

## Shipped (card wasmtime-bundled)
crates/wasmtime-c-api builds libwarp_wasmtime (install name @rpath/libwarp_wasmtime.dylib, soname on Linux), not
`wasmtime`: brew's lib can never stand in silently. scripts/wasmtime-c-api.sh builds it into /opt/cargo/warp-wasmtime and
copies it into folders: after-push.sh beside the installed warp and into the main checkout's lib/, rebuild.sh into a
tree's lib/. `warp build native` links C libraries from warp's library folder first, with rpaths @executable_path
($ORIGIN) and that folder, so the native warp (whose library folder is its own) and built executables find it. The CI
matrix and the release tarballs (.github/workflows/wasmtime-c-api.yml) wait on branch wasmtime-c-api-ci.

## The build (option c)
A wrapper cdylib named `wasmtime` over wasmtime-c-api-impl 50.0.0 (data/native-rebuild-speed/capi, as wasmtime's own
crates/c-api/artifact does: `pub use wasmtime_c_api::*`), features wasi, cache, parallel-compilation, gc, gc-drc,
gc-copying, cranelift, wat, plus two setters: `warp_config_gc_heap_initial_size_set(config, bytes)` and
`warp_config_collector_set(config, 0 auto | 1 drc | 2 copying)`. wasm_config_t's Config field is pub(crate); the
setters cast through its `#[repr(C)]` single field (a patched c-api config.rs would be the clean form, or an upstream
PR adding both setters). Needs rustc >= 1.97 (wiggle-macro 50). ~6 min to build at nice 19, 18.6 MB dylib, 537 wasm_*
exports. The driver picks it up by DYLD_LIBRARY_PATH (brew's install name is absolute; dyld's leaf-name search still
prefers DYLD_LIBRARY_PATH); calling the new setters needs the driver linked against it (a `cc` that adds
`-L<its folder>` first, C_COMPILER is `cc -O2`).

## Measured (warm code cache, gpu_visualizer_warp.warp: warp.wasm compiling it, the run then traps the same in all)
| host | time |
|---|---|
| driver, brew DRC | 31.2 s |
| driver, copying (Auto), default initial heap | 8.0 s |
| driver, copying, 512 MB initial heap | 2.5 s |
| Rust harness (`warp`) | 2.6 s |
The machine code is cached per collector: a switch recompiles warp.wasm/the seed once (`not this wasmtime's machine
code, compiled anew: … compiled for the deferred reference-counting collector`), ~5 s.

## Open
The full `warp rebuild` by the native warp (core/native/warp.warp built on that driver, scratch lib/) raised a compile
error in the seed's compile after 178 s (load 61 on 10 cores); the driver prints no raised message, so its cause is
unknown, and an untested DRC run of the same takes 16+ min. Next: make the driver print a raised error's text, then
rerun at normal load.
