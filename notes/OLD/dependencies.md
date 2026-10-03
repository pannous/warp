# Dependencies

Status 2026-09-26: `cargo outdated -R` → "All dependencies are up to date".

## Update log (2026-09-26)
| commit | change | code changes |
|---|---|---|
| 27b84095 | `cargo update`: 71 semver-compatible bumps | none |
| 5452da66 | rustyline 15→18, libloading 0.8→0.9 | none |
| 3cdadf6c | wat 1.243→1.259, wasmparser/wasm-encoder 0.243→0.259 | `gc_traits.rs`: `into_types_and_offsets()` now yields `u64` offsets → use `into_types()` |
| ae9f8911 | wasmtime/wasmtime-wasi 40→49 | enable `anyhow` feature; tail returns `Ok(x?)` |
| 3d819144 | syn 2→3 (dev-dep, `tests/test_asts.rs`) | none |

Tests after every step (clean HEAD worktree): 463 passed, 1 failed (`test_fetch`, needs network, pre-existing).

## Gotchas
- **wasm-tools must be pinned explicitly** (`wasmparser = "0.259"` etc.), not `"*"`:
  with `"*"` cargo unifies them with the version wasmtime pins internally, so they never move.
  Two wasmparser versions in the lock (ours + wasmtime's 0.258) is fine — we don't pass wasmparser types into wasmtime.
- **wasmtime ≥ 4x: `wasmtime::Error` is its own type** (`wasmtime-internal-core`), no longer `anyhow::Error`.
  With `default-features = false` you must add the `anyhow` feature, else hundreds of `?` errors.
  Even with it, a *tail expression* returning `wasmtime::Result` inside an `anyhow::Result` fn/closure
  doesn't convert → write `Ok(expr?)`.
- `vendor/` is gitignored and NOT wired as a cargo source; offline builds use `~/.cargo/registry`.
  Update with `cargo update --config net.offline=false`, then `cargo vendor --config net.offline=false vendor`.
- Build breaks during a bump are easy to confuse with other agents' in-progress edits in the shared checkout:
  verify in a detached worktree of HEAD + your patch (`git worktree add --detach probes/deps_check HEAD`).
