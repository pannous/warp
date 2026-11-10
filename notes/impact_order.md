# Open cards by impact (user 2026-11-10: the next cards go out in this order)

The batch before this one was picked by ease. From here on, each worker that frees up takes the highest card below
that fits it and that nobody holds. Cards in progress finish first. Impact means: how many programs, users or workers
the card unblocks, and how bad the failure is. A wrong value is worse than a refusal (AGENTS.md "Refuse, never guess").
Paused topics stay paused (the events roadmap is behind the tests roadmap, user 11-09). Visitor cards are not ranked.
A card that needs core/ or extensions/ still asks warp-interviewer first.

## 1. Main and the lanes green: every worker waits on these
| # | card | why |
|---|---|---|
| 1 | after-push-tiers-1-2-1110 | gate tier 2 red on main (4 of 12): every landing is judged against it |
| 2 | zero-bake-regressions | 28 feedback-test regressions after L3 (tests that passed before) |
| 3 | after-push-self-lane-1109 | promote-self build-failed: the self lane (warp self) is stale |
| 4 | playground-ci-red | Playground CI red since the seed refresh: the public page is unguarded |
| 5 | shared-test-slot | one test slot machine-wide while a build runs (in progress, user OK) |

## 2. Wrong values: a plausible wrong result is the worst bug
| # | card | why |
|---|---|---|
| 6 | spaced-unit-comparison | `2 km == 2000 m` gives `2 no m`; `if x > 1500 m:` a DimensionError (in progress) |
| 7 | core-while-nested-method-arg | `out.add(q.pop())` inside `while` silently adds nothing |
| 8 | core-lambda-block-capture | a block inside a lambda doesn't change the lambda's local: `thrice { n += 1 }` stays 0 |
| 9 | class-setter-ignored | a class setter is silently skipped |
| 10 | gcd-overflow-zero + three-gcds-generic | gcd past 64 bits gives 0; one generic gcd fixes both |
| 11 | position-range-empty | `xs#2..3` prints nothing |
| 12 | string-concat-warn + warning | `"a" + warn("x") + "b"` gives `aøb` |

## 3. The playground: the public face, where the user and visitors try warp
| # | card | why |
|---|---|---|
| 13 | ref-timestamp-bug | the playground traps where native runs, with a position outside the file (in progress) |
| 14 | page-only-test-reds | 14,224 of 15,593 tests pass in the page vs 14,667 of 16,104 natively: one gap, many symptoms |
| 15 | example-native-mismatch (user 🐞) | an example runs natively, but only two frames in the browser |
| 16 | slow-website-startup (user 🐞) | 700 ms for a bare website |
| 17 | pressing-try-run | old output stays visible after try/run |

## 4. Speed and size: hello costs ~1.58 G instructions, ~10,000× what it should (notes/hello_budget.md)
| # | card | why |
|---|---|---|
| 18 | compile-parses | every compile parses lib/math.warp whole |
| 19 | compile-sugar-unpack | every compile unpacks the sugar rules |
| 20 | non-sugar-lowering | `print "Hello"` runs every lowering pass over the reached library words (~700 M, the biggest share) |
| 21 | compiler-start-cost | warp.wasm's _start retires 39 M before any command |
| 22 | hot-predicates-texts | hot predicates turn nodes into texts to compare them |
| 23 | rebuilding-wasm | the compiler build went 4.4 → 6.5 T: every build, every worker |
| 24 | text-residual | text == still costs warp-self 2.45 T vs 2.28 T before |

## 5. Red test clusters: pass rate (cloud workers fit these: lib-only, report the rest)
| # | card | why |
|---|---|---|
| 25 | cloud-suite-failures | Linux reds sorted into platform vs real (cloud, in progress) |
| 26 | lists-n-z-lib, types-n-z-lib | the next slices after numbers and control |
| 27 | tree-find-replace-gaps | test_tree_find/replace 7 red |
| 28 | the numbers n–z core clusters | units as literals (16 lines), a variable named like a unit (8): notes/test_triage.md |

## 6. Then
User 🐞 cards not ranked above (repl-bash-commands, sample-without-annotation, complex-number-examples, sort-more …),
then the Soon column in its own order. module-manager (user card) needs a design first: it goes to warp-interviewer
when a worker with time for design frees up.
