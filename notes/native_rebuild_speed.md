# The native driver's GC: copying collector and initial heap (card wasmtime-native-rebuild)

core/native/driver.warp on brew's libwasmtime 50 (DRC collector, default GC heap) rebuilds warp.wasm in 16+ min; with
the copying collector and a 512 MB initial GC heap a compile is about 12x faster.

## What the C API offers
- brew's and the released `*-c-api` builds have **no gc-copying** (they run DRC) and no setter for the collector or
  the initial GC heap size.
- The c-api crate does have a `gc-copying` feature, and `Collector::Auto` picks copying first whenever it is compiled
  in, so a build with it switches the collector with no driver change; the initial heap still needs a setter.

## Shipped (card wasmtime-bundled)
crates/wasmtime-capi builds libwarp_wasmtime (install name @rpath/libwarp_wasmtime.dylib, soname on Linux), not
`wasmtime`: brew's lib can never stand in silently. scripts/wasmtime-c-api.sh builds it into /opt/cargo/warp-wasmtime and
copies it into folders: after-push.sh beside the installed warp and into the main checkout's lib/, rebuild.sh into a
tree's lib/. `warp build native` links C libraries from warp's library folder first, with rpaths @executable_path
($ORIGIN) and that folder, so the native warp (whose library folder is its own) and built executables find it. The release
builds it per platform (.github/workflows/release.yml).

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

The machine code is cached per collector: a switch recompiles warp.wasm/the seed once (`not this wasmtime's machine
code, compiled anew: … compiled for the deferred reference-counting collector`), ~5 s.

## Open
The full `warp rebuild` by the native warp (core/native/warp.warp built on that driver, scratch lib/) raised a compile
error in the seed's compile after 178 s (load 61 on 10 cores); the driver prints no raised message, so its cause is
unknown, and an untested DRC run of the same takes 16+ min. Next: make the driver print a raised error's text, then
rerun at normal load.

## Upstream (prepared, waiting on the user's confirmation)
Two wasmtime PRs that would let crates/wasmtime-capi go, prepared in data/upstream/ (README.md: titles, bodies,
test results; submit.sh --confirm forks, pushes, opens them; dry run without). wasmtime main (52.0.0-dev) still lacks both.
- A `c-api-gc-config`: wasmtime_config_collector_set (wasmtime_collector_t AUTO/DRC/NULL/COPYING) and
  wasmtime_config_gc_heap_initial_size_set, C++ wrappers, and gc-copying in the C API's WASMTIME_FEATURE_LIST
  (on by default, so a default C API build's Collector::Auto is copying: the setter is then not even needed).
- B `gc-growth-usable-capacity`: GcHeap::usable_capacity (the active semi-space for copying) in should_collect_first
  and collect_and_maybe_grow_gc_heap: the general form of patches/copying-heap-growth.patch (same /4 for copying).
The Bytecode Alliance AI tool policy wants the submitter to have reviewed every line and ideally written the PR text.
Once both are in a release on Homebrew/APT: driver.warp's 59 `from "warp_wasmtime"` become "wasmtime" and
warp_config_gc_heap_initial_size_set becomes wasmtime_config_gc_heap_initial_size_set; scripts/wasmtime-c-api.sh
keeps only wgpu; crates/wasmtime-capi, the libwarp_wasmtime packaging (release.yml, native_dist.sh, nfpm, brew
formula, pypi wheels) and the after-push/rebuild/gate install steps go, replaced by a dependency on wasmtime >= that
release.
