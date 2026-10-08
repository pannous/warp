# Traits (interfaces, protocols, typeclasses)

User request (2026-10-03), verbatim:
> sort ["b" "a"] perfect time to implement capabilities or aspects or whatever they are called in Rust. Similar to
> prototypes and interfaces in other languages.

User decision (2026-10-03), verbatim: "add all the synonyms we discussed before and add feature to interface protocol and
I think trait as the standard cannoic is fine because it's short"

What other languages call it: Rust traits, Haskell typeclasses, Swift protocols, Java/Go/TypeScript interfaces, JS
prototypes. Warp's canonical word is **trait**; `interface` (wiki/struct.md, WIT), `protocol`, `typeclass`, `prototype`,
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
```warp
trait shape{area perimeter}          // or area(s): parameter names only show in fixes
class square{side:int}
area(s:square) := s.side*s.side      // the witness area·square
area(square(3))                      // 9: calls area·square
square(1) is shape                   // false: perimeter is missing
area(dot(1))                         // compile error: dot is not shape: area(dot(1)) needs area(x:dot); fix: define area(x:dot) := …
```
An operation called on a value whose type is unknown at compile time calls the one type that defines it; when several
types define it, a generated dispatcher `area·dispatch(x) := if instance_of(x, "rect") then area·rect(x) else
area·square(x)` picks the witness at run time (traits::with_dispatchers; `instance_of` reads the instance's type name).

## Lowering (src/lowering/traits.rs)
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

## Membership through equals (2026-10-04)
`x in xs` (position) and `xs.has(x)` of an instance whose type overrides `equals` search the list with `equals·T`
(traits::position_by_equals, a loop lowered in lower_dispatch); without an override they compare structurally.

## Generic constraints (2026-10-04)
`smallest(xs: Comparable list) := (sort xs)#1`: the annotation `xs: T list` (analyzer::with_list_annotation; the parser
leaves `list` as the next signature item) is `list of T`, T a type word, a declared type or a trait. A list of a trait
holds Nodes; where the function is called with a list of instances of a known type, each operation of the trait must
have its witness (traits::unmet_constraint): `dot is not Comparable: smallest([dot{x:3}]) needs compare(a:dot, b:dot)`.
Lists of numbers or texts pass (sorted by value at run time). `xs: list of T` reads the same.

## Fixed on the way
`x = sort [3 1]; x#1` trapped (sort/split/reverse results were held as Ints), `(sort xs)#1` indexed the data list
`(sort xs)`, elements of a list of instances were held as Ints, `lower(a) == lower(b)` compared numbers.

## Printable and Iterable (2026-10-04, lowering/printable.rs)
`text(p:person)` (P31) and `iterate(b:bag)` become the witnesses `text·person`, `iterate·bag`; where an instance's type
is known, `as text` / str / print / interpolation call the first, `for x in b` and `x in b` walk what the second gives.
The pass runs right after type_constructor: later passes would read `for x in b` of an instance as a walk of its keys.

## Classes and foreign interfaces (classes-32, tests/types/test_class_interfaces.rs)
- A class method satisfies a trait operation: `trait Shape { area }` + `class Sq{side:int; area() := side*side}`; the
  method is the witness area·Sq. `s.area()` of a declared trait's operation is the call `area(s)`
  (traits::operation_calls), which picks the witness like any call.
- `f(s:Shape)` of a declared trait is untyped (traits::lower_trait_parameters before class_methods, again in
  lower_declarations for definitions later passes make): it takes any conforming instance. `xs: shape list` keeps its
  annotation (generic constraints above).
- Declarations as other languages write them: the spaced `trait Shape { area }`, Java's `double area();`, Kotlin's
  `fun area(): Int`, Swift's `func area() -> Double`, Go's `Area() float64` and `type Shape interface {…}`,
  TypeScript's `area(): number`, Rust's `fn area(&self) -> f64` (traits::foreign_signature: the receiver implicit).
- A class naming its traits, `implements Shape` (Java, TypeScript), `: Shape` (Swift, Kotlin), is skipped with a got-it
  note (topic conformance-list): conformance is structural (T2). Not a checked claim yet.

## Later
- `<` defined directly (`a:person < b:person := …`)
- `implements Shape` as a checked claim (claim_error) instead of a note
