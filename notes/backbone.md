# The new backbone: first principles and plan

Preliminary draft, 2026-11-17, from the user's decisions in the supervisor session. The agent that builds the
backbone starts from this file with fresh context. Inputs: notes/kernel.md (the 13-form kernel, the census, what rule
patterns need), notes/layers.md, wiki/Philosophy.md, wiki/guiding-principles.md, notes/taste.md.

## Preamble: rules for every agent on this work

- **The old compiler is frozen.** Nothing new goes into it: no features, no cleanups. It only keeps building, so it
  can compile the new one. Everything moves to the new backbone.
- **The backbone is rewritten, not migrated.** Write each part the way first principles say it should be. Never copy
  an old pass and trim it: read it for the cases it handles, then close it.
- **Tests are a gauge, not the goal.** The pass rate tells us roughly how well the engine works. A red test means a
  missing rule or library function, or a form we discuss again. It never means a special case in the kernel or the
  emitter. Nobody compares the old and new compilers' output: only the tests count, not how they pass.
- **No watering down.** A compromise is never silent. It goes into "Debts" at the end of this file, with its reason
  and the principle it bends. The user reviews the debts at each milestone.
- **Separation of concerns.** Each stage has one job, a declared input language and a declared output language
  (nanopass style). It reads only its input language. A stage that peeks at surface forms its predecessor should have
  removed is a bug.
- **Untangle before adding.** When two concerns meet in one function (kinds inferred while emitting, binding redone
  inside a sugar pass), split them first.
- **Small, named, total.** Each function does one thing under an English name and handles every case of its input
  language. A case it can't handle is refused loudly, never skipped silently.
- **Ask before growing the kernel.** A kernel form or primitive is added only with a written case (below) and the
  user's OK.

## First principles

The condensed philosophy of the wiki. The two groups say what warp is and how its compiler is built. Every design
choice cites the principle it follows.

### The language

1. **Data and code are one notation.** One reader reads warp data and warp programs alike: atoms, keys, groups,
   operators, juxtaposition. Code is data until it runs (`data x` quotes it). This is the Lisp idea with a modern
   notation.
2. **Many spellings, one meaning.** Every spelling programs may write stays valid. Aliases are one table, alias →
   canonical word, and everything warp prints uses the canonical word. Spelling never changes meaning; only an
   explicit, printed reading may (`[a -1]` is formatted as `[a, -1]`).
3. **One general rule over special cases.** A form that falls out of the general rules is welcome, however esoteric
   (`"x" * yes`). A form that needs its own rule for one phrase is bloat. A fix holds for every sibling of the example
   that showed the problem.
4. **Refuse, never guess.** Where code reads two ways, or a form is unknown, the compiler refuses with a fix-it that
   names the readings. A plausible wrong value is the worst bug.
5. **Meaning is typed.** Units are values (`3ms`, `5km == 5000m`). A name can be found from the expected type
   (`on escape` finds `Key.escape`). Typing is optional to write and always there to the compiler.
6. **Plain warp first.** A feature is a function, an enum, a class, an overload or a block parameter in the library
   whenever plain warp can say it. A rule is only for meaning plain warp cannot express.
7. **Vocabulary is library.** The names and values of a domain (keys, units, colors, months) are declared in lib/.
   The core holds mechanisms only; a core diff that adds names of a domain shows a mechanism is missing.
8. **Familiar conventions first.** Where warp has no stated reason to differ, the meaning people know from Julia,
   shells and mainstream languages wins.
9. **Progressive disclosure.** A beginner writes `print 1 + 2`. The machinery (types, rules, operators) is there when
   wanted, never in the way.
10. **Less state.** Immutable values by default (ranges, texts, records). Globals and mutation are allowed but
    discouraged, and visible where they happen.

### The compiler

11. **Stratified stages, each with one job:**

    reader → spelling rules → declaration prescan → binding analysis → typing → typed rules → kernel → emitter

    Each stage's output language is written down. A stage sees only its own input language.
12. **The emitter accepts only the kernel and refuses anything else loudly.** This is the central guard: no surface
    form, no hidden word, no "just this one" shape reaches the emitter.
