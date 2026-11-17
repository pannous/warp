# First principles

Every agent reads this file at start, before, during and after the backbone rewrite (notes/backbone.md). It is the
condensed philosophy of the wiki: each design choice cites the principle it follows. A line here changes only with the
user's OK, through warp-interviewer.

## Core identities

Forms that look different but are one thing. Each pair is one meaning; the differences are notation or meta only. A
special case that breaks one of these identities is a bug.

- **Data and code are one notation.** One reader reads warp data and warp programs alike. Code is data until it runs:
  `data x` quotes it.
- **Functions and operators are the same, except for notation.** `a + b` is a call of the function `+`. An operator
  is a function name with a fixity (infix, prefix, postfix) and a place in the precedence table. Any function can be
  given an operator spelling, and any operator can be defined and overloaded like a function.
- **Functions and methods are the same, except for meta.** A method is a function whose first parameter is the
  receiver. Where it is declared (inside a class or not) is metadata, not meaning.
- **`f(x)` is `x.f`, universally.** `f(x, y)` is `x.f(y)`, for every function, built-in or user-defined, on every
  type.
- **Overloads are distinguished by parameter types, or by the return type alone.** Several functions may share a
  name. The call picks the one whose parameter types fit. When only the return type differs, the expected type picks
  it: `x: Meters = parse(t)`. Where nothing decides, the compiler refuses and names the candidates.
- **`@meta` is always a meta node.** `@gpu`, `@global` and `@primitive` annotate what follows. They never compute a
  value; they steer how it is compiled.
- **Aliases are one table.** Every alias maps to a canonical word, for example `through` and `…` to `to`, or `≤` to
  `<=`. Spelling never changes meaning, and everything warp prints uses the canonical word.
- **Built-ins are declared like any function.** A typed prototype in the library whose body is a primitive: a wasm
  instruction or a runtime word. The compiler knows no built-in by name.
- **Units are values.** `3ms`, `5km == 5000m`, `90km/h`: a unit is a value multiplied in, and its type travels along.
- **A name can come from the expected type.** `on escape { … }` finds `Key.escape`, because the parameter wants a
  `Key`.
- **Blocks are values.** `{ it * 2 }` is a function of `it`. A parameter declared `block` takes the code unevaluated.
- **Emptiness is tested loosely.** `if no x`, `x is empty` and `if x` test emptiness; `x is ø` tests only for missing.

## The language

1. **Many spellings, one meaning.** Every spelling programs may write stays valid. Only an explicit, printed reading
   may decide between two readings: `warp format` turns `[a -1]` into `[a, -1]`.
2. **One general rule over special cases.**
   - A form that falls out of the general rules is welcome, however esoteric (`"x" * yes`).
   - A form that needs its own rule for one phrase is bloat.
   - A fix holds for every sibling of the example that showed the problem.
3. **Refuse, never guess.** Where code reads two ways, or a form is unknown, the compiler refuses with a fix-it that
   names the readings. A plausible wrong value is the worst bug.
4. **Meaning is typed.** Types are optional to write and always there to the compiler.
5. **Plain warp first.** A feature is a function, an enum, a class, an overload or a block parameter in the library
   whenever plain warp can say it. A rule is only for meaning plain warp cannot express.
6. **Vocabulary is library.** The names and values of a domain (keys, units, colors, months) are declared in lib/.
   A core change that adds names of a domain shows that a mechanism is missing.
7. **Familiar conventions first.** Where warp has no stated reason to differ, the meaning people know from Julia,
   shells and mainstream languages wins.
8. **Progressive disclosure.** A beginner writes `print 1 + 2`. The machinery (types, rules, operators) is there
   when wanted, never in the way.
9. **Less state.** Values are immutable by default (ranges, texts, records). Globals and mutation are allowed but
   discouraged, and visible where they happen.
10. **Beauty is a requirement.** Judge a form by notes/taste.md, not by whether it merely works.

## The compiler

11. **Stratified stages, each with one job.**

    reader → spelling rules → declaration prescan → binding analysis → typing → typed rules → kernel → emitter

    Each stage's input and output languages are written down. A stage reads only its input language.
12. **The emitter accepts only the kernel and refuses anything else loudly.** No surface form, no hidden word and no
    "just this one" shape reaches it.
13. **Kernel forms are argued one by one.** There is no number limit. Each form has a written case: why it can't be a
    rule into the other kernel forms or a primitive, and what it costs. The user agrees to each form before it
    exists.
14. **Every surface form is a rule or a library function.** For example `for`, `unless`, `n times`, comprehensions,
    ranges, units and string interpolation.
15. **One binding analysis.** Scopes, shadowing, captures and globals are resolved once; every later stage reads the
    result.
16. **Rules are semantic pattern matching.**
    - The stage follows from the pattern: a pattern can match shape, declared signatures or inferred types, and the
      rule runs at that stage.
    - A rule's output must type-check.
    - Rules are hygienic, keep source positions, and terminate by construction.
    - Rules on inferred types fire only once the types have settled.
17. **Operators are one table in the library.** Precedence is a set of relations between groups, sorted
    topologically. Two unrelated operators mixed without brackets are refused. One spacing rule covers prefix and
    infix forms for every operator that has both, minus among them.
18. **Mechanisms, not cases.** core/ holds machinery only: no domain names, no surface keywords and no one-off
    conditions.
19. **Smaller and faster.** The compiler's size and speed are part of its design. Rules are indexed by their head and
    compiled.
20. **Elegance first, never repeat.**
    - Picture the most general, most beautiful code and build that.
    - A list, condition or rule written twice becomes a name.
    - Never degrade the input to suit a tool: fix the tool.

## How agents work by these principles

- **Separation of concerns:** one job per stage, per file and per function. When two concerns meet, untangle them
  before adding anything.
- **Small, named, total:** handle every case of the input language, and refuse what you can't handle loudly. Never
  skip a case silently.
- **Tests are a gauge, not the spec.** A failing test is evidence. Never bend the kernel or the emitter to make one
  pass: add the missing rule or library function, or bring the form to the interviewer.
- **No silent compromises.** A compromise against a principle is written down with its reason, in the Debts of
  notes/backbone.md during the rewrite and as a card afterwards.
