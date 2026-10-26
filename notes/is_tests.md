# is! tests as warp files (card warp-is-tests, decision warp-is-tests)

Tests are warp files under test/<topic>/, two kinds, never mixed: a pairs file of pure data with the short tests
(test/numbers/test_math.pairs.warp), one test a line, tab separated: code ⇥ expected ⇥ an optional note :

```
'2+3*4'	14
'0-1'	-1	a note
// 'fib 10'	55	card fib-memo: regression
```

and an ordinary warp file (test/numbers/test_math.warp) with an `is!` line for each test whose pair would be longer
than 80 characters, whose code spans lines, or whose expectation is computed (`1 + pi`):

```warp
is! 'use sin;sin π' 0
```

- Warp's own reader takes a pairs file as a list of lists: a tab between two items of a line separates columns
  (looser than `;`, tighter than a newline); indentation and a tab before a comment or the line's end separate
  nothing. An empty column keeps its place (`'y'⇥ø⇥note`). A text alone is a program, never a pair: an expected ø is
  written `'y'⇥ø`.
- A commented-out pair is a skipped test, its note naming the card of the regression that broke it: `warp test` prints
  `⊘ skipped "fib 10" 55: card fib-memo: regression`. So a commit breaking a few lines can land: forward crunch per line.
- Matching is permissive (lib/prelude.warp is!): `got == expected`, or both empty/falsy (0 ≈ ø ≈ [] ≈ no).
- `is!` is lib/prelude.warp's `is!(warp: text, expected: any)`: `eval(warp) == expected`; a failure prints
  `✗ is! "3+3" 7: got 6`. Programs may define it the user's way, `def is!(warp: text, expected: any) := …`.
- Bang words: a name glued to `!` is one word when the program or a standard module defines it. Undefined, the `!`
  keeps its meanings: suffix (`x!`, `name! email?`), macro (`println!(…)`), `!=`.
- Code goes in single quotes (`'…'`, `«…»` when it holds a quote): nothing interpolates there. A negative or computed
  expectation goes in parentheses: `is! '0-1' (-1)`.

## `warp test [folder…]`: the runner (card warp-test-runner)
`warp test` (test/ by default) or `warp test <folder>…` runs every `*.pairs.warp` pair and every top-level `is!`
line of the `test_*.warp` files under the folders through `warp pure`, with the installed binary
(src/compiler/pure_tests.rs). Each test is its own program in a child process (`warp test --case <file> <index>`,
half the cores, at most 8 at once, 30 s each), so a hang, trap or crash fails only that test. Its `use` reads from
the test file's folder. An expected column that is not data (`π`, `(-1)`, `1 + pi`) runs through `warp pure` too.
Prints ⊘ for skipped pairs, a ✗ line per failure (`file:line "code" expected: got …`), then `m of n failed in f files,
s skipped`; exits 1 on a failure. `--self`: through `warp self` (warp-self.wasm); `--each`: a ✓ line for each passed
test too. The verdict lines print in file order after the run, so two runs (pure and self) diff line by line. The
parent builds a missing compiler first, so children never race to build it.
`warp pure "2+3*4"` / `warp self "…"`: inline code through the warp compiler, `use` reading from the current folder.

## Known gaps (cards)
- eval of code holding π, time literals, nan, inf, complex: "parse_code: π cannot cross to another task yet": card
  eval-code.
- Each is! compiles its code in a module of its own through the host: ~0.1–0.2 s a line.
