# CI (Rust CI, ubuntu-latest)

- FFI signatures come from parsing system headers. glibc's math.h declares libm via
  `__MATHCALL` macros, so on Linux nothing parses → `use m` imported nothing.
  Fix: `LIBM_F64_FUNCTIONS` table in src/ffi.rs is the fallback (same set the linker hardcodes).
- Law proofs shell out to `lean`; CI installs it via elan (`LEAN_TOOLCHAIN` in rust.yml, keep in sync with local `lean --version`).
- `test_law_overflow_found_by_property_tests` fails by design since unbounded Int (fcbd300b).
  The Lean export still models Int as `BitVec 64`, so Lean still "finds" overflow that runtime no longer has.
