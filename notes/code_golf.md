# Code golf (card code-golf, 2026-10-09)

19 holes of [code.golf](https://code.golf), each solved twice in samples/golf/: `<hole>.warp` readable, `<hole>.short.warp`
golfed. `<hole>.txt` is the expected output, `<hole>.args` the arguments (arabic-to-roman, rot13); the quine has none.
- probes/golf_expected.py writes the .txt and .args files from reference solutions in Python.
- probes/golf_check.py runs every solution and prints its char count or the first differing line.
- probes/golf_best.py scrapes the leaderboards for the best char counts (3 s between requests: code.golf answers 429).
- tests/programs/test_code_golf.rs runs every solution, natively.

## Char counts

Chars as code.golf counts them: Unicode characters, so `…` is one. Best = code.golf's leaderboard, 2026-10-09.

| hole | warp readable | warp short | Python | Ruby | JavaScript |
|---|---|---|---|---|---|
| fizz-buzz | 200 | **51** | 53 | 45 | 56 |
| fibonacci | 184 | 44 | 36 | 25 | 33 |
| prime-numbers | 109 | 50 | 40 | 27 | 36 |
| 99-bottles-of-beer | 493 | 293 | 118 | 120 | 142 |
| pascals-triangle | 155 | 64 | 50 | 40 | 52 |
| quine | 510 | **2** | 31 | 25 | 25 |
| arabic-to-roman | 435 | 175 | 88 | 63 | 90 |
| divisors | 130 | 47 | 53 | 41 | 49 |
| happy-numbers | 223 | 88 | 48 | 30 | 51 |
| leap-years | 203 | 50 | 43 | 33 | 40 |
| catalan-numbers | 159 | 46 | 44 | 32 | 41 |
| sierpiński-triangle | 249 | 85 | 54 | 46 | 47 |
| diamonds | 342 | 85 | 56 | 47 | 75 |
| collatz | 258 | 61 | 57 | 46 | 54 |
| look-and-say | 376 | 112 | 60 | 51 | 60 |
| evil-numbers | 162 | 60 | 40 | 26 | 37 |
| niven-numbers | 143 | 48 | 44 | 29 | 42 |
| pernicious-numbers | 172 | 51 | 40 | 28 | 34 |
| rot13 | 337 | 109 | 59 | 35 | 89 |

The leaderboard entries are years of collective golfing; the warp ones are a first pass. Warp wins fizz-buzz (51 to
Python's 53), divisors (the math module's `divisors`) and the quine (`1` + a line break: a script echoes its final
value). It is close on catalan, collatz, niven and leap-years.

## What helps
- `for 1…100:` with `it`: `…` is one char and inclusive (`..` excludes the end).
- `use math` words: is_prime, divisors, choose, digits, to_base. `use os` gives `args`.
- `"x"*bool` repeats 0 or 1 times, `"Fizz"*a+"Buzz"*b||it` falls back to the number, and `\(expr)` holes. A bare `$x` stays literal
  text by decision.
- Exact big integers: catalan's 57 digits and `((10^w-1)/9)^2` (the diamond rows 12321) need no special code.
- One-line function definitions: `h(n):=n>4?h(…):n<2`.
- A loop head or `f(n):=` ending its line takes the lines indented below it, by tabs or spaces, no `:` needed; a
  comma line there is one statement (`a, b = b, a + b` swaps).
- Open slices: `s[2…]` to the end, `s[…2]` from the start (inclusive; `..` excludes the end).
- `print x` without parentheses after `&&`/`||`/`and`/`or`, in an `if c:`/`then`/`else` branch, and guarded by a
  trailing `if` (`print it if it%2` prints nothing when false).
- `s.chars` walked (`for c in s.chars`, `s.chars.map(…)`) is the list of chars; elsewhere it is the count.
  `for char in s` visits every char of a text.
- `for (1…5).filter(f):` walks any expression; only a `;` in the group makes it C's `for(i=0;i<n;i++)`.
- `chr(…)` of any expression as a function result: `f(x):=chr(x+1)`.

## What costs chars
- The final-value echo: `warp run` prints a script's last value after its printed output, so a program must end with
  a print. `for …:{print a;a,b=b,a+b}` adds a line with b; ending with `ø` or `()` still echoes the loop's value; a
  text echoes quoted. Workarounds: `print (…).filter(f).join("\n")` or a last `print`. Kept by decision P236
  (card golf-echo).
- `&` and `|` are logical (by decision); bits by word: `bit_and`, `bit_or`, `bit_count` (`use math`), `xor` is bitwise.
- Negative indexes don't wrap (by design: `last(xs)`).
- `text` is a type name, so `for text in args` filters by type, with a warning.

## Gaps filed (board column Next)
none open.
Fixed: golf-expression (`for (…).filter(f):`), golf-bits (bit_and, bit_or, bit_count), golf-count (the error names `use text`); golf-chr, golf-chars, golf-print, golf-inline; golf-text, golf-sum (text arithmetic is no number for `||`/`or`); golf-indented, golf-bare,
golf-indented-while, golf-indented-swap (indented bodies without `:`); golf-open (`s[2…]`, `s[…2]`). Decided: golf-echo (P236, kept).
