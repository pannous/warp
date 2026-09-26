# Task: effect inference, `effects of f`, effects → WASM imports

Read Purpose.md, DESIGN.md (sections "Effects", "Effects as enforced capabilities", "The compiler is a query interface") and CLAUDE.md first.

Goal:
1. Closed effect set: Pure, State, Allocation, IO, FFI, Async, Unsafe (`EffectSet` bitset).
2. Host/WASI/FFI declarations carry trusted effect signatures; a function's effects are the union over its body's calls (fixpoint over recursion).
3. Query: `effects of f` (source-level) and a Rust API returning the resolved EffectSet.
4. Optional declared constraints (`f(x) := … ! Pure`) are checked; violation is a diagnostic with a span and the call chain causing it.
5. Replace source-substring feature detection (src/wasm_emitter/mod.rs ~l.2915: `code.contains("fetch ")`, `"puts "`, `"import "`) by imports derived from resolved calls/effects. A module with no IO effect must emit no WASI IO import.

Keep it coherent with the planned src/semantic/ IR (EffectSet later lives on semantic Expr/FunctionDecl); don't block on that IR existing.
Tests in tests/ (e.g. tests/test_effects.rs), experiments in probes/, guide in notes/effects.md.

## Coordination — another agent works in the SAME checkout on `law` + Lean (notes/law_lean_task.md)
- Put your logic in new files (src/effects.rs); keep edits to shared files (wasp_parser.rs, analyzer.rs, function.rs, wasm_emitter/mod.rs) small and localized.
- Never `git add -A` / `git add .` / `git commit -a`: stage only your own files/hunks by explicit path (`git add -p` is not available; stage whole files you alone touched, or coordinate).
- Don't commit test_results.txt unless your change caused the diff; never revert or reformat changes you didn't make.
- `git pull --rebase` before pushing. Never reset, stash others' work, or cargo clean.
- If a build/test break is caused by the other agent's in-progress edit, message it (session name via ListAgents, the law/Lean one is `warp-d3`) instead of fixing it yourself.

Run ./test.sh before and after; never modify existing tests. Small conventional commits, no AI attribution lines, push when tests pass. Subagents allowed.
