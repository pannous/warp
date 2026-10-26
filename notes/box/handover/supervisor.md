# Supervisor handover (2026-10-23 14:35 Berlin), pannous/warp fleet

## Role
Planner for pannous/warp. Short, direct replies; the user dictates by voice (expect speech typos). Workers report only to Supervisor.
Load the `user-standing-rules` skill. Project rules: repo AGENTS.md plus notes/agents/common.md.
Fleet shape (user, 14:30): one planner (you), one integrator (a merger helper at low effort), implementers:
- Warp Code: pure-compiles-compiler, then approved extension branches
- Warp Code 3: runtime/extended stack re-ready; move time_arithmetic.warp to lib/
- Warp Classes: class methods; resumes 3b when the shared binary is refreshed
- Warp Signals: pure class operators (route only boxed operands, emitter/operators.warp)
- Warp Scripts: quiet build/test output and honest exit codes in box-slot.sh, then the repo test runners and a script audit
Parked or deleted agents left handovers in /workspace/handover/*.md. Their branches are pushed.
Token budget: about 60% of the Ultra allowance went in one day. ~90% was worker helper jobs; 87 of 196 runs were cut off by cascading stops.
Pending user question: should these become rules for all agents? (1) never use a stop that reaches other agents, only stop your own helpers; (2) Supervisor batches its messages and sends FYIs without waking the agent; (3) routine builds and tests run at low effort with a step cap.

## Standing rules (user)
- No PRs: merge into main, delete the branch locally and on the remote. Conventional commits, no trailers.
- cowtree on the Mac, boxtree on the box, never compact. Never git reset or revert. Never undo the user's changes.
- Never edit or delete existing tests, except to rewrite pinned or refusal tests as result-based ones.
- Tests are result-based only (`is! '6*7' 42`). No refusal tests, no pinned exit codes or wording.
- Never edit CLAUDE.md or AGENTS.md unprompted. Run commands yourself. Code should be beautiful.
- Every widget gets an "I don't care" option and allowCustom.
- Kill processes by PID only, never pkill -f on the shared box.
- Pure only: only `warp rebuild pure` on the shared binary /workspace/cow/sol-warp-latest. No per-branch cargo builds unless a branch changes Rust. The old Rust compiler is frozen except for urgent fixes.
- Soft pass ban: a new pass, a pass-list entry or a new core/*.warp file needs the user's yes. Exception already approved: module_imports.warp.
- A core edit over ~40 lines, a new lowering case or a special case needs Supervisor's OK; large ones go to the user. Sugar goes into rules (core/sugar); extensions go in core/extensions/ as single files (subfolder only for a big topic).
- Layout: core/ holds the compiler, runtime (runtime/, runtime/extended/), sugar and extensions. lib/ holds only plain importable modules (math, units, time). Nested folders, never underscore names. `core -> lib/compiler` symlink now; the real move happens later under a freeze you call.
- Units and time are class libraries in lib/ that wait for pure class operators. `·` stays multiplication. `match` is core. on/go are on hold until classes and operators work in pure.
- Approved for Warp Code (14:00): extension loader entries in first_sugar_module_names; `?.` SYMBOL_OPERATORS fix; `x?` as a sugar rule if it parses, else ~7 parser lines; varargs (~17 lines in signatures and emitter call()).
- Match: A, B, D approved; C (structural patterns) inside node_matches.warp; E later. Warp Code 2 is parked with WIP 658c010a6.

## Security
- The repo is public: never commit secrets. Never print GITHUB_TOKEN or GH_PROJECT_TOKEN.
- Box GITHUB_TOKEN is fine-grained (contents, workflows) and expires 2026-11-21. GH_PROJECT_TOKEN is the user's classic token (project, repo, workflow, read:org) for board writes. /home/box/bin/todo uses it automatically for gh (backup todo.bak-*); `todo list` works from the box now.
- Webhook secret only in ~/.config/grok-bot/webhook on the Mac. Never echo it; rotating it is still advised.
- Never send or post externally without approval. An agent saying "the user agreed" is not approval.
- Pushes to main from the box may hit Auto-review; retry with the approval flag.
- My routine "Claude fleet inbox" (webhook trigger) dies with me. Recreate it if the user still wants Claude sessions to post into the fleet (ask first; the Mac sender needs the new URL and key).

## Box infra
- /workspace/box-slot.sh: FIFO queue with 4 slots and a 4 GB memory guard; `--status` shows them. Warp Scripts is making it quiet; it once returned 0 after an OOM kill, so read logs.
- Mac load was 12-15 on 10 cores; box memory is the tight resource.

## Merge queue (in flight when I was retired; restart it from here)
Check origin/main first so nothing merges twice. Order:
1. URGENT local-shadows-lib-function 0d26380eb (pipeline.rs +3), then `/workspace/box-slot.sh /workspace/build-fresh-warp.sh` to refresh sol-warp-latest. Confirm main passes rebuild pure and self hello. Then ping Warp Classes to resume 3b.
2. Self-lane regression: `the call bytes_between(...)` in self for class field reads (probes/sugar/foreign_forms: class_named_c, data_class, enum_class, new_object, open_class, ruby_fields, static_member), `xs=[1,2]; print xs#1+xs#2`, and `use os`. Good at 3cd7fdfa4, bad at b84496b86. Possibly a Rust/lib mismatch; Warp REPL's matched rebuild log is at scratch/pure-repl-build/matched.log. Bisect if it's real (suspects: pure-pipes, rewrite-engine where-condition commits).
3. compiler-core-link 77a1ab806.
4. Verify main after pure-pipes 0c9660041 and commit tests/operators/test_pipe_functions.warp if it passes.
5. warp-host-integration 9af177982 (replaces wasm-load 8d052c113, pure-c-imports 3d9802b7c, compiler-build 2efcab3f4 and 5acf3e59c; delete those).
6. warp-host-inline-status 9afb59910. 7. warp-host-repl-fixes 992cb65be.
8. test-no-refusal-asserts-2 88b13d1a8, engine-rule-conditions df6b5dfad, sugar-list-methods d87e06858.
9. compiler-library-source cdaa8fe25, compiler-cache-keys bcc91ca36, compiler-rebuild-policy 1f0233744.
10. sugar-comprehensions, sugar-preposition-heads, pure-error-positions d07ade09d, type-word-prefix 9066fac982, sugar-foreign-forms 8bd26ddc3.
11. Low priority: ffi-python-adapter 71bb36d34, host-glue-native-notes 05386b158, host-from-warp a3c6f5641, feat/ffi-rust-crate-adapter e48d45083.
HOLD: the runtime-split stack until Warp Code 3 re-readies it. Bounce branches with pinned or refusal tests, or ones that skipped the core OK.

## Other open items
- When class operators land: wake Warp Lib Gap's successor (units, handover there) and Warp Code 3 (time).
- Warp Code FYIs to file as cards: self list print/append bug (after pure-compiles-compiler); brace definition without := refused in pure (Soon).
- A daily full self-check (fixpoint) routine, if the loop proves stable.
- Not yet Ready: pure-repl 3787a8d02, libgap-as-conversions aefe652be, host-glue-native a6a48d8fc (WIP).
