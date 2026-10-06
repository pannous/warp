# Classes with methods (issue #14, card g-1nug)

## What works today (2026-10-06)
- `class`/`struct`/`type`/`record` declare a type: fields, typed fields (`age:int`), optional fields (`left?`).
  Construction: positional `person("Ann" 40)` or named `person{name:"Ann"}`. Field access `p.age`.
  Trees as values: the issue's `Tree`/`insert` example runs.
- Methods as free functions over the type: `greet(p:person) := "hi " + p.name`, called as `greet(p)` or `p.greet()`
  (uniform call syntax, library_words). Dispatch on the parameter type (overloads.rs).
- Traits (wiki/trait.md): Comparable, Equatable, declared traits; conformance is structural, by free functions.
- Missing: methods written **inside** the class body. `class person{name; greet() := "hi " + name}` counts `greet` as
  a field ("person takes 2 fields, got 1").

## Plan
1. **Methods in the class body** (this branch). Each definition in a class body becomes a type-dispatched function
   whose first parameter is the receiver: `greet() := "hi " + name` in class person is
   `greet(self:person) := "hi " + self.name`.
   - In the body, a bare field name reads the receiver's field; `self` and `this` name the receiver.
   - Calls: `p.greet()`, `greet(p)`, and `p.area` for a method without parameters (`area := side*side`, a getter).
   - The class keeps only its fields, so construction counts fields as before.
   - A method that assigns a field of its receiver is a loud error until step 2.
   - Pass: lowering/class_methods.rs, early in SOURCE_PASSES.
2. **Mutating methods** (P116): `inc() := n += 1`; `c.inc()` updates the variable c, like `xs.add(v)` does (objects
   are values, wiki/class.md).
3. **Constructors and required fields** (wiki/constructor.md): `value{…}` / `value(name){…}` blocks, `name!` required.
4. **Traits in the class body.** `class dot{x:int; compare(other) := x - other.x} is Comparable`: methods already
   are the free functions traits look for. Mostly free once step 1 lands.
5. **Inheritance** (P117): `class b extends a` copies a's fields and methods into b, b's own definitions override.
6. Observable fields (`on set p.age`) with warp-54 (signals), when fields become signal targets.

## Decisions (user, 2026-10-06, via the Interviewer)
- P115 receiver: bare field names read the receiver's fields; `self` (alias `this`) names it.
- P116 a method assigning its own fields: `c.inc()` updates the variable c, like `xs.add(v)` (step 2).
- P117 `area := side*side` in a class body is a getter, computed at each `s.area`.
- P117 inheritance: yes. `class b extends a` copies a's fields and methods into b; b's own definitions override a's.
  Traits (`is shape`) stay as they are (step 5).

## Step 1 as built
- lowering/class_methods.rs (early in SOURCE_PASSES): each `:=` definition in a class body becomes
  `name(self:class, params…) := body`, and the class keeps its fields. The parser keeps a `:=` body as code, not as a
  field type (lists.rs `transform_fields_to_types`).
- A method name that two or more classes define is declared as the implicit trait `trait has·name{name}`. traits.rs
  then names each class's method `name·class` and picks one by the receiver's static type. `x.name` / `x.name(…)` of
  such a name becomes `name(x, …)` before that happens.
- overloads.rs dispatches on the return type only among definitions with the same parameter types, so methods of
  two classes that return different classes are no return-type overloads.

## Step 2 as built (P116)
- A method whose body assigns a field of its receiver (`n += 1`, `self.n = …`, `x += dx` in a block) gives the changed
  object: its function body is `(body; self)`.
- On a variable, `c.inc()` is the update `c = inc(c)`; on any other receiver (`counter(4).inc()`) the call gives the
  changed copy. The value of `c.inc()` is the changed object.
- Not yet: a method that changes its object and gives another value (a `pop`). Its value is the changed object.

## Step 3 as built (P117 inheritance)
- `class dog extends animal {…}`: the parser keeps the parent as the annotation `@extends(animal)` on the class name.
- class_methods first gives each class its parents' items: animal's fields first (so `dog("Rex")` fills name), then
  dog's own; a field or method dog defines again replaces animal's. A parent that is no class, or a cycle, is an error.
- It declares `dog like animal` (traits.rs likeness), so a dog is accepted where an animal is wanted (`greet(a:animal)`).
- The methods dog inherits or overrides are defined for dog itself, so a name both classes define dispatches per
  class like any shared method.
- library_words: a class's own `sum·point` (any `name·Type` variant) shadows the library word `sum`, so a shared
  method named like a library word stays the class's.