13. **Kernel forms are argued one by one.** There is no number limit; notes/kernel.md's 13 forms are the starting
    candidates. Each form needs a written case:
    - why it can't be a rule into the other kernel forms or a primitive;
    - what it costs in the emitter;
    - which surface forms lower to it.

    The user discusses each case before the form exists.
14. **Every surface form is a rule or a library function.** `for`, `unless`, `n times`, comprehensions, ranges,
    `match` sugar, units and string interpolation never touch the emitter.
15. **One binding analysis.** Scopes, shadowing, captures and globals are resolved once, in one stage, and every
    later stage reads its result. Today seven passes each redo it. Bug that shows it: a local `push` in one function
    breaks `.push` in another.
16. **One notation for built-in operations.** Every built-in is a declared prototype in the library, typed like any
    function. Its body is a primitive: a wasm instruction or a runtime word.

    ```
    operator +(a: i64, b: i64) as i64 = wasm i64.add
    operator +(a: text, b: text) as text = runtime concat
    ```

    Overloads resolve by type. A resolved built-in lowers straight to its wasm instruction, so there is no call
    overhead. The compiler knows built-ins only through these declarations, never by name in core code.
17. **Rules are semantic pattern matching.** A rule's pattern may match shape, declared signatures or inferred types,
    and that choice puts the rule in its stage (user decision rule-stages). A rule's output must type-check.
    - Hygiene: names a rule introduces never capture the user's names.
    - Positions: every node a rule builds keeps the source position of what it replaced, so errors point at user
      code.
    - Termination: rules terminate by construction. Each step lowers toward the kernel, and a cycle is refused
      when the rules are loaded.
    - Rules on inferred types fire only on settled types.
18. **Rules are found by their head.** Matching is indexed by the pattern's head word or operator and compiled, so
    many rules cost almost nothing. The compiler's speed is part of its design.
19. **Mechanisms, not cases.** core/ holds the machinery (reader, binding, typing, rule engine, emitter, the runtime
    the compiler itself needs) and no domain names, surface keywords or one-off conditions.
20. **Elegance first, never repeat.** The most general, most beautiful code is the target. A list, condition or rule
    written twice becomes a name.

## Layers

A proposal, to be decided:

| Layer | Holds | Rule of thumb |
|---|---|---|
| **kernel** | the kernel forms and the primitives: the emitter's whole input language | a form is here only with a written, user-approved case |
| **core** | the machinery: reader, binding analysis, typing, rule engine, kernel emitter, the runtime words the compiler needs | mechanisms only, no names of a domain |
| **extensions** | rule sets, one file per topic (`for`, units, ranges, `unless`, match sugar) | rules only, no compiled-in hook table; loaded like modules |
| **library** | functions, classes, enums, operator prototypes with primitive bodies, aliases, the operator table | plain warp |

The question for the user: do extensions stay a layer of their own, as rules only, or merge into the library, with
rules as just another kind of library declaration? The recommendation is to merge: then "library" covers both
functions and rules, and the core shrinks to the machinery alone.

## Operators: a core feature done properly

- **One operator table, in lib/.** Each row gives:
  - the symbol and its canonical name;
  - the fixities it has: infix, prefix, postfix;
  - its associativity;
  - its precedence group.

  `operator` declarations extend the table.
- **Precedence as relations, sorted topologically.** Precedence is not a number. It is a set of relations between
  groups: `multiplicative > additive`, `additive > comparison`. A topological sort gives the levels (as in Swift's
  precedence groups and Fortress).
  - Two operators with no relation between them, mixed without brackets, are refused with a fix-it, never guessed.
  - A cycle is refused when the table is loaded.
  - The wiki's rule that superscripts bind tighter (`x²⁺³ == x⁵`) is one relation in this table.
- **Juxtaposition and calls.** `square 3 * 3` reads two ways, so it is refused (wiki/precedence.md). Juxtaposition
  is a precedence group with its own relations like any other, not a parser branch.
