# Classes with methods (issue #14, card g-1nug)

Roadmap card: class-system (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

## Overview
- `class`/`struct`/`type`/`record` declare a type: fields, typed fields (`age:int`), optional fields (`left?`),
  defaults, required fields (`name!`: "P needs field name"). Construction: positional `person("Ann" 40)` or named
  `person{name:"Ann"}`; `init{…}` / `init(name){…}` blocks are the constructor (P162). Field access `p.age`.
- Methods are functions dispatching on their first argument's class: a method adds to the function of that name, the
  most specific wins, at run time when needed (notes/dispatch.md "Methods"). Written as free functions over the type
  (`greet(p:person) := "hi " + p.name`, called `greet(p)` or `p.greet()`) or inside the class body (step 1).
- P115 receiver: in a class body bare field names read the receiver's fields; `self` (alias `this`) names it.
- P117 `area := side*side` in a class body is a getter, computed at each `s.area`.
- P116 a method assigning its own fields: `c.inc()` updates the variable c, like `xs.add(v)` (step 2; objects are
  values, wiki/class.md).
- P117 inheritance: `class b extends a` copies a's fields and methods into b; b's own definitions override a's (step 3).
- Traits (wiki/trait.md): Comparable, Equatable, declared traits; conformance is structural, by the class's methods
  (`class dot{x:int; compare(other) := x - other.x} is Comparable`), a claimed one checked (P177, below).
- Fields are signal targets: `on change p.age {…}` hears `p.age = 3` (notes/signals.md).

## Step 1 as built
- lowering/types/class_methods.rs (early in SOURCE_PHASES): each `:=` definition in a class body becomes
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
- A method that changes its object and gives another value (`pop() := items.pop()`, a block ending in a value
  `dequeue() := { first = items#1; items = items[1:]; first }`) gives the pair [value, object]; `q.dequeue()` stores
  the object back and gives the value (classes-36: a block's statements run before the value, not as data).
  A change inside `if`/`else` gives the object unless a branch ends in a value.
- `p.counts#i = v` / `+=` of a list field: `counts·elements = p.counts; counts·elements#i = v; p.counts = …`
  (lowering field_elements, functions), so a method setting an element changes its object; also outside classes (`s.xs#2 = 6`).
- init(x) := … is the constructor like init(x){…}; what a loop in init sets (`for x in xs { i = … }`) stays local.

## Step 3 as built (P117 inheritance)
- `class dog extends animal {…}`: the parser keeps the parent as the annotation `@extends(animal)` on the class name.
- class_methods first gives each class its parents' items: animal's fields first (so `dog("Rex")` fills name), then
  dog's own; a field or method dog defines again replaces animal's. A parent that is no class, or a cycle, is an error.
- It declares `dog like animal` (traits.rs likeness), so a dog is accepted where an animal is wanted (`greet(a:animal)`).
- The methods dog inherits or overrides are defined for dog itself, so a name both classes define dispatches per
  class like any shared method.
- library_words: a class's own `sum·point` shadows the library word `sum` only for an arity overload or a trait
  operation; a method named like a library word is a method of the generic function `sum` (notes/dispatch.md), and
  `sum([1, 2])` stays the library's.

## Ported cases (tests/types/test_class_cases_ported.rs, probe probes/class_cases.sh)
Cases from Python, Java, Kotlin, Swift, C# and Ruby.
- Implicit receiver (Java/Kotlin/C#): inside a method, `area()` calls the class's own (or inherited) method on self,
  a bare getter name `area` reads it from self; a parameter of the same name shadows both.
- `super.speak(…)` in dog's method: inherit gives dog the speak it would have inherited as its own method
  `speak·super·dog` and the call is `self.speak·super·dog(…)`. Static dispatch, nothing at run time; chains work
  (puppy's super is dog's speak, whose super is animal's). super of a method the parent lacks is an error.
- `dog(…) is animal` is true when dog extends animal (traits.rs IS_TYPE through the likeness); `animal(…) is dog` false.
Also covered: field defaults `x:int=0`, `self.w`, getters, methods with arguments, overriding per subclass, value
equality `==`, mutating methods, field assignment, a method giving a changed copy, nested objects, named construction.
- Static members (P122, the explicit keyword): the parser keeps `static` in front of a statement as the
  annotation `@static` (before a top-level function it changes nothing).
  In a class body `static k = 3` is the main-level `global c·k = 3` plus the getter `k(self:c) := c·k`, so `C.k`,
  `c.k` and a bare `k` in methods read the one value and methods may change it (`clicks += 1`); it takes no
  field slot. `static make(x) := …` is the function `c·make`, called `C.make(…)`. A plain `k = 3` stays a field with
  a default.
- A field with a default (`x:int=1`, `k = 3`) is read from self in methods too.
- `p.items.add(v)` / `.insert(…)` of a field path update the field like `xs.add(v)` updates a variable
  (declaration_forms is_place); a method doing it changes its object, so a Stack's `push` works.
- Convention (P123 follow-up): new tests name classes with a capital, `Point{x:1 y:2}`.
P123 one text form `Point{x:1 y:2}`:
- key_emitter emit_default_key: an instance keeps its curly bracket info; its fields are emitted as data.
- library_ops list_text (print, interpolation): a key without operator (an instance) joins name and fields without
  the `:` it puts in an entry `a:1`.
- casts: `string(p)`, `"\(p)"`, `p as text` of an instance known only at run time go through list_text too.
- P126: a text inside a container prints quoted (`P{x:1 name:"a"}`, `["a" "b"]`, `{a:"x"}`) in print,
  interpolation and string(); a text on its own still prints bare. list_text quotes text and character items;
  emit_dynamic_text passes a lone text or character through unquoted; `string([...])` of a literal list of numbers and
  texts takes the same runtime text. A `"` or `\\` inside such a text is escaped, including in computed texts,
  and Node::serialize escapes them the same (list-text-quotes).

## Representation: GC structs (classes-4, wasm_emitter/struct_backend.rs)
Generic form: `P(1, 2)` is a `$Node` key `P` over a cons list of `x:1`, `y:2` entries (type_constructor.rs, the
Instance mark), `p.x` is `struct_body` + `map_find`, a search comparing field-name symbols. The struct backend
replaces that where the static type is known, modelled on map_backend.rs (typed maps) and list_abi.rs:
- `P·instance`: one struct type per class (emit_instance_types), mutable fields, an int field i64, a float field f64,
  any other a Node. Today's immutable `$P` (field_storage "int"→I32) stays for FFI/WIT.
- Struct variables (find_typed_structs): a local whose every assignment is a construction of one class giving every
  field in declared order with values of the field's kind (int/float fields: static kind Int/Float), and field writes
  `p.x = v`, `p.x += v`, `p.x++` (struct.set) of values of the field's kind; a text or list written to a number field
  is a compile-time type error ("x of Point is an int field, got [1] (a List)"). Any other write, a global, a
  capture, a parameter or a `RAN_WITHOUT_ERROR` block keeps the Node form
  (names_held_as_nodes, shared with typed maps). `p.x` is struct.get (boxed where a Node is wanted, the bare i64 in
  numeric code); where the instance itself is wanted it becomes the Node `P{x:… y:…}` built from the fields.
- Struct parameters (find_struct_abi): `m(p:P)` takes `(ref null $P·instance)` when its body only reads fields of p
  and every mention of m is a direct call passing a struct variable (passed as it is) or a construction of P
  (straight into struct.new). Any other argument keeps the Node parameter: a duck-typed value may lack fields the body
  never reads (operators::test_like, `keep(p:photo) := p.width` given `{width:3}`). Closures, tuple functions and
  `compare·T` witnesses (runtime dispatch, witness.rs) keep Node parameters (directly_called_functions).
- Struct results (classes-9, struct_results): a method that changes its object (`inc() := n += 1`, lowered
  `inc(self:C) := (self.n += 1; self)`, called `c = inc(c)`) writes the struct's fields in place and gives it back
  when every call is `x = f(…, x, …)` replacing the struct variable x it passes once: nobody else holds that struct,
  so value semantics hold (`d = inc(c)` keeps the Node form). Decided by a greatest fixpoint (a variable is a struct
  only while the functions it is assigned from give structs).
- probes/bench_class_instances.sh, 10^6 iterations, debug build: construct_and_read 248 → 5 ms, read_only
  (`p.x * p.y`) 141 → ~0 ms, method_call (`p.sum()`) 170 → ~0 ms, field_write (`p.x += i`) 16 ms, changing_method (`c.inc()`) 32 ms; plain ints 2 ms.
- P127 int fields stay fast i64, a value that does not fit a loud run-time error. Nothing overflows: an i64 field carries warp's exact-int encoding like an int local
  (fixnum or a handle to the big number), so `b.n = 2^70` keeps 2^70 (an_int_field_holds_any_int). 0.5 is an exact number of kind Int; a declared int variable or a struct int field traps
  at run time with "an int must be a whole number" when given a fraction (`x += 0.5`, `p.x = y`), as wiki/Footguns.md
  refuses `x:int=5; x=2.5` (big_int.rs emit_fits_declared, struct_backend.rs emit_field_value); `/=` still keeps an int
  an int. Open: a Node instance `P(0.5)` (card int-field). P126 texts inside containers print quoted everywhere (print, interpolation, string()): `P{x:1 name:"a"}`,
  `["a" "b"]`; a top-level `print "a"` still writes a.
Upcast fields (P201/P203, card upcast-field, analyzer/upcast_fields.rs, on the source before class_methods):
`s: Shape = Circle(…); s.r` (also a parameter `f(s: Shape)`, a variant through its sum type) is the compile error
"Shape has no field r, only Circle has: match … or read (s as Circle).r". Checked only when a descendant has the
member, so library methods and unknown names stay run-time lookups; a name tested in its scope (`if s is Circle`) is
smart-cast and skipped. `s as Circle` is the checked downcast `(cast·value = s; if not (cast·value is Circle) raise …;
cast·value)` (class_methods checked_cast), "s is no Circle".
Objects are references (P200 instances, P200b maps and new fields; cards real-references, shared-maps): $Node's kind,
data and value are mutable; `p.x = v` still lowers to `p = field_with(p, "x", v)`, but the runtime field_with
(library_ops.rs) sets an existing entry in place (field_set_in_place) or grows the fields in place (fields_grow: ø
becomes the entry, a single entry a cons list, a new field goes in front of the meta entries) and gives back the same
object, so every holder sees it: aliases (`q = p; q.x = 7`), list items (`xs = [p]`), parameters and results of
functions, for instances and maps alike. struct_backend.rs and map_backend.rs keep a variable on the struct or hash-table
backend only while it is never used as a whole value (used_whole: `q = p`, `[p]`, print(p), an argument to a non-struct
parameter make it the shared Node); field writes and a bare final `p` do not count. A map parameter becomes a hash-table
copy (`m·map = m`, declaration_forms.rs keys) only when the function never sets an entry of it. The old
copy-in/copy-out pass (lowering/shared_instances.rs) is gone.
Copies (P205/P207, runtime instance_copy(object, shallow)): `x.copy()` is deep (deep_copy with a memo of
`original:copy` pairs, so a part held twice or a cycle is copied once; numbers, texts, closures and data are shared),
`x.copy(shallow: true)` makes only the object's own entries and cells new (copy_spine). The class keeps its type; a
class's own `copy` method wins, `clone` is an alias that always calls it (METHOD_ALIASES), Kotlin
`p.copy(y = 5)` copies then sets (class_methods copies, `shallow = yes` among its arguments is the flag).
Lists share too (P200b, card shared-lists): add (`xs += [v]`, list_extend), pop (list_drop_last), remove (map_without,
maps alike), insert (list_insert_at) and `xs#i = v` change the list in place; `+` (list_concat) makes a new one; typed
lists share their array (notes/typed_lists.md). Open: a list held by a field or item does not grow in place yet
(card shared-lists-follow); P206's got-it note when a copy shares a resource (copies share closures and data silently).
Next steps: struct elements in typed lists (`for p in points`), a struct result of a construction inside a function
(`moved(dx) := point(x + dx, y)`).

Keyword methods (classes-6, card classes-keyword): `def area() -> int {…}`, `fun area(): Int {…}`,
`func area() -> Int {…}`, `def scaled(k) {…}` in a class body are methods: class_items feeds each item through
declarations.rs keyword_definition (warp-dd's) before splitting methods from fields, the result type stays on the
method (dropped when the method changes its object and so gives it back); the parser no longer reads a keyword
method's body as field types (warp_parser transform_fields_to_types: `k` of `side * k` was `type k`).

Constructor block (classes-7, wiki/constructor.md): `init{ id = random() }` or `init {…}` in a class body is the
function `P·init(self:P)` (class_methods::constructor_name), its field names read and set on self, giving self; a
field only it sets is an optional field (ø until it runs). type_constructor passes every construction `P(…)`, `P{…}`
of a class that has one through it, after the given fields are matched and defaults filled.
`init(name){…}` (classes-10) is `P·init(self:P, name)`: a call `P(a)` with as many arguments as it has parameters
passes them to it, on an instance of the declared defaults (ø for the other fields), instead of matching them to
fields; a field it sets that is no parameter becomes a field.

Open (next batches): struct elements of lists (above), a method
named like a type word (`double()`: "double is a type"), mixins, a field named `pi` (card footgun-pi, P130: the class's own field), 
Extension methods (classes-11, card functions-extension, declarations.rs): Kotlin `fun Int.twice() = this * 2` and
Swift `extension Int { func twice() -> Int { self * 2 } }` define `twice(this:Int)` (`self` when the body says self),
so `3.twice()` calls it as a method like any function whose first parameter is the receiver.

Generic classes (classes-12, warp_parser atoms.rs): `class Box<T>{item:T}`, `class Pair<A, B>{…}`: a field of a type
parameter is `any` (a Node field), a method parameter of one is untyped; `Box<int>(3)` and `Box<int>{item:3}`
construct a Box (the type arguments of a declared class are read past, not checked yet).

Methods named like a type word (classes-13, P142: class methods are always allowed): `double() := x*2` in a class
body is the method `method·double` (class_methods renamed_type_word_methods), its calls `c.double()`, `c.double` and
`double()` in the class body renamed too; `double(3)` and `3.double()` stay the conversion.

Properties (classes-14, wiki/property.md): `get age() {…}` is the getter `age := …` (read `p.age` like a field),
`set age(v) {…}` the method `age·set(self, v)`; the warp form `age:{2026 - birthday} set{birthday = 2026 - it}` both,
`it` the new value. `p.age = v` of a variable p runs it: `p = age·set(p, v)` (class_methods setter_calls).

Mixins (classes-15): `mixin Walker{steps:int=0; walk() := name + " walks"}` declares no class but items classes take
in: `class Duck with Walker, Swimmer {…}` (Dart/Scala) or `include Walker` in the body (Ruby). class_methods
with_mixins appends the mixin's fields and methods after the class's own (so `Duck("Don")` fills the own fields
first), leaving out those the class defines itself; the methods read the class's fields.

Methods that change their object and give a value (classes-16, card obj-list): `pop() := items.pop()` returns the
pair `[pop·value, self]`; `s.pop()` of a variable is `(pop·result = pop(s); s = pop·result#2; pop·result#1)`
(class_methods Change::GivingValue; a changing method ends in a value unless its last statement assigns or appends).
A method named like a list mutation (pop, push, add, remove, insert, append) is renamed `method·pop` like a type-word
method, but called so only on what holds an instance (a variable assigned one, `p:Stack`, self) or bare in its class
body: `items.pop()` and `xs.pop()` of lists stay list mutations. `s.xs.pop()` pops a list field (list_mutations
popped_list takes any place, as add/insert did).

Instances held as Nodes (classes-17): a field read by name of an instance that is no struct variable (`for p in ps
{ s += p.x }`) no longer searches with a made symbol: struct_body knows an instance by its own op code (Op::None,
D4) without comparing type names, and instance_field (list_ops.rs) walks the fields comparing the string-table
offset and length of the name the program wrote (the table deduplicates) before the general map_find, which still
answers runtime-built instances and meta entries `@name`. The target is evaluated once. bench list_of_instances
(10^6 points, build + two field reads each): 3098 → 151 ms.

Tagged objects (classes-18, card person-name): `Person{name:"A"}` and now also spaced `Person { name: "Alice" … }`
(a capitalized word of no declared type before a block of fields, the README example; fields may start on the next
line) are the data key `Person:{…}` (D4: no class, no construction). A field read `p.name` that finds no entry `name`
on the node itself reads the tagged object's fields (list_ops tagged_field, between the entry and the meta entry
`@name`), so `a:{b:1}.b` is 1 and `a:{b:1}.a` still `b:1`. Card class-ticket (a static in value{}) worked already
after classes-10; its test is in test_tagged_objects.rs.

Spaced children (card spaced-child): inside a data literal (the block of `a{…}` or `Person {…}`, not a declared type's
constructor) a spaced `c { d:3 }` is the child node of the glued `c{ d:3 }`, whatever its block holds: data has no
call with a block (parser flag in_data_literal, atoms.rs). In code `run { … }` stays a call with a block.
`x = a{ b:2 c { d:3 } }; x.c.d` is 3 (tests/parser/test_spaced_children.rs).

## Ported forms (classes-19, tests/types/test_class_forms_ported.rs)
The same class as other languages write it (parser atoms.rs, class_methods.rs class_items):
- Kotlin: primary constructor `class Point(val x: Int, var y: Int = 0) {…}` (its parameters are the fields, the
  declaration ends at its line without a body), `data class` / `open` / `abstract` (CLASS_MODIFIERS: a warp class
  compares by value already), expression bodies `fun sum() = x + y`.
- JavaScript: `constructor(x, y) {…}` and `sum() {…}` members, `new Point(1, 2)` of a declared class.
- Python: `class Point:` with an indented body, `class Dog(Animal):` (parents in parentheses, `object` none),
  `def __init__(self, x, y):` the constructor, explicit `self` first parameters dropped, `@dataclass` fields.
- Swift: `struct` with `var count = 0` fields and `mutating func` (MEMBER_MODIFIERS dropped), `init(…)`.
- C# (classes-20): positional `record Point(int X, int Y);` (`int X` the field X:int), `new Point(1, 2)`.
- Rust (classes-20): `impl Point { fn sum(&self) -> i32 {…} }` adds its functions to the class Point (class_methods
  with_impls; `impl Trait for Point` too), `&self` is the receiver.
- Kotlin `p.copy(y = 5)` is `field_with(p, "y", 5)` (class_methods copies).
Constructors of all of them are `init(params){…}` (classes-10, P162). A method named like a library word (`sum`, `count`)
is a method of the generic function `sum` (card dispatch, notes/dispatch.md; formerly renamed `method·sum`, classes-16). - classes-21: Java's typed fields `int x;`, a constructor named like the class `Point(int x, int y) {…}`, C-style
  methods `int sum() {…}`, `Point p = new Point(3, 4);` (`p:Point = …`, the type kept for D10 dispatch);
  TypeScript `twice(): number {…}`; C# auto-properties `int X { get; set; }` and the object initializer
  `new Point { X = 3 }`; Python class attributes (a field with a value read as `Counter.count` is static).
  Every class body is normalized to its members (with_members), not only those with methods. The declared-type
  pre-scan takes the name right after `class`/`record`/`type` (`record = find(…)` declares nothing).
Also: Ruby (`attr_accessor`, `initialize`, `@x`, `end`, `Point.new`, endless
`def twice = @x * 2`), Go (`type P struct {…}`, `func (p P) M()`), operators (`__add__`, `operator +`, class_operators.md),
a method named `norm` (tests/functions/test_operator_word_methods.rs).

Smart scopes (classes-22, wiki/inventions.md, declarations.rs smart_scope): `Number { Square = it*it }` defines
`Square(self:Number) := self*self` for a builtin type word, `it` the value the method is called on: `3.Square`.

Operators on instances (classes-23, wiki/operator.md "a & b will try to invoke et, and, add"): `a + b` of an
instance whose class defines `plus` (or `add`, Python `__add__`, Kotlin `operator fun plus`) is `a.plus(b)`; also
`-` minus, `*` times, `/` divide, `%` mod, `<` less, `>` more, `==` equals (class_methods OPERATOR_METHODS). Known
instances only (constructions, annotated or constructed variables, loop variables, chains `a + b + c`); an unknown
operand keeps the built-in operator (card class-method: dispatch at run time). A method named by its glyph
`+(o) := …` and C++/C#'s `operator +(o) := …` (classes-25: the parser's try_parse_operator_method_head reads a glyph
before `(…) :=` as the head, as_member renames it to the english name).
Foreign spellings are aliases (classes-24, alias rule in notes/agents/common.md): they work and give a got-it note
with an "I meant: <warp word>" fix, `diagnostic::note_alias(written, warp_word)` (educate_once, topic
`alias-<foreign word>`): `__add__`/`add` → `plus` (every non-first name in
OPERATOR_METHODS), `data class`/`open class`… → `class`, `mutating func` → `func`, `val x`/`var count` → the field
(`async` stays silent: it may mean something in warp), `new Point(1, 2)` → `Point(1, 2)` ("new is superfluous").
Tests: tests/types/test_class_aliases.rs.
P162 (classes-26): `init` is the constructor. Aliases with the note "warp says init": `value` (the
first warp name), `constructor`, `__init__`, `initialize`, `__construct`, `New`, `Create`, `new`, and a method named
like its class (class_methods with_init_constructors, CONSTRUCTOR_ALIASES). An alias the program also calls as a
method (`p.new(2)`, Rust's `Point::new(1, 2)`: method_calls_named) stays a method. The constructor function is
`P·init`.

Run-time dispatch (classes-27, card class-method): a library-word method (`sum`, the counting words `count`/`size`/…,
type words) called on a variable of no known class outside the class bodies, `total(x) := x.sum()`, is
`if x is Bag then x.method·sum() else x.sum()` (class_methods dispatched_by_class, one branch per class defining it).
A list mutation (`x.pop()`) on an unknown receiver dispatches the same way (card mutation-unknown; the list's branch
still writes the change back to x); inside
class bodies receivers are fields of declared types. An operator on a variable of no known class
(`for q in [V(1), W(9)] { q + q }`, card class-operators) is `if q is V then q.plus(q) else … else q + q` per class
defining the operator's method (operators.rs dispatched_operator); variables assigned a number or a text, looping over
a range or a list of them, and fields of a type word (`x:int`) keep the plain operator (plain_variables). A choice
between an instance (or a map) and a number is a Node of its run-time kind (inference.rs branches_kind, so
`print(if c then V(7) else 5)` joins).

Go (classes-28, tests/types/test_class_forms_ported.rs a_go_struct_with_methods): `type Point struct { X int; Y int }`
(the parser skips `struct` after the name), fields `X int` (typed_field takes both orders, `X, Y int` gives an
untyped X), methods `func (p Point) Sum() int {…}` and pointer receivers `func (p *Point) Move(dx int) {…}`
(class_methods go_method, taken in like Rust's impl blocks, the receiver read as self). A class's own method named
like a LINQ word (`Sum`) stays the class's (welcome_forms linq_calls skips defined_names). `p := Point{…}` is charged
(P138): reading works, a changing method needs `var p = …`. P167: positional braces `Point{1, 2}` of a declared
class build `Point(1, 2)` with a note (class_methods positional_braces); of an unknown name they stay tagged data.

Ruby (classes-29, tests/types/test_class_forms_ported.rs a_ruby_class_with_initialize): `class Point` with indented
lines up to `end` (atoms parse_type_declaration_body, the methods' `end` lines dropped: without_end_lines),
`attr_accessor :x, :y` (also reader/writer) the fields x and y with a note, `@x` in a class body `self.x`
(lookahead reads_instance_variable: before an operator, the statement end, or a line end with `end` next; an
annotation `@deprecated fun f()` stays one), `def initialize(x, y)` the constructor (P162), `Point.new(1, 2)` the
construction `Point(1, 2)` with a note (class_methods ruby_constructions, unless the class defines `new`). Parser fix
on the way: a call followed by an indented block keeps its parameters (`def f(x)` + lines; lists.rs dropped them).

Wiki gaps (classes-31): class.md's nested block of fields `address { street; city; zip? }` declares the field
address holding them (class_methods nested_fields). struct.md's example works. property.md is a sketch with open
questions (`age:{date - 1996}` getters of data, setters): nothing ported from it.

Interfaces (classes-32): class methods satisfy traits, trait-typed parameters, foreign interface/protocol forms and
`implements`/`: Shape` lists: notes/traits.md "Classes and foreign interfaces".

Enums and sealed classes (classes-33, tests/types/test_enums_ported.rs): a warp enum is the sum type of its cases,
each case its own value (lowering/types/plain_enums.rs, below). Ported: Swift's `enum Direction { case north, south }` and
`switch d { case .north: 1 }` (plain_enums reads `.north` and the arm `case.north` as `north` when one enum has the
case), Kotlin's `enum class` (a note), `when (c) { A -> 1; 1, 2 -> …; is T -> …;
else -> … }` (welcome_forms when_chain: an if chain, `is T` the type test; also without a subject), Rust's
`Color::Green` (plain_enums). Kotlin's `sealed class Shape` (a class without fields: a bodiless class is `{}` now) and
`class Circle(val r: Int) : Shape()` (the superclass with `()` is the parent, as extends). Open (a design question):
cases with values, Rust's `Circle(f64)`, Swift's `case circle(Double)` and their destructuring patterns.

Text, equality and order as methods (classes-34, tests/types/test_class_witnesses.rs): a class's `text()`,
`equals(o)` and `compare(o)` are the witnesses text·P, equals·P, compare·P (class_methods with_witness_methods types
the other instance `o:P`; they are no type-word methods). Aliases with a note: toString, to_s, __str__, ToString,
String (Go) → text; Equals, __eq__, equal → equals (`==` is no operator method any more: Equatable dispatches it);
compareTo, CompareTo, cmp → compare. `P·init(P{…}, 3)` has P's shape for the trait passes (traits::shape).
Not ported: hashCode/__hash__ (warp has no hash witness; they stay plain methods), Swift's `description` property.

## Instances and json (classes-37, tests/types/test_class_json.rs)
- `to_json(p)` in a program declaring classes is `to_json_of_classes(p, ["Point", …])` (class_methods
  lower_json_classes, after std_aliases so `JSON.stringify(p)` / `json.dumps(p)` count): both adapters (std_adapters.rs
  without_class_tags, host.js withoutClassTags) turn an object `{"Point": {…}}` of a class name into its fields, also
  nested and in lists. At run time an instance and tagged data `html{…}` look alike, hence the names; a map entry named
  like a class (`{Point: {…}}`) loses its key too. Kotlin `Json.encodeToString(p)` is to_json, `@Serializable` is
  ignored, Python `dataclasses.asdict(p)` is p itself (its fields read like entries), each with a note.
- `object as Point`, `parse_json(t) as Point`, `Point.from_json(t)`: `(Point·from = object; Point(Point·from.x, …))`,
  a field of a class type built from its path `Line(Point(Line·from.a.x, …), …)` (class_methods from_objects); a
  statement sequence as a construction argument is data, so no temporaries below the top. `parse_json(t) as [Point]`
  and a field `points:[Point]` map each element (`list.map(Point·element => Point(…))`); such a list is built before
  the construction, into `elements·1`, … (built_instance).

## Destructuring instances (classes-38, tests/types/test_class_destructuring.rs)
class_methods destructurings: `{x, y} = p` and `{x: a} = p` by field name (maps too); `(a, b) = p` and Python's
`a, b = p` of a known instance (a variable assigned a construction, `p:Point`, or a construction) in field order, a
count other than the fields' left as it was; match arms `Point{x, y} => …` and `Point(x, y) => …` the guard
`parts·from if parts·from is Point` binding the fields before the body. All go through `parts·from`. Not built: nested
patterns, constants inside a pattern (`Point(0, y)`), rest patterns.
Nested and constants (classes-39): a pattern is matched by matched_pattern into class tests (`p is Point`), value
tests (`p.x == 0`) and bindings with paths (`y = parts·from.start.y`): `_` matches anything, a name binds, a number or
text is compared, `Point{x: 0, y}`, `Point(a, 0)`, `Line{start: Point{x, y}, end}` nest. The arm's guard is all tests
joined with `and`; an assignment takes names only (a class in it no test, a constant leaves it as written).
positional_braces leaves an arm's pattern alone (no "prefer Point(x, y)" hint there). Rest patterns (card rest-patterns,
tests/control/test_rest_patterns.rs): Rust's `Point{x, ..}` leaves the other fields out, `Point(x, ..)` takes the first
fields before the `..` and the last ones after it; lists in switch.rs: `[first, ...rest]` (`*rest`, `rest...`) binds the
rest as a slice, `[.., last]` counts from the end.

## Class components (classes-40, tests/web/test_class_components.rs; notes/web_framework.md step 4)
A class with a `render()` method (aliases `view`, `template`, `build` with a note) is a component: its construction
as a child of an element (`div{ Greeting("Ann") }`) or as the program's value is `Greeting("Ann").render()`
(class_methods rendered_components); anywhere else it stays an instance. Function components, props, the trailing
block as children and per-instance state are web's (blocks.rs lower_trailing_blocks, component_state.rs). A method body's templates
(`"#\(n)"`) are lowered before receiver_reads, so a field in a hole is read from self.

## Field names that are words (classes-41, card classes-field, tests/types/test_class_field_names.rs)
- An operator word directly before a key colon names the key: `{from:a to:b}`, `{is:1 or:2}` (parser
  `word_names_key` in warp_parser/lookahead.rs); `then:`, `else:`, `do:` stay operators (BLOCK_COLON_WORDS).
- `l.start` beside a user function `start` reads the field when start is a declared class field and l is a parameter
  or of known instance shape (library_words::method_call); before, it became `start(l)` and recursed forever.
- Without any class (card classes-function): `p.name` inside the function `name(p)` reads the field, since the call
  would recurse with the same argument forever (library_words `defining`, `reads_own_parameter_field`).

## Sum types and enum cases (P179, P178; tests/types/test_variant_payloads.rs, test_enum_cases.rs)
- `type Color = red | rgb(int, int, int)` (parser parse_sum_type): Color is a class without fields carrying
  `@variants(red …)`, each variant with payload a class `@extends(Color)` with fields value / value1… or named
  `rgb(r, g, b)`. Positional fields `c#1` (1-based) or `c.0` (Rust) work on any class instance.
- lowering/types/sum_variants.rs (SOURCE pass after soft_keywords): `x is Color` → `is_type(x, Color) or x == red …`,
  `red is Color` is true; `Shape::Circle(2)` → `Circle(2)` for any variant, also in match patterns.
- `enum Shape { Circle(r), Rect(w, h), Dot }` and Swift's `case circle(radius: Double)` lines: as soon as one case has
  values the enum IS that sum type (parser parse_enum_with_values, same sum_type builder). An enum without values
  is the sum type of its cases too (decision enum-case B, lowering/types/plain_enums.rs, test_plain_enum.rs):
  `Coin.Dime` is `Dime`, `Dime is Coin`; `Coin.Dime as int` its index 2 (counting from 0, as C does; warp's
  positions `x in xs`, `c#1` are 1-based), `as index` the same, `as position` 3 (spec: wiki/enum.md); `2 as Coin` and `Coin(2)` the case, "9 is no Coin case" past the cases;
  `f(1)` for `f(c: Coin)` is a type error offering `f(1 as Coin)` (card coin-errors, test_fixits). Enums with
  written values (`Penny = 1`) are on hold (WIT enums carry no values, FFI can work around it; branch enum-coin). A match on a declared enum or sum type that misses
  a case does not compile (match_exhaustive.rs).
- Open: a match on a subject statically known as a bare variant (`s = Dot; match s {Circle(r) => r*r …}`) types the
  dead arm and fails (card match-static); inside a function it works.
- Emitter fix found on the way: `((a) or b) or c` took the parenthesized operation as a truthy literal and dropped c
  (arithmetic.rs is_operation).

## Checked conformance claims (P177; tests/types/test_conformance_claims.rs)
`class Square implements Shape, Named {…}`, Swift's `struct Square: Shape {…}`, Kotlin's `class Square(…) : Shape {…}`:
the parser (skip_conformances) puts the named traits on the type's name as `@implements(Shape Named)`; traits.rs
named_claims checks each declared trait like the claim `class Square{…} is Shape`: a missing operation is the compile
error "Square claims Shape but defines no area(s:Square)". A name that is no declared trait (a parent class written
`: Base`) is not checked. The old "warp needs no list of them" note is gone. Found on the way: a trait written
`interface Shape { fun area(): Int }` on one line took `fun` as an operation; function keywords are dropped there.
Shared lists (card shared-lists, not started): lists are still values. All list changes lower to `place = new value`
(`ys.add(2)` → `ys = ys + [2]`, pop → slice, `ys#1 = 5` → list_with_at copies the cells), and list_dispatch.rs typed
lists copy on alias by design ("ys = xs copies it when either variable is updated"). Sharing needs: in-place runtime
append/set/pop/remove/insert on Node cons lists (ø grows like fields_grow), add lowered to a form other than
`ys = ys + [v]` (Python: `+` makes a new list, `+=` extends in place), typed $IntList/$FloatList locals aliased to one
struct and kept typed only while never used whole (as map_backend does).
