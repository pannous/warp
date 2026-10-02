# Levenshtein field test report (worker warp-a0 → supervisor warp-f3)

Branch: `algo-levenshtein`. The natural version fails (`test_levenshtein` is `#[ignore = "next"]`); `test_levenshtein_idiomatic` passes (= 11).
Probe runner: `cargo --offline test --all-features --test test_algo_levenshtein probe -- --ignored --nocapture` evaluates every snippet in this folder.

## Natural program
`samples/levenshtein.wasp` → `Error("undefined variable: let")`, expected 11.

## Failures (minimal snippet → actual vs expected; cause)
1. **`let`/`var` inside any block** (fun, for, if): `fun f(n) { let m = n + 1; return m }; f(2)` → `undefined variable: let`, expected 3.
   At top level it only works by accident: `foo x = 3; x` → 3 too, so the unknown head word is dropped silently. Nothing lowers `let`/`var`; the only code that sees it is the style hint (normalize.rs:774).
   Wiki: `let` is not reassignable, yet `let x=3; x=4` → 4.
2. **Indexing a string parameter**: `fun f(a) { return a[1] }; f("abcd")` → `index out of range`, expected 'b'. `a[0]` → cast failure, `a#2` the same; `a: text` and `a: string` annotations don't help.
   A variable argument gives `f needs an Int for parameter a, got s`, which also names the wrong kind.
   Cause: analyzer.rs:1677 `with_usage_kinds` sets `used_as = List` for any indexed parameter. `infer_parameters_from_calls` (analyzer.rs ~1786) then skips the parameter because `used_as` is already set, and an annotation doesn't override it.
3. **A list literal with an index element**: `p=[5,6,7]; c=[p[1]]; c[0]` → cast failure, expected 6. `[1] + [p[1]]` → `list + int`. This also breaks `c.push(p[1])` and `curr.push(prev[j] + 1)`, because push lowers to `c = c + [e]` (analyzer.rs:1398). `[#s]` breaks too.
   Cause: wasm_emitter/list_emitter.rs:360 `is_statement_sequence` counts `Key(_, Op::Hash, _)` as a statement, so the literal is evaluated as a block.
4. **Pushing a min() result**: `c=[1]; x=2; y=3; c.push(min(x, y)); c#2` → `list + int`, expected 2. min lowers to `x<y ? x : y`, and inside `[...]` the `x : y` is read as a key:value entry. Fix in min_max.rs `extremum`: parenthesize the ternary.
5. **min() rejects indexing**: `p=[1,2]; min(p#1, 5)` → `min arguments must be plain values or arithmetic`, expected 1. Indexing has no side effects (min_max.rs:56). Better: bind the arguments to temp locals.
6. **`for j in 0...n { … }` never terminates**: → out of fuel, expected 6 (with s += j, n = 3). The parse is `for j in 0 to (n {s+=j})`, because `try_parse_for_in` (wasp_parser.rs ~1610) splits off `xs {…}` only at the top of the iterable, not inside the `to` key. `0...3 {` and `0...(n) {` work.
7. **`+=` mixes scopes**: `def g(a) { t = a; 7 }; t = 0; t += g("x"); t` → `WASM validation failed: type mismatch: expected i64, found (ref $type)`, expected 7. `t = t + g("x")` works.

## Also noticed
- `len(x)` → `undefined function: len`; Python users will write it.
- `#s` alone on a line (for example a function's last expression, or `{ #a }`) is a shell comment (wasp_parser.rs:584), so `def g(a){ s=a\n #s }; g("abc")` → "abc". That's a footgun, since the length hint itself recommends `#var`. Suggest: `#ident` with no space counts; `# ` with a space is a comment.

## Non-idiomatic constructs (all should be eaten, with education)
| mine | idiomatic |
|---|---|
| `0..n` meant inclusive (Kotlin) | `0...n`; `..` is exclusive (Swift). This gives a silently wrong answer! |
| `let x = …` | `x := …` (hint) or plain `x = …` |
| `a.length` | `#a` |
| `a[i-1]` 0-based on text | `a#i` 1-based (wiki/indexing.md) |
| `fun` | works; samples use `def` |
| `return x` at the end | a bare last expression |

## Workarounds the idiomatic sample needs
- no `let`
- copy the parameters to locals (`s = a`) to dodge #2
- `0...(#t)` in parentheses to dodge #6
- `best = min(..)` then `push(best)` to dodge #3 and #4
- named locals for the min arguments to dodge #5