- **Aliases, one mechanism with its nuances:**
  - word aliases (`through` for `to`) and symbol aliases (`…` for `to`, `≤` for `<=`);
  - aliases limited to one fixity (a word that is only infix);
  - discouraged aliases, which work with a got-it note;
  - type aliases.

  Each is an alias → canonical row. Spelling never carries meaning.
- **Meaning is overloading.** `a + b` is a call of the overloaded `+`. Built-in overloads lower to wasm (principle
  16), user overloads are ordinary functions, and units and vectors overload like any type.
- **Minus, one general rule for every operator with both a prefix and an infix form:**
  - Spaced on both sides, or on neither: infix (`a - b`, `a-b`).
  - Attached to what follows, after a space or at the start of an element: prefix (`[a -1]` is `[a, -1]`, printed so
    by `warp format`).
  - A literal number with an attached sign (`-1`) is the prefix operator applied to a literal, folded to a constant.
    There is no separate token rule.

  The same rule holds for `+` and every operator that can be both prefix and infix. Nothing is specific to minus.
- **Ranges** (`to`, `upto`) show the pattern: their spellings are aliases, and their meaning is one overloaded
  operator returning an immutable `Range`.
  - `for` walks anything that has `next()`.
  - A float range is an interval only, so `for` over it is refused by the general rule.
  - The range pilot on the old compiler was stopped when the old compiler was frozen; its branch notes are input
    here.

## What is reused and what is rewritten

- **Rewritten:** everything between the reader and the wasm bytes: spelling rules, prescan, binding, typing, the
  rule engine, kernel lowering and the emitter.
- **Reused as they are:**
  - the value layout (notes/self_hosting.md);
  - the runtime words;
  - the host words and the native driver;
  - the test harness;
  - lib/, as far as it is plain warp.
- **Open, the reader:** reuse it as it is, or rewrite it as a pure notation reader whose keyword branches (about
  1,700 lines today) become spelling rules. The recommendation is to reuse it for milestone 1 and rewrite it before
  milestone 2.
- **The old compiler** compiles the new backbone until the new backbone compiles itself, then retires.

## Milestones

1. **Hello:** `print "Hello"` through the new stages, with the emitter accepting only the kernel.
2. **The new compiler builds itself.**

After that, the test pass rate is a ballpark of how well the engine works, nothing more. Then the user and the
interviewer go through the ugly forms the tests show: each is either standardised, because it arises from the
principles, or dropped.

## Guardrails, measured every day

`scripts/vitals.py` gets a backbone section:
- lines per layer (kernel, core, extensions, library);
- the number of kernel forms and primitives;
- how many non-kernel forms the emitter refused;
- build cost.

A kernel addition, a debt or a rise in core lines shows up there the day it happens.

## Theory and prior art for the design

- **Spritely Hoot / Guile:** a small kernel (Tree-IL), then CPS, then wasm GC. The shape of this plan.
- **The nanopass framework (Keep and Dybvig):** many small passes, each with a formally declared input and output
  language. That is principle 11.
- **Racket's "sets of scopes" hygiene (Flatt) and syntax objects:** hygiene, and positions carried through
  rewriting.
- **Scala 3 typed quotes, Lean 4 elaborators:** typed rewriting whose output must type-check.
- **Bidirectional type checking:** local, predictable typing with good error messages, a better fit than global
  inference for "refuse, never guess".
- **Swift precedence groups, Fortress:** precedence as a partial order, with unrelated mixes refused.
- **Term rewriting, termination orderings and critical-pair checks:** rules that provably terminate and don't
  conflict.
- **A-normal form (ANF):** a candidate for the kernel's internal shape. Every intermediate value gets a name, which
  makes the emitter trivial.

## Open questions for the user

1. **Layers:** do extensions merge into the library, as rules being library declarations? Recommended: merge.
2. **The reader:** reuse it for milestone 1 and rewrite it as a pure notation reader before milestone 2?
   Recommended: yes.
3. **Folder:** where the new backbone lives until the switch, for example `backbone/`, then it replaces core/.
4. **The kernel list:** start the case-by-case discussion from notes/kernel.md's 13 forms plus primitives?

## Debts

(Each compromise against a principle, with its reason. Empty at the start.)
