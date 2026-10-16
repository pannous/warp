# One lookup for every word (card one-dispatch, user 2026-10-16, IMPORTANT)

Decision default-methods, generalized: `x.word` ≡ `x.word()` ≡ `word(x)` ≡ `word x` ≡ `word of x`, all resolved by
one lookup, multi-dispatch on the receiver's type, the most specific meaning wins; redefining a type's own library word
for that type is a loud error. Today the answer is spread over ~20 passes, each with its own idea of "the program
defines X" (survey below). This section is the plan and, as slices land, the record.

## The rule
A word names a generic function. `resolve(word, receiver, arity, form)` lists its meanings, most specific first:

| level | meaning | examples |
|---|---|---|
| 0 | a variable or parameter in scope (bare words only; `x.word` never reads a variable `word`) | `random = 4; random` |
| 1 | the receiver's own member: field of its class, entry of its literal or map, class method or method typed with its class (likeness distance: own class before a parent) | `{first: 1}.first`, `Deck(3).sort()` |
| 2 | the program's own untyped definition (any value), at the arities it takes | `count(x) := 7`, `x.square` of `square(x)` |
| 3 | a used module's word (`use json`: bare `parse` is json.parse) | `use json; parse t` |
| 4 | the library word of the receiver's built-in type (list/text/map/number modules, counts, conversions, number words, `type`, swizzles) | `xs.first`, `x.round`, `t.int`, `p.zy` |
| 5 | nothing: `undefined function: word` | `5.shout` |

