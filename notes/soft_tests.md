# Tests in a program (card test-soft, P209, P210)

```warp
square(x) := x * x
test square(3) == 9            // soft: records pass/fail, goes on
test "squares" {               // named: a failing check or any error fails only this block
  check square(4) == 16        // check stays assert
}
```

- `warp test file.warp`: the tests run; each failure prints `✗ test square(3) == 10` or `✗ "squares": assertion failed: …`
  when it happens; the last line is `✓ 3 tests passed` (exit 0) or `2 of 3 failed` (exit 1).
- `warp run` / `warp file.warp` / `use`: the tests are skipped (P209).
- The playground has no `warp test`: its Run runs a program's tests (src/web.rs run_shown), so the kitchen sink shows
  `✓ 33 tests passed`.
- How: src/lowering/test_blocks.rs, a SOURCE_PASSES pass, rewrites top-level tests (and `test` lines inside test
  blocks) into counting code over hidden names `tests·run`, `tests·failed`; pipeline::for_tests sets the mode. The ✗
  line shows the statement as written (diagnostic::written_statement).
- A program that names `test` itself (`test(x) := …`) keeps its word: the pass does nothing.
- A condition may be a phrase: `test switch 3 {…} == "three"`. Write `test (#"héllo") == 5`: `test #x` is an index.

Open / not done:
- Page tests (`test "…" { render … }`, src/page_tests.rs) follow P209 too: skipped by a plain run, run under
  `warp test` and the playground's Run; their failures are ✗ lines in the Error, then `m of n failed`. A program mixing
  page tests with soft `test C` lines is not handled (the page runner takes the soft lines as setup).
- `test` lines inside functions or loops are not tests (top level and test blocks only).
- A fresh worktree needs web/playground/served-files.js for `cargo browser-test` (build.sh writes it).
