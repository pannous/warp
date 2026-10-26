# Warp REPL handover (card pure-repl), 2026-10-23 14:35 UTC+2
- Branch pure-repl @3787a8d02, pushed (merge of warp-host-wasm-load 8d052c113 onto my 302a79542). Card is in Now, NOT Ready (I moved it back while it was red).
- Green with the known-good 4e3808b76 compiler pair (/workspace/cow/compilers/warp-host-wasm-load): test_host 86/86, test_repl 17/17, test_repl_terminal 6/6 (logs: /workspace/cow/trees/pure-repl/scratch/pure-repl-build/good*.log).
- Contents: lib/repl.warp (replay of kept defs/assignments, multi-line, :help :session :reset, exit), libedit readline/history via FFI, main.rs: bare warp-host on a tty starts lib/repl.warp; tests/host/test_repl*.sh + terminal.py.
- Regression: pair rebuilt from 8d052c113's core with /workspace/cow/sol-warp-latest (Rust d21817966) gives "no field node_slice" (.slice/.copy/starts_with) and "outside the core: bytes_between" (use os): test_host 35/86. Supervisor thinks it's a mismatch (sol-warp-latest predates 0d26380eb).
- Matched-Rust check: Rust warp built at 3787a8d02 OK; `rebuild pure` was in flight at park (scratch/pure-repl-build/matched.log). Result not yet read; it's unconfirmed whether this is a main regression.
Next steps
1. Read matched.log: if the matched pair works, it was a mismatch; if not, bisect core (suspects d11e05a86, 3b70334bd, a861e6786, f4e278d95, b0995f812, 89807a321).
2. `todo ready pure-repl pure-repl` on the Mac (green with the known-good pair; the Integrator needs that pair or a fixed one).
3. Once warp-host-repl-fixes (e03f3bc4b) lands: readline -> owned_text, drop the setlocale-from-'c' workaround.
Open: soft imports + SIGINT host word (proposals on warp-host-repl-fixes); persistent instance (card warp-host-persistent-instance, Warp Wrapper). Bug cards filed: pure-if-try-while, pure-use-text-in-lib, pure-replace-error-value, pure-text-literal-from-line, pure-to-json-list, host-run-wasm-float. Repros: /workspace/cow/warp/data/pure-repl-bugs.