## Ported cases (tests/types/test_class_cases_ported.rs, probe probes/class_cases.sh)
Cases from Python, Java, Kotlin, Swift, C# and Ruby. Batch 1 (branch classes-1) added:
- Implicit receiver (Java/Kotlin/C#): inside a method, `area()` calls the class's own (or inherited) method on self,
  a bare getter name `area` reads it from self; a parameter of the same name shadows both.
- `super.speak(…)` in dog's method: inherit gives dog the speak it would have inherited as its own method
  `speak·super·dog` and the call is `self.speak·super·dog(…)`. Static dispatch, nothing at run time; chains work
  (puppy's super is dog's speak, whose super is animal's). super of a method the parent lacks is an error.
- `dog(…) is animal` is true when dog extends animal (traits.rs IS_TYPE through the likeness); `animal(…) is dog` false.
Already worked: field defaults `x:int=0`, `self.w`, getters, methods with arguments, overriding per subclass, value
equality `==`, mutating methods, field assignment, a method giving a changed copy, nested objects, named construction.
Batch 2 (branch classes-2):
- Static members (P122, user: the explicit keyword): the parser keeps `static` in front of a statement as the
  annotation `@static` (no "no meaning" note any more; before a top-level function it still changes nothing).
  In a class body `static k = 3` is the main-level `global c·k = 3` plus the getter `k(self:c) := c·k`, so `C.k`,
  `c.k` and a bare `k` in methods read the one value and methods may change it (`clicks += 1`); it takes no
  field slot. `static make(x) := …` is the function `c·make`, called `C.make(…)`. A plain `k = 3` stays a field with
  a default.
- A field with a default (`x:int=1`, `k = 3`) is read from self in methods too (it gave the bare symbol before).
- `p.items.add(v)` / `.insert(…)` of a field path update the field like `xs.add(v)` updates a variable
  (declaration_lowering is_place); a method doing it changes its object, so a Stack's `push` works.
- Convention (P123 follow-up): new tests name classes with a capital, `Point{x:1 y:2}`.
Batch 3 (branch classes-3), P123 one text form `Point{x:1 y:2}`:
- key_emitter emit_default_key: an instance keeps its curly bracket info (it was turned into a `[…]` list, so the
  result read `P[x:1 y:2]`); its fields are still emitted as data.
- library_ops list_text (print, interpolation): a key without operator (an instance) joins name and fields without
  the `:` it puts in an entry `a:1` (print wrote `P:[x:1 y:2]`).
- casts: `string(p)`, `"\(p)"`, `p as text` of an instance known only at run time go through list_text too (it was
  "has no runtime text yet").
- Not yet: a text field prints unquoted at run time (`P{x:1 name:a}`, like `print ["a"]` writes `[a]`), so it does
  not read back as the same value; decided P126 (quote texts inside containers, below), not implemented.

## Representation: GC structs (classes-4, wasm_emitter/struct_backend.rs)
Generic form: `P(1, 2)` is a `$Node` key `P` over a cons list of `x:1`, `y:2` entries (type_constructor.rs, the
Instance mark), `p.x` is `struct_body` + `map_find`, a search comparing field-name symbols. The struct backend
replaces that where the static type is known, modelled on map_backend.rs (typed maps) and list_abi.rs:
- `P·instance`: one struct type per class (emit_instance_types), mutable fields, an int field i64, a float field f64,
  any other a Node. Today's immutable `$P` (field_storage "int"→I32) stays for FFI/WIT.
- Struct variables (find_typed_structs): a local whose every assignment is a construction of one class giving every
  field in declared order with values of the field's kind (int/float fields: static kind Int/Float); any other write
  (`p.x = v` too, until step 3), a global, a capture, a parameter or a `RAN_WITHOUT_ERROR` block keeps the Node form
  (names_held_as_nodes, shared with typed maps). `p.x` is struct.get (boxed where a Node is wanted, the bare i64 in
  numeric code); where the instance itself is wanted it becomes the Node `P{x:… y:…}` built from the fields.
- Struct parameters (find_struct_abi): `m(p:P)` takes `(ref null $P·instance)` when its body only reads fields of p
  and every mention of m is a direct call passing a struct variable (passed as it is) or a construction of P
  (straight into struct.new). Any other argument keeps the Node parameter: a duck-typed value may lack fields the body
  never reads (operators::test_like, `keep(p:photo) := p.width` given `{width:3}`). Closures, tuple functions and
  `compare·T` witnesses (runtime dispatch, witness.rs) keep Node parameters (directly_called_functions).
- probes/bench_class_instances.sh, 10^6 iterations, debug build: construct_and_read 248 → 5 ms, read_only
  (`p.x * p.y`) 141 → ~0 ms, method_call (`p.sum()`) 170 → ~0 ms; plain ints 2 ms.
- Decisions (Interviewer warp-33, user, 2026-10-06): P127 int fields stay fast i64; a write that does not fit is a loud
  run-time error "x of Point is an int field: 2^70 does not fit in 64 bits; declare it x:bigint or x:number" (step 3,
  writes). P126 texts inside containers print quoted everywhere (print, interpolation, string()): `P{x:1 name:"a"}`,
  `["a" "b"]`; a top-level `print "a"` still writes a.
Next steps: 3. writes `p.x = v` as struct.set (with the P127 check), 4. methods that change or return self, a struct
result (like list_abi's returns_list), struct elements in typed lists.

Open (next batches): GC struct writes and struct results (above), keyword methods in a class body (`def area() -> int {…}`,
`fun area(): Int {…}`, Python/Kotlin/Swift style: today "no field area"; class_methods method_parts sees only `:=`
heads, reuse declarations.rs keyword_definition on the body items first, warp-dd owns that function), the `value{…}` constructor block and `value(name){…}`
(wiki/constructor.md; today silently ignored), a `pop` method (changes the object and gives another value), a method
named like a type word (`double()`: "double is a type"), property setters (wiki/property.md), generics
`class Box<T>`, mixins, a field named `pi` (card footgun-pi).
