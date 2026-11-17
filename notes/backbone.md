# The new backbone: first principles and plan

Preliminary draft, 2026-11-17, from the user's decisions in the supervisor session. The agent that builds the
backbone starts from this file with fresh context. Read first: notes/first_principles.md. Inputs: notes/kernel.md (the 13-form kernel, the census, what rule
patterns need), notes/layers.md, wiki/Philosophy.md, wiki/guiding-principles.md, notes/taste.md.


**Names (user 2026-11-17):** **warp** is the old compiler (core/, frozen), **wap** the new one built here. The few
language extensions wap needs to be written (named patterns that build, bind by place and read, see Layers) are built into warp first,
and small parts of warp may be rewritten with them as a trial.

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

notes/first_principles.md: the core identities (functions = operators, `f(x)` = `x.f`, overloads, `@meta`,
aliases), the language and compiler principles, and how agents work by them. Every design choice here cites one.

## Layers

kernel, core, preload, library, as wap/ is laid out: notes/first_principles.md "Layers" (user 2026-11-17). Kernel
forms are named patterns of Node with `$` holes: notes/first_principles.md, wiki/pattern.md.

## Stage languages: the contract between the workers

Each stage reads one language and writes one (nanopass, principle 11), and a worker owns each stage. A stage meeting a
form outside its input language refuses it, naming the stage that should have removed it. The wap lead owns this
section, wap/kernel/forms.wap and wap/core/stage.wap; a change to a language or a new kernel form goes through the
lead first, as a change of this table.

| stage | file | owner | reads | writes |
|---|---|---|---|---|
| read | core/parse/reader.wap | wap-reader | text | notation |
| operators | core/parse/operators.wap, table in the preload | wap-operators | notation | operated |
| shape rules (stage 1) | core/rules/, core/sugar/ | lead, until the rule engine has an owner | operated | surface |
| declare | core/analyze/declarations.wap | wap-binding | surface | declared |
| signature rules (stage 2) | core/rules/ | as stage 1 | declared | kernel |
| bind | core/analyze/scope.wap | wap-binding | kernel | bound kernel |
| type | core/analyze/types.wap | later | bound kernel | typed kernel |
| kind rules (stage 3) | core/rules/ | as stage 1 | typed kernel | typed kernel |
| emit | core/emit/ | wap-emitter | typed kernel (milestone 1: kernel) | Module (core/emit/wasm.wap) |

