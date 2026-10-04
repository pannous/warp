# Traits (interfaces, protocols, typeclasses)

User request (2026-10-03), verbatim:
> sort ["b" "a"] perfect time to implement capabilities or aspects or whatever they are called in Rust. Similar to
> prototypes and interfaces in other languages.

User decision (2026-10-03), verbatim: "add all the synonyms we discussed before and add feature to interface protocol and
I think trait as the standard cannoic is fine because it's short"

What other languages call it: Rust traits, Haskell typeclasses, Swift protocols, Java/Go/TypeScript interfaces, JS
prototypes. Wasp's canonical word is **trait**; `interface` (wiki/struct.md, WIT), `protocol`, `typeclass`, `prototype`,
`capability`, `aspect` and `feature` declare the same and hint toward `trait` (`traits::TRAIT_KEYWORDS`, first = canonical).
Note: the C++ wasp listed `prototype` and `interface` among its class kinds (Keywords.cpp); warp never had them as class
keywords, so they are free for traits.

## Decisions (forks T1–T5, approved by impl-text as recommended; T1 then set by the user)
- T1 keyword: `trait`, with the synonyms above.
- T2 conformance is structural: a type conforms by defining the operations, retroactively (Go/Haskell instance by
  definition, Swift extension). An explicit claim `class dot{x:int} is Comparable` is optional and checked: a claim
  without the operation is a compile error naming it (`traits::claim_error`, the one place to make claims mandatory).
- T3 a user type adopts a trait with free functions whose first parameter takes the type:
  `compare(a:person, b:person) := a.age - b.age` (negative, 0 or positive, like Rust's Ord, C's qsort, Python's cmp),
  `area(s:square) := s.side*s.side`. Methods in the class body or `<` can become sugar for these later.
- T4 Equatable: every built-in kind and every class instance is Equatable by value (field by field, synthesized like
  Swift's derived Equatable). `equals(a:word, b:word) := …` overrides it for a type.
- T5 dispatch: static where the types are known at compile time, a runtime witness table otherwise (below).

## Built-in traits
| trait      | operation              | built-in conformance                              |
|------------|------------------------|---------------------------------------------------|
| Comparable | `compare(a:T, b:T)`    | int, rational, real, float (by value), text, codepoint (by code points) |
| Equatable  | `equals(a:T, b:T)`     | every kind, instances by their fields             |

`x is Comparable`, `x is Equatable`, `x is shape` are type tests, answered at compile time.

## Declared traits
```wasp
trait shape{area perimeter}          // or area(s): parameter names only show in fixes
class square{side:int}
area(s:square) := s.side*s.side      // the witness area·square
area(square(3))                      // 9: calls area·square
square(1) is shape                   // false: perimeter is missing
area(dot(1))                         // compile error: dot is not shape: area(dot(1)) needs area(x:dot); fix: define area(x:dot) := …
```
An operation called on a value whose type is unknown at compile time calls the one type that defines it, else it is a
compile error asking for a type annotation (no runtime dispatch for declared traits yet).

## Lowering (src/traits.rs)
- `lower_declarations` (before type_tests): `trait shape{…}` becomes ø carrying the `Trait` (Meta data), which later
  passes collect (`Traits::of`); `x is shape` becomes the type test `is_type(x, "shape")`.
- `lower_conformances` (after type_constructor): a definition of a trait operation whose first parameter (every
  parameter of compare/equals) takes a declared type T is renamed to its witness `op·T`, so every type has its own.
  Instance parameters get the `Instance` mark on their annotation (analyzer::param_kind: held as a Node) and their uses
  the `TypedAs` mark. Claims are checked and dropped.
- `InstanceTypes`: flow-insensitive static shapes (instance of T, list of T) of constructions, variables, `xs#i`, `sort
  xs`, `c ? a : b`, `if … then … else`, for-loop variables and marked parameters. library_words reads fields of such
  values like of object literals, so `p.age` works on a parameter `p:person` and on `first = (sort xs)#1`.
- `lower_dispatch` (after library_words): `p < q` of known instances is `compare·T(p, q) < 0`, `p == q` is
  `equals·T(p, q) != 0` when T overrides equality, `op(x, …)` is `op·T(x, …)`. `sort`, `<`, `min`/`max` (lowered to `<`)
  and declared operations on a type without the witness are compile errors:
  `dot is not Comparable: sort needs compare(a:dot, b:dot); fix: define compare(a:dot, b:dot) := … (negative, 0 or positive)`.
- Runtime (wasm_emitter/witness.rs): `node_order` (sort, and `<` where the types are only known at run time) orders two
  instances through the witness table: the mutable funcref global `compare·witness` holds the generated dispatcher
  `instance_compare`, which matches the type name of the first instance and calls `compare·T` (call_ref; main installs
  it, `ref.func` is declared in an element segment). The dispatcher is compiled after the user functions because it
  needs their indices, while node_order is compiled before them; the global bridges the two. A type without a witness
  traps `not comparable`.

## Default methods (2026-10-04)
`trait shape{area; describe(s) := area(s) * 2}`: a requirement written as a definition is an operation with a default
body (`Operation::default`). `traits::with_default_methods` defines it, first parameter typed, for every type that defines
the other operations (`area(s:square)`) but not this one; lower_conformances then makes it the witness `describe·square`.

## Fixed on the way
`x = sort [3 1]; x#1` trapped (sort/split/reverse results were held as Ints), `(sort xs)#1` indexed the data list
`(sort xs)`, elements of a list of instances were held as Ints, `lower(a) == lower(b)` compared numbers.

## Later
- runtime dispatch of declared operations (a witness table per operation, like compare's)
- Printable (`text(p:person)` for interpolation and `as text`), Iterable
- generic constraints `sort(xs: Comparable list)`
- `x in xs`, `xs.has(x)` through an `equals` override (they compare structurally at runtime now)
- `<` defined directly (`a:person < b:person := …`) and methods in the class body as sugar for the witnesses
