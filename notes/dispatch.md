# One lookup for every word

`x.word` ≡ `x.word()` ≡ `word(x)` ≡ `word x` ≡ `word of x`, all resolved by one lookup: multi-dispatch on the
receiver's type, the most specific meaning wins (decisions default-methods, library-words-typed,
dispatch-untyped-library-words, class-members-free, field-forms, type-words-reserved).

## The rule
A word names a generic function. Its meanings, most specific first:

| level | meaning | examples |
|---|---|---|
| 0 | a variable or parameter in scope (bare words only; `x.word` never reads a variable `word`) | `random = 4; random` |
| 1 | the receiver's own member: field of its class, entry of its literal or map, class method or method typed with its class (own class before a parent) | `{first: 1}.first`, `Deck(3).sort()` |
| 2 | the program's own untyped definition, at the arities it takes | `count(x) := 7; count(5)` |
| 3 | a used module's word (`use json`: bare `parse` is json.parse) | `use json; parse t` |
| 4 | the library word of the receiver's built-in type | `xs.first`, `x.round`, `t.int`, `p.zy` |
| 5 | nothing: `undefined function: word` | `5.shout` |

Exception: typed library words are immutable for their own types, so for a value of those types level 4 comes before
level 2 (`count(x) := 7; count([1,2])` → 2, with a hint).

- Static when the receiver's type is known at compile time; otherwise one run-time choice (`is_type` test of the
  receiver, never of a function argument: testing calls it).
- Forms: a getter is the 0-argument meaning, a setter the 1-argument one; `x.word = v` ≡ `x.word(v)` ≡ `word(x, v)` ≡
  `word of x = v` call the setter when one exists, else store the field. Plain fields answer only `.` and `of`.
- A function word used as a value is its 0-argument call (bare `canvas` is `canvas()`).
- After a number, a unit symbol is the unit, even beside a variable of that name.
- Shadowing is per name and arity, scoped per function: a program `first(x)` leaves `first(xs, 3)` to the library.
- Uniform call: `x.f(args)` calls `f` when f takes `args + 1` arguments (`ProgramNames::takes_method_call`).
- Redefining a built-in type's library word for that type (`first(xs: list) := …`) is a loud error; every type and
  conversion word is refused as a function name ("hex is a type; name your function to_hex").

## Where it lives
- src/lowering/words/library_words.rs `method_call`: OwnMember × WordMeaning.
- src/lowering/words/program_names.rs `ProgramNames` (scoped, per arity): `of`, `takes`, `binds`, `takes_call`,
  `function_takes`, `inside`, `rewritten_where_free`, `rewritten_by_scope`. Ask it instead of building another
  "the program defines X" set. Program-wide on purpose: std_aliases and rust_paths, of_lookups, markup tags, operator
  call forms, type_tests' Names, needed_definitions.
- src/lowering/words/typed_library_words.rs (start of MODULE_PASSES): an untyped redefinition `count(x) := 7` becomes
  `count·any` with a hint, and each call of an arity the program's takes is chosen by the argument types
  (library_parameter_types, MEMBER_SIGNATURES).
- A field write `m.title = v` takes no word unless the program defines the setter; parameters and loop variables are
  of unknown kind; another module's bare word on what may be an object reads its field (`p.center`).
- probes/dispatch/compare_binaries.py: every `is!` snippet of the given test files run with two binaries, results
  compared; a quick behaviour check while the test queue is long.

Open: card parameter-unit (`f(x) := x as ms` loses the conversion at run time).

# Dispatch on return type (D10, P17)

`(render "hello") as pdf` and `docx example = render "x"` pick the overload of `render` by the expected result type;
when it picks no single overload, the first-declared one is taken with a got-it warning naming the explicit form.

## Overloads
Two or more definitions of one name whose result types differ are overloads, in declaration order:
```warp
class pdf{body}
class docx{body}
render(t):pdf  := pdf("%PDF " + t)      // the result type written after the head (also `render(t) as pdf := …`)
render(t):docx := docx("<w:t>" + t)
```
The result type is the written one, else the type of the instance the body constructs. The English phrase names it
after a final `as`: `to render text as pdf: …` is `render(text) as pdf := …` (atoms.rs phrase_result_type).
Definitions of one name whose result types are not all known stay what they were (the last one wins).