The languages, each a set of Node shapes:
- **notation:** what the reader makes and nothing else (notes/reader.md, wap/core/parse/reader.wap):
  - atoms: `Number` (int, or float when written with a point or a negative exponent; `-1` glued at an item's start
    is a negative Number, `a-1` is not), `Char` (a text of one code point), `Text` (escapes read; a template's `${…}`
    and an extension's `\:name` kept as written; a URL, a word glued to `://` up to a blank, bracket or separator),
    `Symbol` (a word, or an operator: a glyph with its repeats and one `=`, `<=` `==` `...`; `:=`, `=>`, `and`,
    `yes`, `null` are all plain Symbols);
  - three keys (reader.md Q1, Q11): `Key(key, Op.Colon, value)` and `Key(key, Op.Assign, value)` by the same rules,
    `a:1` `a=1` tight, `a: b c` the rest of the phrase, `a b : c` and `x = a + b` all before and after;
    `Key(head, Op.None, group)`, a head glued to a bracket, `f(x)` `a{x:1}` `xs[1]` `f(x)(y)`;
  - `List(items, bracket, separator)` in three uses: a phrase, items with blanks between, separator Space
    (`a + b` is `[a, +, b]`); a glued run of no head and bracket, separator None (`a.b`, `3ms`, `a-b`, `-x`); a
    bracketed list, Curly, Square or Round, with the loosest separator inside. Separators cascade (glue < space <
    `,` < `;` < tab < newline), the loosest splitting first; a list never has one item unless bracketed. An
    indented block is a Curly list with separator Newline, glued to the line before it like any bracket;
  - `Empty` for an empty text and for `{}` and `()` (wiki/node.md: no items is the empty list whatever the bracket).
    Facts in the metadata (core/stage.wap): positions; `comment` (a comment, on the next atom or on the atom ending
    the line); `meta` (the `@annotations` before an item, without `@`); `warning` (a duplicate key in `{…}`). No
    `True`, `False`, `Type`, `Data` or `Error`: a text it cannot read is refused.
- **operated:** notation in which every operator phrase is read by the table: `a + b * c` is
  `(Call + a (Call * b c))`, every alias replaced by its canonical word (`≤` → `<=`), a prefix or postfix operator
  a `Call` of one argument, by the one minus rule. A mix of unrelated groups is refused. Juxtaposition that is no
  operator (`print x`, `square 3`) stays a phrase: deciding it needs declarations, stage 2. `=` keys are the call
  `(Call = x 1)`; a glue key `Key(f, Op.None, (args))` stays, for stage 2 to read as a call, an index or a
  constructor. Until the rule engine runs, core/rules/lowering.wap reads exactly these: a glued call, an operator
  call (`=` → Set, `:=` → Define, `=>` → Fn), a phrase headed by a word, `{…}`, the literal words `yes` `no` `ø`.
- **surface:** any warp: operated, with the spellings of stage 1 rewritten (`for`, `unless`, `a if c` …). What remains
  is either kernel or needs a declaration to decide.
- **declared:** surface, unchanged, plus facts: each definition's `Meta` names what it declares, and the program's
  block holds the table of declared names and signatures.
- **kernel:** only the forms of kernel/forms.wap, literals (`Number`, `Char`, `Text`, `True`, `False`, `Empty`) and
  `Symbol` as a name. Every phrase decided: no phrase, no operator word, no keyword is left.
- **bound kernel:** kernel, unchanged, plus two facts on each name: `binding`, what it refers to (`local`,
  `parameter`, `capture` of an enclosing function's name, `global`, `function` or `library`), and `defined_at`, the
  defining name itself, positioned (core/analyze/scope.wap).
- **typed kernel:** bound kernel plus each form's type, and on each `Call` the overload chosen.

**Facts live in Meta, never in the shape.** Positions (line, column), the rule or stage that built a node, a name's
binding, a type, the overload, a tail call, `@gpu`: each is a named fact in the node's metadata, read and written
through core/stage.wap (`with_fact(n, name, value)`, `fact_of(n, name)`), so a kernel form keeps the same holes from
stage 2 to the emitter and a pattern written for one stage matches in the next. A stage that only learns facts
(declare, bind, type) never rewrites; a stage that rewrites gets positions and provenance from the walker.

**Handoffs.** A worker hands a stage over with an outcome test in test/wap/ (a program compiled by wap, run, its output
checked) and a line here when its language changed. The lead reviews it against this table before it lands.

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
- **Meaning is overloading.** `a + b` is a call of the overloaded `+`. Built-in overloads lower to wasm ("Built-ins are
  declared like any function"), user overloads are ordinary functions, and units and vectors overload like any type.
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
- **The reader** comes first and gets special care: notes/reader.md, a data reader inspired by wasp's node parsing
  (not a copy). Three strictly separate steps: (1) the data reader parses any data into Node and knows no language;
  (2) then structural language matching (spelling rules); (3) then the analyzer (user 2026-11-17).
- **Was open, the reader:** reuse it as it is, or rewrite it as a pure notation reader whose keyword branches (about
  1,700 lines today) become spelling rules. The recommendation is to reuse it for milestone 1 and rewrite it before
  milestone 2.
- **The old compiler** compiles the new backbone until the new backbone compiles itself, then retires.

## Milestones

0. **Step 0, the first brick:** carve the emitter's lower half (wasm module builder, instruction encoding, type and
   function sections, the value layout, runtime words) behind a clean interface that knows no surface forms
   (`emit_instruction`, `declare_type`, `add_function` …). The old emitter keeps running on top of it until the
   switch; the new kernel emitter is written on it. No other cleanup of the frozen compiler (user 2026-11-17).
1. **Hello:** `print "Hello"` through the new stages, with the emitter accepting only the kernel. Reached
   2026-11-17 (b5a5029b3d).
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

1. ~~Layers~~: decided, four layers; kernel forms are named patterns of Node.
2. **The reader:** reuse it for milestone 1 and rewrite it as a pure notation reader before milestone 2?
   Recommended: yes.
3. ~~Folder~~: decided, wap/ (kernel/, core/; preload and library later).
4. **The kernel list:** start the case-by-case discussion from notes/kernel.md's 13 forms plus primitives?

## Debts

(Each compromise against a principle, with its reason. Empty at the start.)

- **wap's reader refuses instead of returning an `Error` node** (wap/core/parse/reader.wap). Bends notes/reader.md
  §1, which promises `Error` in the notation. Reason: in warp (which runs wap) an `Error` node cannot be told apart:
  `match` takes a Key or a List for `Error(…)`, and `is Error` says no. Paid off when the reader runs on wap.
- **Ints longer than 18 digits read as floats** (wap/core/parse/reader.wap `whole`). Reason: warp's `int(text)`
  wraps beyond 64 bits. Paid off by an unbounded int reading.
- **`print` known by name in wap's emitter** (wap/core/emit/emitter.wap, milestone 1). Bends "built-ins are declared like
  any function". Reason: Hello needs one output before the kernel has Prim. Paid off when `print` becomes a library
  function whose body is a host primitive.
- **Glued names read as operators** (`a-b` is `(Call - a b)`). Kebab names are stage 2 by notes/reader.md, but the
  operator stage decides the glued run first, before declarations say whether `a-b` is a name. Paid off when the
  stage leaves a glued run of declared-name candidates for stage 2, or the user drops kebab names.
- **`format` prints no comments yet** (wap/core/format.wap). Comments ride in Meta; the printer reads only shapes.
- **A program's value is shown only when it is an int** (wap/core/emit/emitter.wap). Bends "refuse, never guess"
  mildly: any other value is dropped, not shown. Reason: an int is the one value the host shows as warp does (main's
  i64 result); a float would show the host's digits (0.1 as 0.10000000000000001), a char or yes/no as a number.
  Paid off when values show themselves through the runtime's show words, boxed into a $Node.
- **A name the program binds nowhere is taken for a library word** (wap/core/analyze/scope.wap, `binding: library`)
  without asking whether the library declares it. Bends "refuse, never guess": a misspelled name is not refused by
  the binding analysis. Reason: the preload's declarations are not read yet. Paid off when the declare stage reads
  them; then a name neither the program nor the library defines is refused, naming where it was written.
- **The emitter learns a value's type by emitting it** (wap/core/emit/emitter.wap), until the typing stage gives
  types. A function's body is emitted at its first call, a parameter needs its type written (`x: int`), and a
  function calling itself or a global read before the statement that sets it is refused. Bends "meaning is typed"
  (the types are there, but found late). Paid off by the typing stage: the emitter then reads each form's type fact.
- **A definition is known by its line and column** (wap/core/emit/emitter.wap `place_key`): the emitter finds the
  function, local or global a bound name refers to by where its definer was written. Two files' definitions at the
  same line and column would collide. Reason: wap compiles one file so far. Paid off when positions carry their file,
  or the binding fact names its definition uniquely.