- Static when the receiver's type is known at compile time (instance shape, literal, plain variable); otherwise ONE
  run-time choice between the levels that may apply: `if x has member word then member else <level 2-4 meaning>(x)`
  (generalizing today's map_field_or, field_or_word, dispatched_by_class and `f·dispatch`).
- Forms: a getter is the 0-argument meaning, a setter the 1-argument one; `x.word = v` ≡ `x.word(v)` ≡ `word(x, v)` ≡
  `word of x = v` call the setter when one exists, else store the field (decisions property-accessors, setter-calls).
- A function word used as a value is its 0-argument call (random-forms, saved-words: bare `canvas` is `canvas()`).
- After a number, a unit symbol is level 0 for units: the unit, even beside a variable (unit-after-number).
- Shadowing is per name and arity, scoped per function: a local `random = 4` in one function changes only that function;
  a program `first(x)` leaves `first(xs, 3)` to the library (as cross-module-overload does for modules).
- Uniform call: `x.f(args)` calls `f` when f takes `args + 1` arguments (one arity rule; today two).
- `first(xs: list) := …` (a program redefining a built-in type's own library word for that type) is a loud error;
  an untyped `first(x)` is the program's own word (level 2), see question Q1.

## Where it lives
`src/lowering/words/meanings.rs` (new): `ProgramNames` (scoped, per arity, computed once per pass from the tree) and
`meanings(name, receiver: Receiver, arity) -> Vec<Meaning>`, `Receiver` = Known(type)/Value(kind)/MayBeObject. The
passes keep their positions in the pipeline but ask it instead of their own tables.

## Today (survey 2026-10-16, worktree one-dispatch on final-lowering fe4afbeb1)
`x.word`: library_words.rs `method_call` (:906), in order: `pair.0` → `dict.fromkeys` → literal field :916 →
instance field :928 → library word, or `map_field_or` :931/:1052 → declared field :947 → loaded module word
(`field_or_word`) :953 → user function ≥1 parameter :966 → conversions :971 → `is_known_method` :1030 → maybe-object
field or number word :988 → number word :995 → `x.type` :999 → swizzle :1007 → undefined error :1011.
Earlier passes that already rewrite dots (pipeline.rs): of_lookups (`a of b` → `b.a`), module_properties (getter/
setter table, inlined), channel_words, plain_enums, member lambdas, reflection words (`f.params`, `p.fields`),
class_methods (method_aliases, `method·x` renames + dispatched_by_class, statics, `age·set`, method_calls),
foreign_modules, std_aliases (`x.json`), welcome_forms (`.to_s`), function_methods (called form, exact arity),
blocks (object-literal function entries), broadcasting (`names.upper`, `people.name`), lower_count_in,
instance_calls, traits operation_calls / generic_calls (generics.rs, `Generic::method_for`).
Bare words: units.rs (AFTER_NUMBER mark), module_properties `bare_side`, saved_words (SAVED_WORDS table),
host/aliases.rs (random forms), modules.rs + needed_definitions (module vs program, per arity), parse_word.rs,
getters.rs, ambiguous_forms (library word values), bare_calls.rs, type_constructor conversion_calls,
library_words `shadowed` + `word_call`, emitter scope tables.
"The program defines X" is computed 8 ways: library_words `shadowed` (scoped), objects.rs `defined_names`
(everything), music_words (+ loop variables, parameters), soft_keywords `names_given` (+ fields),
type_tests `program_names`, bare_calls (assigned + any parameter), needed_definitions (top level, per arity),
modules.rs (top level).
Gaps against the decisions: used-module-bare-words not built (parse_word ignores `use json`); saved-words still a
table, no `canvas()`; `width(canvas, 12)` reaches no setter; three setter mechanisms (inlined property, `age·set`,
`p = f(p, v)`); no error for redefining a type's library word; class-body methods named like library words take
`method·x` + dispatched_by_class instead of the generic function; `int(x) :=` refused but `hex(x) :=` allowed.

## Migration, one commit and one test per case, each special case deleted once covered
1. method_call → `meanings` + one static/run-time choice: deletes the 6 bare-form branches (literal, instance,
   declared, map field, loaded module word, maybe-object). Pinned by test_field_named_like_library_word,
   test_map_field_named_like_word, test_dot_without_parens, test_dot_word_same_path, test_number_methods,
   test_swizzles, test_type_of, test_conversion_methods, test_user_method_form, test_spaced_construction.
2. One uniform-call arity rule (library_words :966 and function_methods :28). test_user_method_form,
   test_conversion_methods, test_builtin_clash.
3. `ProgramNames`, scoped and per arity, replacing the 8 "defines X" sets (parse_word, host aliases, saved_words,
   music_words, std_aliases, type_constructor, bare_calls, library_words). test_bare_host_words, test_parse_word,
   test_type_calls, test_builtin_clash, test_sound_use, test_window_size.
4. Used module words bare (level 3): `use json; parse t`. New test tests/modules/test_used_module_bare_words.rs.
5. One setter rule: properties, class `set`, `p.f = v` through the 1-argument meaning; `width(canvas, 12)` and
   `width of canvas = 12`. test_property_definitions, test_property_access, test_window_size, test_sound_buses, new
   test for the uniform setter.
6. Saved words via the bare-call rule: `canvas()` in lib/draw.warp, `sound()`, SAVED_WORDS deleted.
   test_save_painting, test_sound_render.
7. Class-body methods named like library words join the generic function (drop `method·x` + dispatched_by_class).
   test_class_methods, test_dispatch.
8. Redefining a built-in type's library word for that type: loud error. New test.
9. Units and type words through the same lookup (unit-after-number mark, type-word definitions per Q2).
   test_units_after_numbers, test_type_calls.

## Open questions (sent to warp-interviewer, defaults taken)
- Q1 untyped `count(x) := 7`: default (a) the program's own word for every value at its arities (other arities keep
  the library word), no error; only a typed redefinition for the type's own type errs.
- Q2 type words as function names (`double(x) := x*2` refused today, `hex(x) :=` allowed): default (a) one rule, a
  program may define an untyped function named like a type word (level 2); `x: double` stays the type.

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
- Across modules (card cross-module-overload): a std module's method like shell's `sort(file:Path)` joins the list
  word's generic function too; a path literal becomes a Path instance at a Path position (notes/shell_words.md).

## Every positional argument (card dispatch-positional, 2026-10-12)
A method's signature is the class of each positional parameter, None where it takes any value (traits.rs Signature);
`collide(a:Ship, b:Rock)` is the method `collide·Ship·Rock`, `describe(k, s:Ship)` is `describe·any·Ship`, a first-only
method keeps its old name `sort·Deck`. Generic::method_for: per position the likeness steps from the argument's class
(0 its own, a parent 1, any value ANY_DISTANCE); the method no other one beats at every position wins; two left are
ambiguous ("meet(Dog, Dog) is ambiguous: meet(Dog, Animal) and meet(Animal, Dog) fit equally well", fix: define the
exact one). At run time the dispatcher tests `instance_of` at every typed position, per combination of the classes
that fit there (or no instance), the combinations testing more classes first.

## One method form (card one-method-form, user 2026-10-15)
`class P{def x(k):=…}` and `def x(p:P, k):=…` are one method: `p.x(k)`, `x(p, k)`, `x p`, `&x`, methods(P), dir(p)
behave alike. A class-body method lowers to `x(self:P, k)`; an outside one keeps its place (generic functions, traits
and dispatch see one form) and gets the class-body rules where they differed:
- bare fields: in `def x(p:P) := n + 1` a field n of P (inherited ones too) reads p.n, unless a parameter is named n
  (class_methods/outside_methods.rs, receiver_reads with the receiver p);
- a type word as the name: `def double(p:P)` is renamed `method·double` like a class-body `double()`, and
  `double(P(3))`, `double(p)` of an instance call the method in both forms (renamed_type_word_methods);
- reflection: methods(P) ≡ functions(P) lists every function outside class bodies whose first parameter is declared P
  or an ancestor (class_layouts); dir(p) = fields + methods(P), no untyped generic words. A module's warp.meta keeps
  outside methods among its functions (only class-body methods travel inside the class definition).
A first try moved outside definitions into the class body; that changed dispatch errors, traits, a units helper and
compiled modules' exports, so the outside form stays where it is.
Open: a trait's functions (`f(s:Shape)` is untyped before reflection reads it), `.declared_in` metadata.
