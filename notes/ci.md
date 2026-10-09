# CI (Rust CI, ubuntu-latest)

- FFI signatures come from parsing system headers. glibc's math.h declares libm via
  `__MATHCALL` macros, so on Linux nothing parses → `use m` imported nothing.
  Fix: `LIBM_F64_FUNCTIONS` table in src/ffi/mod.rs is the fallback (same set the linker hardcodes).
- glibc's math.h includes bits/mathcalls.h from the multiarch dir /usr/include/x86_64-linux-gnu (card use-cmath,
  2026-10-09): without it in ffi_parser.rs INCLUDE_DIRS cbrt had no signature, the emitter's m.cbrt need was never
  provided and each rerun asked again, without end (a stack overflow even with 32 MB stacks). A rerun whose need is
  still missing now stops with a type error naming it (wasm_emitter/mod.rs inherited_needs). Reproduce on macOS with
  glibc headers from a libc6-dev .deb: WARP_INCLUDE=<deb>/usr/include[:<deb>/usr/include/x86_64-linux-gnu].
- Law proofs shell out to `lean`; CI installs it via elan (`LEAN_TOOLCHAIN` in rust.yml, keep in sync with local `lean --version`).
- `test_law_overflow_found_by_property_tests` fails by design since unbounded Int (fcbd300b).
  The Lean export still models Int as `BitVec 64`, so Lean still "finds" overflow that runtime no longer has.
