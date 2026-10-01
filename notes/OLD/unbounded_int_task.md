# Task: unbounded Int by default, cheap in the common case

Read Purpose.md, DESIGN.md ("Exact numbers by default", "Lowered IR and representation plan"), todo.md (Unbounded Int entry), notes/laws.md, CLAUDE.md first.

Semantics: Warp `Int` is a mathematical integer. `square(3037000500)` == 9223372037000250000, never a wrapped negative.
Representation is a compiler choice, never observable (except cost):

1. **Proven-small → plain i64, zero overhead.** Range analysis proves no overflow (literals, loop counters bounded by lengths, comparisons, masks, `%` by constant, results of `count`). Start simple; conservative is fine.
2. **Unknown → i64 fast path + overflow check → promote to BigInt on overflow.** Like Python/Lisp fixnum→bignum. WASM has no overflow flag: use sign-bit checks for add/sub; for mul check via i64 bounds or the wide-arithmetic proposal (i64.mul_wide_s) if wasmtime supports it. The fast path must stay unboxed; only the overflow branch allocates.
3. **BigInt representation** in WASM GC: e.g. a new Node kind / GC struct holding an (array i64) of limbs + sign, operations as runtime functions (tree-shaken when unused). Round trip back to Node (num-bigint is commented out in Cargo.toml / src/extensions/numbers.rs — enable it, vendor it).
4. **Explicit opt-out:** `@i64` / an `Int64` type with wrapping (or trapping — decide, document) semantics for code that wants machine ints.
5. After it lands: switch `law::lean` Int export back to Lean's unbounded `Int` for `Int`, keep `BitVec 64` for `Int64`; re-add the Mathlib fallback (~/dev/script/lean4/hyper) so `law square(x) >= 0` becomes Proved again. Mark the todo.md entry DONE (keep its text).

Measure cost: add a benchmark in probes/ (e.g. fib/loops/sum over lists) comparing wrapping i64 vs checked fast path vs forced bignum; record numbers in notes/unbounded_int.md. Target: checked path within ~10-30% of raw i64, proven-small path identical.

Tests: new tests in tests/ (overflow promotion for + - * and pow, demotion not required, round trip of big literals like 123456789012345678901234567890, comparisons across small/big). Never modify existing tests; if an existing test assumes wrapping, report it.

Coordination: other agents (effects warp-b2, deps warp-34, footguns warp-c6, law+Lean) work in this same checkout. Stage only your own files by explicit path, never git add -A / commit -a, git pull --rebase before push, never reset/stash others' work. Message the footguns agent to move integer overflow to Solved once verified.
Run ./test.sh before and after. Small conventional commits, no AI attribution, push when tests pass. Subagents allowed.
