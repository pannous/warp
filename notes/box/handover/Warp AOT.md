# Warp AOT handover (card warp-host-aot), 2026-10-23
- Branch warp-host-aot (tip b91a719e0): merged into origin/main; branch and box worktree already removed.
  Commits: 7c2c6cc5f feat (compile --aot, .cwasm run, build), eecb6b981 test (tests/host/test_host_aot.sh), b91a719e0 docs.
- No open branches, nothing uncommitted, no subagents running.
Next steps:
- Align with pure-c-imports compiler lookup (beside binary, lib/../data, <prefix>/share/warp): check aot.rs/main.rs
  compile --aot and build don't hardcode data/; built apps carry no compiler, so compile_wasm inside one needs that lookup.
- Warp FFI merge 836b8e9a4: .cwasm call sites pass no c_calls letters, so precompiled modules link by import types; verify FFI letters case.
- test_host_aot.sh may pin exit codes / error wording; rewrite to result-based asserts per new rule.
- Remove Mac worktree /Volumes/Worktrees/warp-host-aot and /Volumes/Build/warp-host-test-target if unused.
Open questions: should .cwasm/built apps store c_calls letters too (needs Warp FFI input)?
