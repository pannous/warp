# Warp Code (grok-warp-code) handover, 2026-10-23 ~15:00 CEST

All work is committed and pushed, and no helper jobs are running. Git and the board are the status.

## Ready (express), waiting for the integrator
| Card / branch | Tip | Notes |
|---|---|---|
| pure-compiles-compiler | e3c9dd0a4 | 3 one-line fixes: emitter/host.warp runtime_global accepts an empty list (`is_empty_list`); emitter/lists.warp length_code handles a char; emitter.warp declared_kinds gains char. Tests: tests/text/test_pure_case_words.warp 5/5, test_pure_char_words.warp 4/4. Full compile with ALL fixes not rerun yet: `/workspace/box-slot.sh /workspace/pcc/full.sh` (compiler in /workspace/pcc/h). If it finishes, try the output as compiler-self, then the fixpoint. Notes: /workspace/cow/scratch/pcc-notes.md. Error file:line is unreliable (runtime code has no positions). |
| pure-tuples-destructuring | 1639a7084a | lib/compiler/extensions/tuples.warp, plus the loader list `extension_module_names` (rewrites.warp, in front of first_sugar_module_names) and the 2-line tuple_fits fix. 9/9. |
| pure-safe-navigation | 82c93dee7e | STACKED on tuples (land after it). 2-line parser fix (`?.` in SYMBOL_OPERATORS, ternary guard) + extensions/safe_navigation.warp. 6/6. |
| pure-varargs | e53683e25f | ~30 lines (the approval was for ~17): signatures, emitter/functions, emitter/runtime_words (2), analyzer.warp (7; fixes a wasm trap). 9/9. Open: calls without parentheses, spreading into fixed params, spreads in list literals. |
| pure-optional-params | 757e07508e | 7-line parser case: `x?` parses as one name and a rule can't split a name. 4/4. |
| engine-rule-conditions | df6b5dfad | condition-order fix, queued |
| sugar-list-methods | d87e06858 | after df6b5dfad; deletes lib/compiler/list_methods.warp |
| sugar-comprehensions | 1fe8d4ae0 | re-readied after a bounce |
| test-no-refusal-asserts | 0a025d88e | |
| test-no-refusal-asserts-2 | 88b13d1a8 | ignored .rs tests on the gap cards |

After each lands: delete the branch (local and remote) and the worktree (/Volumes/Worktrees/<name>, /workspace/cow/trees/<name>). The local branch gwc-proto can be deleted.

## Not done / parked
- **gap-element-array** (Now, NOT fixed), branch tip 58aa46837 (rule in lib/sugar/lists.warp, probes, tests/lists/test_element_arrays.warp 0/2; parser back to main). The rule never fires: `x is a 100 element array` parses as one phrase, and the parser lets `a` take a number but not the hole `$n`. Both parser attempts (letting holes act like names in `applies`) made every build hang. Next: find which sugar rule loops when holes join phrases, or match `$x is a $n …` in rewrites.warp (needs Supervisor's OK). Notes: /workspace/cow/scratch/gea-notes.md. The Mac worktree has stale uncommitted edits; the box tree is current.
- **self-list-print** (Now, untouched): in self, `xs=[1,2]; print xs#1+xs#2` fails with "the call bytes_between", and `print xs` with "unknown name short_text". Supervisor wanted it next after pure-compiles-compiler.
- **gap-brace-def-no-assign** (Soon): `g(n) { return [n, n*2] }` is refused in pure.
- **sugar-definitions** 0f0cb389d / **sugar-declarations** 53fd74273: wait for engine-guarded-call-phrase + sugar-type-tests on main, then re-merge, self-check, re-ready.
- **engine-right-to-left-runs** 1873b3df7 (Later): parked by the user along with the clause-style definitions (fib(0) := 0 …); it's only needed for the clauses migration. The user's decision is pending.
- **Gap cards in Soon:** gap-falsy-if-values, gap-if-call-form, gap-if-colon-after-call, gap-fib-brace-body, gap-fib-implicit-it, gap-max-one-number, gap-span-units-compare, gap-long-double-ffi (Warp FFI).

## Open questions for Supervisor
1. Optional params went in as the parser case, not a rule (a rule can't split a name).
2. Extensions run before all first-sugar rules; moving them after the sugar is a one-line swap.
3. Varargs grew past the approved size (see above).
4. In pure, a char result prints as "a" rather than 'a'.

## Testing notes
- `warp test` with WARP_SELF_HOSTED=pure hands `is!` blocks to Rust. To really test pure, use tests/queue.sh with WARP_SELF_HOSTED=pure-only, or run each test as its own program.
- Pure rebuild: `WARP_LIB=<tree>/lib /workspace/box-slot.sh /workspace/cow/sol-warp-latest rebuild pure`, or the Mac slot. A full self-compile takes ~330 s and 2 GB, and must run alone.
