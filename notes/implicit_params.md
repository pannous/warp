# Implicit parameters: `it`, `$0`/`$1`, `_` and `$1` references (P160 background, 2026-10-06)

Survey for P160 (`$1` in `a[id=1]{ b c { parent=$1 } }`). Every warp result below was run with the warp CLI of
branch wiki-references (on main 7d1f05c79); "→" is what warp gives today.

## What other languages do

| form | language | meaning |
|---|---|---|
| `it` | Kotlin, Groovy | the single parameter of a lambda; a nested lambda's `it` shadows the outer one (Kotlin warns) |
| `$0`, `$1` | Swift | positional parameters of a closure; not allowed in a nested closure that names them too |
| `$0`, `$1` | WebAssembly text | `$name` is an identifier; a function's parameters are locals 0, 1, … (`local.get 0`); a module without a name section has only these indices |
| `$1` | shell, Perl, regex | the 1st argument or capture group (`$0` = the program / whole match) |
| `_` | Scala | each `_` is the next parameter: `_ * 2`, `_ + _` takes two |
| `_` | Python, Rust, Swift | a throwaway name (`_ = f()`, `for _ in`), Swift's `_` argument label |
| `#1`, `%1` | Mathematica, Clojure | positional parameters (`#1 + #2 &`, `#(+ %1 %2)`) |
| `$a`, id refs | YAML anchors `&a`/`*a`, JSON `$ref`, XML `id`/`idref` | a reference to another node of the same document |

## Where each form changes meaning in warp

### `it`: the one parameter
- function body: `square := it*it; square 3` → 9; a getter without `it` stays a getter (`area := 3*4`).
- lambda block: `[1,2,3].map { it*2 }` → [2 4 6].
- loop: `for 1 to 3 { print it }` and `1…3 do print it` → `it` is the item (wiki/range.md).
- CLASH, nested lambdas: `[[1,2],[3]].map{ it.map{ it*10 } }` → "undefined variable: it" (Kotlin: inner `it` shadows).
- CLASH, lambda inside a function with a parameter: `f(x) := { [1,2].map{ it + x } }; f(10)` → the inner `it` was
  rewritten to `x` ("[1, 2].map{x+x}"): the function's `it` binding reaches into the inner lambda.
- CLASH, glued brace: `xs.map{it*2}` → "undefined variable: it", while `xs.map { it*2 }` works: `name{…}` without
  a space is the data literal `map{…}` (the same form as `a{ b c }`, the form references live in).

### `$0`, `$1`: positional parameters
- Swift closure: `f = { $0 + $1 }; f(1, 2)` → 3; `sorted(xs, by: { $0 > $1 })`; `apply(using: { $0 * 3 }, to: 2)`.
- function body: `f := $0 * 2; f 4` → 8; with named parameters `add1 x := $0+1; add1 3` → 4 (tests/wasm/test_wasm.rs).
- WebAssembly positional: the emitter compiles `$n` to `local.get n` (src/wasm_emitter/arithmetic.rs
  emit_numeric_symbol), so `$0` is "local 0" of the compiled function. That is the wasm meaning, and it is why a
  Swift closure that captures values is lifted with the captures first (`closure_lambda(k, $0)` renames them).
  Imported modules: a parameter without a name in the module's name section is called `$0`, `$1` …
  (src/wasm_modules.rs, Export.parameters); function equality renames parameters to `$0`, `$1` to compare bodies
  (src/function_equality.rs). WAT identifiers `$add`, `$main` keep their sigil (tests/parser/test_dollar_names.rs).
- CLASH, nested closures: `[[1,2],[3]].map{ $0.map{ $0*10 } }` → error: which closure's `$0`? (Swift forbids it.)
- CLASH, block with declared parameters: `g(a, b) := { $0 - $1 }; g(5, 2)` → `closure_lambda_1` (a closure value),
  though src/lowering/closures.rs means it to be the body with `$0` = a.
- CLASH, data literal: `x = a{ id:1 b{ up=$1 } }` → `b` becomes a closure (`a{id:1 b:closure_lambda_1}`): any
  `{…}` holding `$1` reads as a Swift closure.

### `$1` / `$a`: references in data (wiki/reference.md)
- implemented default (branch wiki-references): `a[id=1]{ b c { parent=$1 } }` is `a{@id:1 …}`; `$1` is a reference
  only when an enclosing node of the same literal declares `@id:1`, `$a` when one is named `a`; a path follows them
  (`x.c.parent.b` → `x.b`). Otherwise `$1` stays the parameter.
- CLASH: `a{ id:1 b{ up=$1 } }` (id as a field, not an attribute) is still the closure above; and a reference inside
  a lambda inside a literal (`a[id=1]{ f:{ $1 } }`) now means the node, not the closure's second parameter.

### `_`
- a plain name: `_ = 5; _` → 5; Swift's argument label `func apply(_ f: …)`.
- not a parameter: `[1,2,3].map{_*2}` → "undefined variable: _" (Scala would give [2 4 6]).

## The clashes in one line each
1. `$n` means three things: a closure's n-th argument (Swift), the compiled function's local n (wasm), and now a
   node with id n (references). Nested closures make the first ambiguous.
2. `it` leaks from an enclosing function into an inner lambda, and nested `it` is undefined instead of shadowing.
3. `name{…}` glued is data, `name {…}` spaced is a call with a block: `xs.map{it*2}` fails.
4. A block is decided to be a closure by what it contains (`$n`, `it`), not where it stands.

## Recommendation
- References get their own sigil-free spelling or a distinct one, so `$n` keeps one meaning: a reference names the
  node's id or name (`$a`, `$#1` or `^1`), and `$<digits>` is only ever a positional parameter. The current default
  (reference only when an enclosing node declares the id) works, but a reader cannot tell the two apart locally.
- `it` and `$0` in a nested lambda: the innermost lambda wins (Kotlin's shadowing), with a got-it note when an outer
  `it` is shadowed; a function's `it` binding stops at an inner lambda.
- Glued `name{…}` after a dot (`xs.map{…}`) is a method call with a block, as in Kotlin/Swift/Ruby; `a{…}` at the start
  of an item stays data.
- `_` stays a name (no Scala placeholder): one more implicit form would add a fourth meaning.
