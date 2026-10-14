# Dispatch on return type (D10)

User decision (P17, notes/open_decisions.md): "Dispatch on return type" (un-parked). `(render "hello") as pdf` and
`docx example = render "x"` pick the overload of `render` by the expected result type; when the expected type picks no
single overload, the first-declared one is taken with a got-it warning naming the explicit form.

The traits work (notes/traits.md) picks an operation by the type of its first argument; this picks a function by the type
its caller expects back. Both rename the variants to `name·T` (traits::witness_name) and resolve calls at compile time.

## Overloads
Two or more definitions of one name whose result types differ are overloads, in declaration order:
```warp
class pdf{body}
class docx{body}
render(t):pdf  := pdf("%PDF " + t)      // the result type written after the head (also `render(t) as pdf := …`)
render(t):docx := docx("<w:t>" + t)
```
The result type is the written one, else the type of the instance the body constructs (`render(t) := pdf(…)`).
The English phrase names it after a final `as` (card render-as): `to render text as pdf: …` is `render(text) as pdf := …`
(warp_parser atoms.rs phrase_result_type); an `as` followed by more words stays a parameter word.
Definitions of one name whose result types are not all known stay what they were (the last one wins).

## The expected type of a call
| context                         | expected type |
|---------------------------------|---------------|
| `(render "x") as pdf`           | pdf  |
| `docx d = render "x"`, `d:docx = render "x"` | docx |
| `save(render "x")` with `save(d:docx)` | docx (the parameter's declared type) |
| anything else                   | none          |

A function defined once with a known result type takes `(render "x") as pdf` as naming that result, not as converting.
The ungrouped `render "x" as pdf` casts the argument, render("x" as pdf), with the warning argument-cast (user decision
B, card parameter-named-number: `f x as T` is f(x as T) everywhere).
A call with an expected type that one overload returns calls it (`render·pdf("x")`; the `as pdf` is then redundant and
dropped). Otherwise the first-declared overload is taken with the got-it warning (diagnostic::ask, Fallback::Warning,
topic `return-type`): `render has variants returning pdf, docx: which does render "x" mean? (taking pdf); fix: (render "x") as pdf`.

### Companions (card int-let)
A function f without overloads but with a companion `f_as(…, type:text)` gets its expected type at run time:
`x:int = agent "33/3"` calls `agent_as("33/3", "int") as int` (lib/agent.warp asks Claude for a JSON value of that type
and parses it). The `as T` stays for a built-in type word, since the companion's answer has no static type (`yes` shows as yes,
not 1). Companions are found by operation (`agent·1` of lower_arity_overloads is `agent`), and modules.rs links `f_as`
whenever it links f, because the call to it only appears here. `(agent "is 7 prime") as bool` works too. Live check:
probes/agent_types.sh.

## Lowering
src/lowering/definitions/overloads.rs, right after type_constructor (constructions are instances by then) and before the trait passes and
library_words, so `x = (render "x") as pdf; x.body` reads a field: traits::InstanceTypes knows the result shape of a user
function from its body.

# Methods: dispatch on the first argument's class (card dispatch, user 2026-10-12)

User: "a more Julia-like approach where we just have functions and if the function takes the class as argument, then
it's kind of a class method. sort(list) <=> list.sort(). Make this a very general concept."

A function name is a generic function, a set of methods told apart by the class of their first argument.
`sort(d:Deck) := …` adds a method to sort; sort of lists and texts stays the library's. `x.f(args)` is `f(x, args)`.
```warp
class Deck{n:int}; sort(d:Deck) := d.n
Deck(3).sort()     // 3
[3, 1].sort()      // [1 3]: the library sort
class Animal{name:text}; class Dog extends Animal{}
speak(a:Animal) := "..."; speak(d:Dog) := "woof"
speak(Dog("rex"))  // woof: the most specific method wins (a class over its parent, nearest first)
```
- Which names: a name with a method for a declared class and either a fallback (a library word, class_methods
  `is_library_method`, or an untyped definition `sort(x)` of the program) or methods for several classes. A name with
  one method and nothing else stays a plain function (function values, reflection see it unchanged). Declared trait
  operations keep their own path (witness_of); both name a method `f·T` (traits::witness_name).
- Calls (traits.rs `generic_calls`, in lower_conformances: before library_words, which would expand `sort` itself):
  an argument of a known class calls its method statically (direct `call $sort·Deck`); a literal or a variable only
  holding literals (analyzer::literal_variable_kinds) calls the fallback; a value of unknown class goes through the
  generated `f·dispatch`, which tests `instance_of` per class (each class mapped to its most specific method) and
  otherwise calls the fallback.
- Errors: a class without method and no fallback: `area has no method for circle; methods: area(square), area(rect);
  fix: define area(s:circle) := …`. A class like two classes at the same distance that both have a method:
  `f of an image is ambiguous: image is like both photo and sketch, …; fix: define f(x:image) := … to pick one`.
- An untyped definition `sort(x) := …` replaces the library function on purpose (the fallback) and stays a plain
  definition.

Open (later steps): dispatch on every positional argument (Julia), not only the first; class-body methods named like
library words still take class_methods' `method·sort` + `dispatched_by_class` route and should become methods of the
generic function too; the list/map/text library as methods where that simplifies the code.

## Methods: what came after (2026-10-12)
- The fallback counts only when it takes the call's arguments: `b.take()` of a class Box never falls back to list's
  `take(xs, n)` (traits.rs Generic::falls_back), so a single method is called directly.
- list/text/map words are standard functions (decision type-library-words): modules.rs with_type_module_uses adds the
  module for `f(x)`, `f x` and `x.f()`; a class's method of that name (body or `take(b:Box)`) is not the program's own
  word (own_functions), so the module still comes and the method joins its generic function. `from list import zip`
  keeps list's other words out. Sound's words other than play, beep, stop_sound and wait_sound still need `use sound`.
- A field and a method of one name (card warn-field): `class Box{size:int}; size(b:Box) := …` keeps both, `b.size`
  reads the field (traits.rs operation_calls; it used to call size(b), recursing forever inside the method),
  `b.size()` and `size(b)` call the method, and the definition warns (field_method_warnings).
- Follow-up cards: dispatch-positional (done), warn-field (done), library-words (step d, done: every standard function is a
  fallback, one builtin method rule in lower_function_methods).

## Every positional argument (card dispatch-positional, 2026-10-12)
A method's signature is the class of each positional parameter, None where it takes any value (traits.rs Signature);
`collide(a:Ship, b:Rock)` is the method `collide·Ship·Rock`, `describe(k, s:Ship)` is `describe·any·Ship`, a first-only
method keeps its old name `sort·Deck`. Generic::method_for: per position the likeness steps from the argument's class
(0 its own, a parent 1, any value ANY_DISTANCE); the method no other one beats at every position wins; two left are
ambiguous ("meet(Dog, Dog) is ambiguous: meet(Dog, Animal) and meet(Animal, Dog) fit equally well", fix: define the
exact one). At run time the dispatcher tests `instance_of` at every typed position, per combination of the classes
that fit there (or no instance), the combinations testing more classes first.
