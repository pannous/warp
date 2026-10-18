# Implicit parameters: `it`, `$0`/`$1`, `_` and references

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

## The rules in warp
- `it` is the one parameter of a function body (`square := it*it`), a lambda block (`[1,2,3].map { it*2 }`) or a loop
  (`for 1 to 3 { print it }`). A getter without `it` stays a getter (`area := 3*4`).
- `it` and `$0` belong to the innermost lambda (Kotlin's shadowing): `[[1,2],[3]].map{ it.map{ it*10 } }`; a
  function's `it` binding stops at an inner lambda (cards nested-it, it-leak).
- Glued `name{…}` after a dot (`xs.map{…}`) is a method call with a block, as in Kotlin/Swift/Ruby; `a{…}` at the start
  of an item stays data (card glued-block).
- `$<digits>` is only ever positional: Swift closures `f = { $0 + $1 }`, a function body `f := $0 * 2`. The emitter
  compiles `$n` to `local.get n` (arithmetic.rs emit_numeric_symbol), the wasm meaning, so a closure that captures
  values is lifted with the captures first (`closure_lambda(k, $0)` renames them). Imported modules' unnamed
  parameters are `$0`, `$1` … (wasm_modules Export.parameters); function equality renames parameters to `$0`, `$1`
  to compare bodies. WAT identifiers `$add`, `$main` keep their sigil (tests/parser/test_dollar_names.rs).
- References in data (P160, wiki/reference.md): `a[id=1]{ b c { parent=@1 } }` is `a{@id:1 …}`; `@1` names the
  enclosing node whose `@id` is 1, `$a` the one named `a`, in words `ref 1` / `ref a` (a variable or function named
  ref wins); a path follows them (`x.c.parent.b` → `x.b`). `$1` beside an enclosing id 1 stays the closure
  parameter, with a note naming `@1`.
- `_` stays a plain name (`_ = 5`, Swift's argument label), no Scala placeholder.

Open: `a{ id:1 b{ up=$1 } }` (id as a field) makes `b` a closure, as any `{…}` holding `$1`; a block is decided to be
a closure by what it contains (`$n`, `it`), not where it stands.
