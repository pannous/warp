# Task: `law` feature + Lean integration

Read Purpose.md, DESIGN.md (section "Progressive verification: `law`" and the implementation sequence) and CLAUDE.md first.

Goal: a function can carry laws, e.g.

    square(x) := x*x
    law square(-x) == square(x)

with progressive assurance levels that never require changing the source:
1. stated — parsed and attached to the function
2. asserted — checked at runtime in debug builds
3. tested — generated property tests (random inputs of the parameter types) run through the existing is!/round-trip machinery
4. proved — exported to Lean 4 (see ~/dev/script/lean4/hyper for conventions/toolchain), optionally an SMT solver; proof result recorded

Suggested order: parser/normalizer support for `law` → attach laws to functions in analyzer/function registry → property-test generation and runner → Lean export of pure integer/arithmetic functions plus laws as theorems, check via `lake`/`lean`, report proved/failed/unknown → CLI flag or test macro to run verification.

Keep it minimal and coherent with the planned src/semantic/ IR (laws later attach to semantic FunctionDecl; don't block on that IR existing).
New tests in tests/, experiments in probes/, guide in notes/laws.md.
Run ./test.sh before and after; never modify existing tests.
Small conventional commits, no AI attribution lines, push when tests pass.
Subagents allowed for sub-parts (e.g. Lean export vs. property testing).
