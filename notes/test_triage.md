# Test triage 2026-10-28 (decision night-releases-and-test-triage)

The failing lines of test/results.txt (aa60d3ddc3, 4149 ✗), sorted into three groups:
- **gap**: the test is right and warp should do it. These are grouped into cards below and not edited.
- **wrong**: the expectation is wrong. These are listed for the user to approve and not edited.
- **esoteric**: a feature we don't carry over. These are commented out with
  `# triage-held 2026-10-28: <reason> :: <the line>`; an is! test's further lines start with `# :: `.
  To restore one, remove everything up to and including ` :: `, or `# :: ` on a continuation line.

## Wrong expectations (approve to correct)

| file:line | expected now | proposed | why |
|---|---|---|---|
| test/welcoming/test_c_style.warp:26 | "class float" | "class Real" | π is the exact Real (decision exact-constants) |

## Held (esoteric)

| file:line | reason | delete? |
|---|---|---|
| test/welcoming/test_footguns.pairs.warp:383 | `fast` floats that reassociate math | delete? |
| test/welcoming/test_footguns.pairs.warp:395 | C float32 suffix `.1f` | delete? |
| test/welcoming/test_footguns.pairs.warp:403, 404, 425 | symbolic surds (√2·√2 exactly 2) | |
| test/welcoming/test_footguns.pairs.warp:414 | symbolic trig, sin(π/6) exactly 1/2 | |
| test/welcoming/test_footguns.pairs.warp:428 | complex exponentials (Euler's identity) | |
| test/welcoming/test_footguns.pairs.warp:442, 445, 446 | function equality by algebra (undecidable in general) | delete? |
| test/welcoming/test_footguns.warp:44 | function equality by boolean algebra | delete? |
| test/welcoming/test_footguns.pairs.warp:262, 263, 266, 267, 268 | tzdata version stamps on times (267 and 268 contradict each other) | delete? |
| test/welcoming/test_footguns.warp:9 | `law` property declarations | |
| test/welcoming/test_footguns.warp:33, 37 | effect annotations `! Pure`, `! Div` | |

## Gap groups (cards)

### welcoming (155 gaps)
Silently wrong, which matters most:
- `for char in "abc" { n += 1 }` gives 0 (test_golf_words:12, 13).
- A big integer's `rem` gives 0 (test_footguns:362).
- `x.upper!` leaves x unchanged (test_discarded_pure_warning:5, 6, 11).
- `"a" + warning("careful") + "b"` gives "aøb" (test_warning_mode:4).

Groups:
- Dates, times and durations as values: `2024-02-29.month`, `1 hour == 60 minutes`, zoned times. About 25 lines, error "a data".
- A round group `(1 2)` as a list value, mutable and aliased. 13 lines, error "the group (…".
- Assigning to a text element, `x#1 = 'é'`. 8 lines.
- English operator words: plus, minus, equals, greater/less than, at least/most, through. 9 lines.
- Text views: `as bytes`, `as codepoints`, `as graphemes`, `byte in t`, "number of bytes in". 9 lines.
- `xs.insert(v, at: i)`. 7 lines.
- Bool arithmetic, `true + true`, `(1<2) + 1`, `v.has(1) + 1`. 5 lines.
- `round_half_even` and `round_half_up`. 6 lines.
- A method without parentheses, `xs.product`, `s.capitalize`. 6 lines.
- `sleep` with a spaced unit, `sleep(16.7 ms)` or `sleep 5 ms`. 5 lines here, many more in control.
- Float ∞ and NaN, `1.0/0.0`. 3 lines.
- `range(n)`. 3 lines.
- `if x=2` read as a comparison. 3 lines.
- `++i` and `--i`. 3 lines.
- Pairs one at a time: C-style typed declarations, `def … end`, `x <- 3`, `let mut`, `next`, `xs count`,
  shadowing pi and count, sh and sql templates, `fetch`, function equality by definition (f==g alpha-equal).
