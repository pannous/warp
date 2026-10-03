# Task: update all Rust dependencies

State (2026-09-26): Cargo.lock last changed 2026-01-18; `cargo update --dry-run` shows 71 compatible updates; libloading 0.8.9 → 0.9.0 and likely more semver-major bumps (check with `cargo outdated`; wasmtime/wasmer/wasmedge/wasm-encoder/wasmparser are the important ones).
The project builds offline from vendor/ (.cargo/config.toml has offline = true) — updates need online resolution, then re-vendor (`cargo vendor`) so offline builds keep working.

Steps:
1. ./test.sh baseline, note results.
2. Phase 1: `cargo update` (semver-compatible), re-vendor, build + ./test.sh, commit Cargo.lock + vendor/ only.
3. Phase 2: semver-major bumps one crate (or tightly coupled family) at a time in Cargo.toml, fix breakage, test, commit each separately. NEVER downgrade anything.
4. Record difficulties and results in notes/dependencies.md.

## Coordination — two other agents work in the SAME checkout (law + Lean: notes/law_lean_task.md, session warp-d3; effects: notes/effects_task.md)
- Phase 1 first and push it quickly; it is low risk for the others.
- Before a semver-major bump that requires code changes outside Cargo files, message the other sessions (names via ListAgents) so they aren't surprised by build breaks.
- Stage only your own files by explicit path; never `git add -A` / `commit -a`; don't commit test_results.txt unless your change caused the diff; `git pull --rebase` before push; never reset, stash others' work, or cargo clean unless absolutely necessary.
- Distinguish failures caused by your bump from failures caused by the others' in-progress edits (compare against a clean `git stash`-free check: e.g. build a scratch clone of HEAD in probes/ if needed).
Small conventional commits (chore:), no AI attribution lines, push when tests pass.