## The expected type of a call
| context                         | expected type |
|---------------------------------|---------------|
| `(render "x") as pdf`           | pdf  |
| `docx d = render "x"`, `d:docx = render "x"` | docx |
| `save(render "x")` with `save(d:docx)` | docx (the parameter's declared type) |
| anything else                   | none          |

A function defined once with a known result type takes `(render "x") as pdf` as naming that result, not as converting.
The ungrouped `render "x" as pdf` casts the argument (call-as-grouping). A call whose expected type one overload
returns calls it (`render·pdf("x")`); otherwise the first-declared overload with the got-it warning (topic
`return-type`): `render has variants returning pdf, docx: which does render "x" mean? (taking pdf); fix: (render "x") as pdf`.

## Companions
A function f without overloads but with a companion `f_as(…, type:text)` gets its expected type at run time:
`x:int = agent "33/3"` calls `agent_as("33/3", "int") as int` (lib/agent.warp asks Claude for a JSON value of that
type). modules.rs links `f_as` whenever it links f. Live check: probes/agent_types.sh.

## Lowering
src/lowering/definitions/overloads.rs, right after type_constructor and before the trait passes and library_words.

# Methods: dispatch by argument classes

A function name is a generic function, a set of methods told apart by the classes of their positional arguments.
`sort(d:Deck) := …` adds a method to sort; sort of lists and texts stays the library's. `x.f(args)` is `f(x, args)`.
```warp
class Deck{n:int}; sort(d:Deck) := d.n
Deck(3).sort()     // 3
[3, 1].sort()      // [1 3]: the library sort
class Animal{name:text}; class Dog extends Animal{}
speak(a:Animal) := "..."; speak(d:Dog) := "woof"
speak(Dog("rex"))  // woof: the most specific method wins (a class over its parent, nearest first)
```
- A method's signature is the class of each positional parameter, any where untyped (traits.rs Signature):
  `collide(a:Ship, b:Rock)` is `collide·Ship·Rock`, a first-only method `sort·Deck`. Generic::method_for: per position
  the likeness steps from the argument's class (own 0, parent 1, any value ANY_DISTANCE); the method no other beats at
  every position wins; two left are ambiguous, with the fix "define the exact one".
- Which names: a name with a method for a declared class and either a fallback (a library word, an untyped definition)
  or methods for several classes. A name with one method and nothing else stays a plain function. The fallback counts
  only when it takes the call's arguments (`b.take()` never falls back to `take(xs, n)`).
- Calls (traits.rs `generic_calls`, before library_words): a known class calls its method statically; a literal calls
  the fallback; an unknown class goes through the generated `f·dispatch`, testing `instance_of` per typed position.
- Errors: no method and no fallback: `area has no method for circle; methods: …; fix: define area(s:circle) := …`.
- A field and a method of one name: `b.size` reads the field, `b.size()` and `size(b)` call the method, and the
  definition warns (field_method_warnings).
- A class's own method named like a library word (`pop`) takes calls on its instances only; `items.pop()` on an
  untyped field stays the library word.
- Across modules: a std module's method like shell's `sort(file:Path)` joins the list word's generic function.

## One method form (one-method-form)
`class P{def x(k):=…}` and `def x(p:P, k):=…` are one method: `p.x(k)`, `x(p, k)`, `x p`, `&x`, methods(P), dir(p)
behave alike. A class-body method lowers to `x(self:P, k)`; an outside one keeps its place and gets the class-body
rules: bare fields read the receiver unless a parameter has the name (class_methods/outside_methods.rs); a type word
as the name is renamed `method·double`; methods(P) lists every function whose first parameter is declared P or an
ancestor. Moving outside definitions into the class body was tried and changed dispatch errors, traits and compiled
modules' exports, so the outside form stays where it is.
Open: a trait's functions (`f(s:Shape)` is untyped before reflection reads it), `.declared_in` metadata.
