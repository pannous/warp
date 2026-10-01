# CI failures (Rust CI, main)

## Fixed 2026-09-12

- `test_struct_types::test_debug_format` — the global `FieldNameRegistry` in
  `src/gc_traits.rs` matched GC struct types *structurally*. `Person` and
  `Person4` (from `test_magic_object_mismatch`) have identical field layouts,
  so whichever test registered its module last won the name lookup — a race
  under parallel test execution. `GcObject` now carries the id of the module it
  came from (`register_gc_types_from_wasm` returns it) and resolves names
  against that module only.
- `cargo clippy -- -D warnings` had 11 errors and never got to run because the
  test step failed first. All fixed; the lib is clippy-clean.

## Open

- `test_web::test_fetch` — **server-side, not a code bug.**
  `https://pannous.com/files/test` returns **404**; the test expects the body
  `test 2 5 3 7`. Restore that file on pannous.com and the test goes green.
  `download()` now prints the HTTP error instead of returning "" silently.
- `wasm_optimizer_test::{test_library_optimization, test_executable_tree_shaking}`
  fail under `--all-features` only (pre-existing; CI uses default features).
