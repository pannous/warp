# Inspiration: syntax-precedence

Verified locally (2026-09-26) with python3, node, rustc, swiftc, go, ghc, ruby, julia (`--startup-file=no`)
in `probes/footguns/inspiration/scratch/`, unless marked *(not run)*.

### Unary minus and power
Solved elsewhere: **Python / Julia / Haskell**: `-2**2` / `-2^2` → `-4` — `**`/`^` binds tighter than prefix minus, as in mathematics.
**JavaScript**: `-2**2` → *SyntaxError: Unary operator used immediately before exponentiation expression* — the ambiguous form is simply rejected; `(-2)**2` → `4`.
**Rust**: `-2i32.pow(2)` → `-4` — method call binds tighter than unary minus.
Adopt in Warp: give `^` (and `²`, `³`) higher precedence than prefix `-`, so `-2^2` → `-4` like Python/Julia; a literal `-2` token is still negation of `2^…`, never a signed base. Optionally emit a normalizer hint recommending `-(2^2)` / `(-2)^2` in the canonical view.

### `not` and bitwise operators vs comparison
Solved elsewhere: **Python**: `not 1==2` → `True`, `3 & 4 == 4` → `False` — `not` binds weaker than comparisons; `&`,`|`,`^` bind *tighter* than comparisons (fixing C's historical order).
**Go / Rust**: `3&4 == 4` → `false` — same fix: bitwise ops at multiplicative/additive level, above `==`.
**Carbon** *(not run)*: precedence is a *partial order*; `a & b == c` is a compile error demanding parentheses.
Adopt in Warp: `not`/`and`/`or` below comparisons (Python), and either Python's "bitwise above comparison" or Carbon's partial order (mixing bitwise with comparison without parentheses is a diagnostic with a fix-it). The partial order fits DESIGN.md's "never guess" best; `|` staying pipe means bitwise or needs its own spelling (`bitor`, `∨`?) anyway.

### Chained comparison
Solved elsewhere: **Python / Julia**: `3>2>1` → `True`, `1<3>2` → `True` — `a<b<c` desugars to `a<b and b<c` with `b` evaluated once.
**Rust**: `1 < 2 < 3` → *error: comparison operators cannot be chained; help: split the comparison into two* — refuses rather than guesses.
**Haskell**: `3 > 2 > 1` → *Precedence parsing error: cannot mix ‘>’ [infix 4] and ‘>’ [infix 4]* — comparisons are declared non-associative (`infix 4`).
Adopt in Warp: Python/Julia chaining for same-direction chains (`a<b<c`, `a≤b<c`, `a==b==c`) desugared in elaboration with the middle operand bound once; mixed-direction chains like `1<3>2` (legal but confusing in Python) are a diagnostic.

### Assignment in a condition
Solved elsewhere: **Python**: `if x = 2:` → *SyntaxError: invalid syntax. Maybe you meant '==' or ':=' instead of '='?* — `=` is a statement; binding inside an expression needs the distinct walrus `:=`.
**Swift**: `if i = 2 {}` → *error: use of '=' in a boolean context, did you mean '=='?* ; **Rust**: `if x = 2 {}` → *mismatched types, help: you might have meant to compare for equality* — assignment has type `()`/`Void`, never Bool.
**Kotlin** *(not run)*: "Assignments are not expressions, and only expressions are allowed in this context".
Adopt in Warp: assignment evaluates to unit, so in a Bool-expected position (condition of `if`/`while`) `=` is elaborated as comparison (as [wiki/Bad.md](../../../wiki/Bad.md) wants) *or* rejected with a fix-it — pick one and persist it; `if 1=2` must never be truthy. Bidirectional typing (Bool expected) makes this an elaboration rule, not a parser heuristic.

### 🐞 Increment
Solved elsewhere: **Swift** (removed ++ in Swift 3, SE-0004): `i++` → *error: cannot find operator '++' in scope; did you mean '+= 1'?*
**Rust**: `i++` → *error: Rust has no postfix increment operator; help: use `+= 1` instead*.
**Go**: `j := i++` → *syntax error: unexpected ++* — `i++` exists but only as a statement, never an expression, so `i++ + i++` cannot be written.
Anti-example **Python**: `i=1; ++i` → `1` silently (`+(+i)`), a footgun of its own.
Adopt in Warp: Go's rule — `x++`/`++x` is a statement meaning `x += 1` (both spellings identical, per wiki/equality.md), its value is unit so it can't be used inside an expression; `++` applied to a non-place is a diagnostic, never `+(+x)`. First fix the bug that `x=1;x++;x` → `1`.

### Braceless calls
Solved elsewhere: **Ruby**: `f 3-1` → `20` (argument is the whole expression) and `1 + f 3` → *SyntaxError* — a braceless call is only allowed where it is unambiguous, otherwise loud; `f -1` → warning *ambiguous first argument*.
**Haskell**: `f 3-1` → `29`, `1 + f 3` → `31` — the opposite, but *one* fixed rule (application binds tightest) plus `$` for "rest of line": `f $ 3-1` → `20`.
**Julia**: `2x` → `6` — juxtaposition restricted to numeric-literal coefficients, where it cannot mislead.
Adopt in Warp: keep Warp's chosen rule (braceless call takes the whole remaining argument expression, like Ruby/`$`) and apply it uniformly also in operand position, so `1 + f 3` → `1 + f(3)` → `31`; `fib it-1 + fib it-2` then needs the known arity of `fib` (1) to stop the argument at the next `+`-level call — resolve that with the declared signature in elaboration, and where arity is unknown emit a diagnostic instead of a parse.

### Automatic semicolon insertion
Solved elsewhere: **Go**: `x := 1⏎-1` → *compile error: -1 (untyped int constant) is not used* — the lexer inserts `;` after a line ending in an identifier/literal/`)`/`}`/`return`/`++`, a purely lexical, documented rule, and a dangling expression statement is an error.
**Python**: newline ends a statement unless inside brackets or after `\`, so `return⏎{…}` returns `None` *and* the dict line is a separate (dead) statement.
**JavaScript** (the footgun, verified): `let x=1⏎-1` → `x == 0`.
Adopt in Warp: already solved (newline ends the statement); add Go's second half — a bare value statement whose result is discarded (`-1` on its own line mid-block) is a diagnostic, and continuation only inside open brackets or after a trailing binary operator.

### Guessing intent
Solved elsewhere: **Unison** *(not run)*: code is stored as a content-addressed AST; names are resolved once at `add`/`update` time and the hash, not the text, is what later builds consume — exactly "resolve once, persist".
**Python** (`:=` walrus) / **Pascal/Ada** (`:=` vs `=`): assignment, definition and comparison get *distinct tokens*, so no guess is needed.
**Rust/Swift/Python diagnostics**: `did you mean '=='?` — the compiler proposes, the programmer accepts (machine-applicable fix-it, `cargo fix`).
Adopt in Warp: keep the DESIGN.md plan (content-addressed resolution sidecar, Unison-style); canonical view uses distinct spellings (`:=` define, `=` assign, `==` compare) so an accepted resolution can be written back into source, and fix-its are machine-applicable so agents/`warp fix` persist the choice.
