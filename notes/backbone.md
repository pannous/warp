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
| operators | core/parse/operators.wap, table in the preload | wap-reader | notation | operated |
| shape rules (stage 1) | core/rules/, preload/sugar/ | wap-reader | operated | surface |
| declare | core/analyze/declarations.wap | wap-binding | surface | declared |
| lower | core/rules/lowering.wap | wap-reader | declared | kernel |
| bind | core/analyze/scope.wap | wap-binding | kernel | bound kernel |
| type | core/analyze/types.wap | wap-binding | bound kernel | typed kernel |
| elaborate | core/analyze/types.wap, calls.wap | wap-binding | typed kernel | typed kernel, one instance per tuple |
| kind rules (stage 3) | core/rules/typed.wap, engine core/rules/ | wap-reader | typed kernel | typed kernel |
| emit | core/emit/ | wap-emitter | typed kernel (milestone 1: kernel) | Module (core/emit/wasm.wap) |

The languages, each a set of Node shapes:
- **notation:** what the reader makes and nothing else (notes/reader.md, wap/core/parse/reader.wap):
  - atoms: `Number` (int, or float when written with a point or a negative exponent; `-1` glued at an item's start
    is a negative Number, `a-1` is not), `Char` (a text of one code point), `Text` (escapes read; a template's `${…}`
    and an extension's `\:name` kept as written; a URL, a word glued to `://` up to a blank, bracket or separator),
    `Symbol` (a word, or an operator: a glyph with its repeats and one `=`, `<=` `==` `...`; `=>`, `and`,
    `yes`, `null` are all plain Symbols);
  - four keys (reader.md Q1, Q11, Q13): `Key(key, Op.Colon, value)`, `Key(key, Op.Assign, value)` and
    `Key(key, Op.Define, value)` by the same rules, `a:1` `a=1` tight, `a: b c` the rest of the phrase, `a b : c` and
    `x = a + b` all before and after; `=` and `:=` split before `:` (`x: int = 3` is `(x: int) = 3`);
    `Key(head, Op.None, group)`, a head glued to a bracket, `f(x)` `a{x:1}` `xs[1]` `f(x)(y)`, and to a type list,
    `list<int>` (reader.md Q14);
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
  operator (`print x`, `square 3`) stays a phrase: the lowering makes it a call, and typing refuses one whose head
  holds no function. `=` keys are the call `(Call = x 1)`; a glue key `Key(f, Op.None, (args))` stays, for the
  lowering to read as a call or a constructor. core/rules/lowering.wap reads exactly these: a glued call, an operator
  call (`=` → Set, `:=` → Define, `=>` → Fn), a phrase headed by a word, `{…}`; words such as `yes`, `break` or
  `global` are rules (preload/sugar/).
- **surface:** any warp: operated, with the spellings of stage 1 rewritten (`for`, `unless`, `a if c` …). What remains
  is either kernel or needs a declaration to decide.
- **declared:** surface, unchanged, plus facts: each top-level definition's `declares`, `name: signature` (`f: [a, b]`,
  a value's signature ø), and on the program `declarations`, the list of them, and `uses`, the files its `use "path"`
  statements bring in (core/analyze/declarations.wap). Each used file is declared once, after the files it uses; a
  program is bound inside the preload's declarations and those of the modules it uses itself, and the modules'
  statements run before its own. Modules may use each other: a cycle is cut where it closes, so of two such modules
  the one reached first runs last. A used `.wasm` that wap compiled is declared from its wap section, the module's
  declarations as warp text (wiki/wasm-representation.md; card wasm-module-import).
- **kernel:** only the forms of kernel/forms.wap, literals (`Number`, `Char`, `Text`, `True`, `False`, `Empty`) and
  `Symbol` as a name. Every phrase decided: no phrase, no operator word, no keyword is left.
- **bound kernel:** kernel, unchanged, plus two facts on each name: `binding`, what it refers to (`local`,
  `parameter`, `capture` of an enclosing function's name, `global`, `function` or `library`), and `defined_at`, the
  defining name itself, positioned (core/analyze/scope.wap); a library word is one the preload declares, a name
  nothing defines is refused. Each `Define` and `Fn` also learns `captures`, the defining names it reads from the
  functions around it, its nested functions' reads included.
- **typed kernel:** bound kernel plus each value's `type` (`int`, `text` …, as a program writes types; no fact for
  code that gives no value) and each function's `signature` on its name, `(int, text) -> int`; a `Call`'s
  `defined_at` is the one function it means, chosen among its `candidates` by argument types, then the type wanted.
  A function whose parameters have no type stands as one Define per tuple of argument types it is called with; one
  nothing calls is gone. The program comes first after the preload's functions it calls: one Block
  (core/analyze/types.wap). Every node with a position also has its `file` (core/stage.wap `file_of`).
- **emit** reads the typed kernel's `type` and `binding` facts and decides nothing about them: it may choose its code
  by them, but computes no type (test_guardrails.sh emit-infers-type).

**Facts live in Meta, never in the shape.** Positions (line, column), the rule or stage that built a node, a name's
binding, a type, the overload, a tail call, `@gpu`: each is a named fact in the node's metadata, read and written
through core/stage.wap (`with_fact(n, name, value)`, `fact_of(n, name)`), so a kernel form keeps the same holes from
stage 2 to the emitter and a pattern written for one stage matches in the next. A stage that only learns facts
(declare, bind, type) never rewrites; elaborate, in the same walk as type, is the one analysis step that writes: one
Define per tuple of argument types, dispatchers and lambdas, `prim box`, `prim cast` and `!`, the unknowns settled; a stage that rewrites gets positions and provenance from the walker. A
rewrite's result stands for what it replaced: it takes that node's position and facts (a definition's declaration
survives its lowering), while the new parts inside it take only the position.

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
- **Juxtaposition and calls.** A spaced call takes the whole operator phrase after it: `square 3 * 3` is
  `square(9)` (wiki/precedence.md, decision spaced-call-precedence). Juxtaposition is a precedence group with its own
  relations like any other, not a parser branch.
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
- **The reader, settled:** wap has its own pure notation reader (core/parse/), knowing no keyword; the old reader's
  keyword branches became spelling rules (preload/sugar/).
- **The old compiler** compiles the new backbone until the new backbone compiles itself, then retires.

## Milestones

0. **Step 0, the first brick:** carve the emitter's lower half (wasm module builder, instruction encoding, type and
   function sections, the value layout, runtime words) behind a clean interface that knows no surface forms
   (`emit_instruction`, `declare_type`, `add_function` …). The old emitter keeps running on top of it until the
   switch; the new kernel emitter is written on it. No other cleanup of the frozen compiler (user 2026-11-17).
1. **Hello:** `print "Hello"` through the new stages, with the emitter accepting only the kernel. Reached
   2026-11-17 (b5a5029b3d).
2. **The new compiler builds itself.**
   Done when all of these hold, each measured by a script, never by eye (user 2026-11-18, notes/wap_review.md):
   1. the gauge, `warp scripts/wap-census.warp`, says every function of every wap file typed and emitted
      ("typed n of n functions" on each line, the total likewise); test/wap/test_census.sh holds that count and
      only lets it rise;
   2. **met 2026-11-20** (self-loads-27): test/wap/test_self.sh has one line per wap file outside preload/, "wap compiles <file> and the result loads",
      all green; the preload is no program of its own but part of every program, so every test compiles it, and
      the gauge types its functions with the preload as their library;
   3. wap, run by the old compiler, compiles wap/compiler.wap to wap1.wasm; wap1 compiles it to wap2.wasm, and wap2
      compiles a hello that runs; the stages need not be the same bytes (user 2026-11-20);
   4. test/wap/run.sh passes with wap2 as the compiler;
   5. wap1 compiling wap/compiler.wap takes under a minute on the Mac;
   6. wap's source uses no word only the old compiler knows: compiled with no old-compiler library, it still builds;
      **met 2026-11-20:** wap resolves names only against wap/preload and the files a program uses, no `use` leaves
      wap/, and test_self.sh's "the compiler wap makes of itself compiles a program that runs" runs it from a folder
      holding only preload/ and kernel/;
   7. no boxed `any` is left in wap's own source: the boxed-any allowlist of test/wap/test_guardrails.sh is empty.
      **met 2026-11-20:** test/wap/guardrails_allowed.txt has no boxed-any line.
   Until then nothing lands that turns no function of wap green, unless it fixes a wrong value.

## Architecture changes, in order (user 2026-11-20)

From data/review/architecture-review.md, adopted whole. They replace the census-only rule: wap work comes from these
steps and the open milestone-2 points. A step starts when the one before it has landed; work in files the steps before
don't touch may run beside them. One owner per file, as below; a part in another owner's file passes that owner's
review.

1. **wap2 is wap's seed, and the compiler under test is a parameter** (milestone 2 points 3, 4, 5). Owner
   wap-operators: compile.sh `--compiler <module>`, run.sh `--stage <n>`, test_self.sh's wap2-runs-hello
   and timed lines, the census on wap2, wap2 published through refresh-seed's A B C stages. Old warp stays a frozen
   fallback and still builds the native warp and the playground. Done when test/wap and the census run on wap2.
2. **The old-warp Debts paid as one batch, facts that replace first**, test/wap on wap2 before and after:
   - lead: `with_fact` replaces a fact of its name and `with_facts_of` merges by name (stage.wap; nodes.wap with the
     reader); `compact` into preload/lists.wap; `staged` and `replaced` one name; EmptyMap no kernel form (with the
     emitter);
   - wap-reader: `NOTHING_WRITTEN`, nodes.wap's copy of core/node.warp, Op spellings and format's brackets written
     once, the reader writing Error, `EMPTY_BRACES`;
   - wap-binding: the binder's Empty marker (`without_fact`), a default of ø, helpers named apart;
   - wap-emitter: `===` as identity, so stage 1 and stage 3 agree on enum tags.
3. **Stage 2 on the rule engine:** a hole test that reads declarations (`$f: function`, `$p: predicate`).
   - wap-reader: the hole test (rules/stages.wap); `is pred` and `is not` as preload/sugar/ rules (nothing rewrites by arity
     today, so no arity rule);
     the parser's defined_words, local_words and double read gone (operator-spelling aliases stay there).
   - wap-binding: a word alias as binding's fallback for a name nothing in scope defines; the `is`/`not` rewrites gone
     from scope.wap; typing's `print`/`show`/`set`/`#` found by a role the preload marks, not by spelling (the role
     words are a question for the user).
4. **One Signature record from declare**, read by every stage and printed as the wap section: wap-binding (analyze/),
   wap-emitter (Placed by definer id), text keys become definer ids. Then wasm-module-import resumes.
5. **Primitives as opcode mnemonics; the kernel the emitter's input only:**
   - wap-emitter: a Prim names a mnemonic of opcodes.wap, one host table with its WASI types, the special names one
     kernel/primitives.wap table;
   - wap-reader: the preload's prim spellings (`prim i64.add`), helpers that are plain warp moved into the preload,
     `@global`/`@local` rules in place of Variable, Item into core/rules/;
   - wap-binding: typing writes Call for a function-key Get, and the emitter's branch for it goes;
   - lead: kernel/forms.wap exactly what the emitter takes; its reading helpers into core/stage.wap or
     core/statements.wap.

Paused work comes back with its step: the rest of stage 2 with step 3, EmptyMap with step 2, wasm-module-import after
step 4. Overload specificity and `"x" * yes` stay Debts.

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
2. ~~The reader~~: decided, wap's own pure notation reader in core/parse/.
3. ~~Folder~~: decided, wap/ (kernel/, core/; preload and library later).
4. **The kernel list:** start the case-by-case discussion from notes/kernel.md's 13 forms plus primitives?

## Open, not built yet

- **Methods, the rest of wiki/class.md:** bare field names inside a method reading and writing the receiver
  (`inc() := { c = c + 1 }`; scope's, binding a bare name to `self.c`), a getter without parentheses
  (`double := v * 2`), defaults (`n: int = 1`), `init`, and `extends`. Built: methods written after a class's fields,
  each a function taking `self` first, so `b.area()` and `b.area` both call it.
- **Module scoping** (user 2026-11-19): a name is looked up local > parameter > program > used module > preload, the
  nearest scope that defines it wins and shadows the rest; two used modules defining the same name are a loud clash
  naming both places, called or not, never overloads of one name; `m.f` picks one module's word explicitly. Not
  built yet.
- **`#` of a text:** counts its characters (UTF-8, not bytes), as warp does; `#"abc"` is refused until then.
- **Enums, the rest of wiki/enum.md:** `as index` and `as position`, cases with spellings (`Error = "error"`),
  `2 as Coin`, and `==` of cases with fields (by their fields). Built so far: `enum Coin { Penny, Dime }`, cases with
  fields `Circle(r: int)`, a case named by its enum (`Coin.Dime`, and `Coin::Dime` its alias), `match` by case with
  patterns in the fields (`Two(Circle(r), 0)`), `x is Circle`, `Dime is Coin`, and `==` of a plain enum's cases.

## Debts

(Each compromise against a principle, with its reason. Empty at the start.)

- **A name a program defines is read as its alias's canonical word** (wap/core/parse/operators.wap word_read: `length`
  and `size` read as `count`), so two variables `length` and `count` in one scope are one variable. Reason: found in
  the stage-2 self-build; host.wap placing_function names its local `measure` instead. Paid off when an alias applies
  only to a word nothing in the program defines. Paid off: an alias reads only a word nothing in scope defines
  (core/parse/operator_table.wap words_defined, test_lists "a program's own length and count are two names"); the
  local is `length` again.

- **A function value read from a map, called twice through a parameter, traps** (wap/core/emit/function_values.wap;
  reported by warp-wap-reader 2026-11-19): `m: map<text, (int) -> int>`, then `f(f(x))` inside a function taking
  `f: (int) -> int` given `m["inc"]` stops on a null reference. Called once it works, and so does `twice(inc, 3)`.
  Reason: off the self-build census (user 2026-11-19). Paid off once wap compiles itself.

- **`use "m.wasm"` is not read yet** (card wasm-module-import, wiki/wasm-representation.md): a compiled module can't
  be used as its source is. Reason: paused for the self-build census (user 2026-11-19). Branch wasm-module-import has
  the exports and the `wap` section of header lines; the reader, the body-less `export` declaration and the import
  are left (card note). Paid off once wap compiles itself.

- **A failed write traps without naming itself** (wap/core/emit/host.wap writing_function): a write that fails or
  takes none (a closed stdout, `| head`) stops the program with wasm's bare "unreachable", not a raise naming it.
  Reason: a raise there pulls its text and the raise helper into every program, hello 432 → 476 B. Paid off when a
  raise from a helper costs a program nothing it does not use.

- **Boxed `any` in wap's own source** (user 2026-11-19): a value of wap's source whose type typing can't name yet is
  boxed as `any` and checked where it enters a typed place, though the compiler's source should stay fully typed
  (first_principles.md). Only analyze/types.wap decides what is boxed; the emitter never infers it. Each boxed spot
  is one entry in test/wap/guardrails_allowed.txt, which may only shrink. Paid off when that list is empty, which
  milestone 2 requires.

- **A decimal literal is a binary float (f64), not exact:** `0.1 + 0.2` is 0.30000000000000004 in wap and 0.3 in warp,
  whose decimal literals are exact (notes/decisions.md core-round-untyped-half, exact-decimal-big; tests
  test/numbers/test_number_and_unit_rules:7, test_fraction_written_form:11, test_declared_float_exact_reals:13). The
  compiler's own source barely uses decimals, so self-build does not wait for it; the exact number kind (the runtime's
  exact numbers as preload warp) comes after self-build. No test pins the f64 value. Where: core/parse/atoms.wap
  number_read, preload/floats.wap; `1e21` likewise shows 1e21 where warp gives 1000000000000000000000.

- **A slice past a list's end traps** (array_slice, wap/core/emit/primitives.wap): wasm's array.copy stops the
  program without naming the user's line. Refusing it by name would need bounds checks the slice's caller writes, for
  example the preload's `slice` taking a Range and raising when it does not fit.

- **Stage 3 retypes the whole program after each round** (wap/core/rules/typed.wap): a typed rule's output is bound
  and typed again from scratch, under a limit of rounds. Simple, and the output is checked like any code; paid off by
  a per-subtree re-entry if the cost shows. A rule's new call of a function without parameter types finds only the
  versions typing made before, not a new one.
- **The class rule's helper words are ordinary words** (wap/preload/sugar/records.wap `member_as "field"`,
  `member_of`, `shown_case`): a program writing them is rewritten too. Words only rule files may write need a private
  word in the reader; paid off when a program meets one.
- **`for` is two rules, a `to` range and a list walked by index** (wap/preload/sugar/loops.wap), not the iterator of
  notes/kernel.md:82: wap's own source walks only these two; paid off by the first third kind of thing to walk.
- **A list's brackets and separator are spelled twice**, in wap/preload/nodes.wap (opening, closing, separator_text) and
  wap/core/format.wap: warp, which compiles the compiler, refuses the preload's own Node enum, so format.wap cannot use
  nodes.wap. Paid off when wap compiles itself and the compiler sees the preload: format.wap's three go.
- **Stage 3 serves only the rules programs write** (decision hole-types): no rewrite of wap's own moves into it. The
  char read as a text depends on the type its place wants (binding's conversion), `x is w` on what w means (scope);
  a typed rule sees only its hole's type.
- **wap's reader refuses instead of returning an `Error` node** (wap/core/parse/reader.wap). Bends notes/reader.md
  §1, which promises `Error` in the notation. Reason: in warp (which runs wap) an `Error` node cannot be told apart:
  `match` takes a Key or a List for `Error(…)`, and `is Error` says no. Paid off when the reader runs on wap.
- **Ints longer than 18 digits read as floats** (wap/core/parse/reader.wap `whole`). Reason: warp's `int(text)`
  wraps beyond 64 bits. Paid off by an unbounded int reading.
- **Glued names read as operators** (`a-b` is `(Call - a b)`). Kebab names are stage 2 by notes/reader.md, but the
  operator stage decides the glued run first, before declarations say whether `a-b` is a name. Paid off when the
  stage leaves a glued run of declared-name candidates for stage 2, or the user drops kebab names.
- **A program-level call through a function value can read a global before its statement** (wap/core/analyze/types.wap
  called_reads): `H = {"a": f}; H["a"](2); X = 1` reads X's default, since a value's possible functions' late reads are
  not known at the call. Paid off by refusing a program-level call through a value whose functions read late, or a
  run-time check, when code meets it.
- **`format` prints no comments yet** (wap/core/format.wap). Comments ride in Meta; the printer reads only shapes.
- **A program's value is shown only when it is an int** (wap/core/emit/emitter.wap). Bends "refuse, never guess"
  mildly: any other value is dropped, not shown. Reason: an int is the one value the host shows as warp does (main's
  i64 result); a float would show the host's digits (0.1 as 0.10000000000000001), a char or yes/no as a number.
  Paid off when values show themselves through the runtime's show words, boxed into a $Node.
- **The emitter learns a value's type by emitting it** (wap/core/emit/emitter.wap), though the typing stage gives
  each one: it reads the functions' signatures, but an `if` still joins its arms' types and a primitive takes its
  operands' emitted layouts. Bends "meaning is typed" (the types are there, but read late). Paid off when the emitter
  reads each form's `type` fact.
- **A written result type is not declared** (wap/core/analyze/declarations.wap): `f(a): int := …` declares
  `f: [a]`, so only the typing stage knows the result. Paid off when signature rules need it.
- **Function values and captures are refused by the typing stage** (wap/core/analyze/types.wap), as by the emitter.
  Paid off with closures.
- **An int and a float mixed are refused** (`1 + 2.5`: no `+` takes (int, float); wap/preload/prelude.wap). warp
  gives 3.5. Reason: the overloads are chosen by exact types, and no conversion of an int to a float where one is
  wanted exists yet. Paid off by that conversion, in typing, not by mixed overloads.
- **A used module's functions are emitted even when nothing calls them** (wap/compiler.wap: a module's bound statements
  go first in the program's Block), while the preload's uncalled definitions are left out. Bends "smaller and faster".
  Reason: a module is code whose statements must run, so it joins the program whole. Paid off when typing leaves out
  the module definitions nothing calls, as it does the preload's.
- **In a cycle of modules, a top-level statement may read the other module's value before it is set**
  (core/analyze/declarations.wap modules_used): of two modules that use each other, the one reached first runs last,
  so a statement of the other reading its variable sees it unset. Should be refused at that read, like the late-read
  rule for globals; functions are unaffected. Reason: the late-read check sees one program, not the module order.
- **`x.f` finds only functions written in warp, not host words** (wap/core/analyze/types.wap part_typed): the key is
  text, so typing looks f up among the definitions it holds, and a library word declared by signature (`print`) is
  not one of them; `x.f(y)` is a plain call and finds both. Paid off when host words are definitions typing holds.
- **A bare `break`, `continue` or `return` is lowering's word form, not a rule** (wap/core/rules/lowering.wap
  `WORD_FORMS`), while `while c { b }` and `return x` are rules (wap/preload/sugar/loops.wap). Reason: a rule whose
  pattern is a word alone has no head, so the termination check (core/rules/stages.wap `terminating`) sees it fire
  anywhere and refuses it. Paid off when a lone word is its own head. Paid off: S3, wap-rules-statements d313356f76.
- **Ints have no `÷` or `%` yet** (wap/preload/floats.wap holds only the truncated quotient and rem, which programs do
  not see): warp's `÷` and its floored `%` (`-7 % 2` is 1) are to be defined on top of them, and `quotient` is warp's
  exact division (3.5), so the truncated one keeps that name hidden. Paid off when the preload defines `÷` and `%`.
- **A module's rules reach files that don't `use` it** (wap/core/rules/stages.wap `shape_rules`). All loaded rules go
  into one table, so once a module is shaped its rules also apply to every file shaped after it, the preload's and
  the census's included. Bends "use builds the language the program is written in": the scope should follow `use`.
  Paid off by a rule table per program, the language's rules plus those of the modules it uses.
- **Each function value is a type of its own** (wap/core/analyze/types.wap `function_value_typed`): a call of a value
  calls its one function directly, typed for that call's arguments, so a variable or a parameter holds one `x => …`
  only, and an `if` choosing between two function values gives none; a function's name is a value the same way, one
  of several overloads not yet. Its captures
  are copied when it is made, so a change of one afterwards is refused. Paid off by function references (a funcref
  beside the captures, call_ref), for values chosen at run time.
- **Protocols fit structurally, by specialisation only** (wap/core/analyze/calls.wap `conforms`): a function of a
  protocol parameter is typed anew for each type given, so a list of mixed protocol values (run-time dispatch through
  function references), a protocol's default bodies (decision traits-are-protocols), `x is shape`, and a class's field
  counting as a word are not there yet. Paid off by slice B's function references in core/emit/ and the typing of
  defaults in core/analyze/; `map default ListMap` waits for its declaration rule.
- **A method's bare field names come from the classes of its Block only** (wap/core/analyze/scope.wap
  `receiver_fields`): a method of a preload or used module's class still writes `self.c`, and a function value inside
  a method that reads a bare field is not given self as a capture. Paid off when the binding sees the library's
  Records beside its declarations, and a field's capture captures its receiver.
- **A one-letter literal key on a map read fails** (wap/core/analyze/data.wap): `m#"b"` gives the key as a char, so
  the wasm does not validate; `m#"bb"`, `m["b"]` and `m#k` work. Edge case, no wap source writes it.
- **any compares other references by identity:** inside an `any`, refs other than boxes and texts compare by
  identity (a record compared directly compares by fields); by type via dispatch, as show does, when code needs it.
- **The preload copies core/node.warp's enums instead of reading the file** (wap/preload/nodes.wap). wap reads
  core/node.warp whole, but its `List(items: list, …)` gives an untyped list whose items wap can't read (`data#1` is
  refused: no # takes (list, int)). Reading it needs a bare `list` to mean list<any> and an `any` read back as a Node by
  a match. Until then nodes.wap types the items list<Node>, and the two copies must change together.
- **A rewrite that loops forever hangs the compiler instead of being refused** (wap/core/rules/stages.wap): letting
  any rule read a square list as one phrase made `xs = [1 2 3]` hang, some rule rewriting the phrase `1 2 3` again and
  again (the loop was not traced). A list is now one phrase only when its first word leads a rule. A rewrite whose
  result fires the rules again without end should be refused by name, as every other unknown form is.
- **Every word a rule's template spells carries DERIVED** (wap/core/rules/templates.wap filled), though only a
  definition's name reads it (core/analyze/calls.wap ranked). Reason: a template does not know which of its words
  will stand in a definition's name place. Paid off when templates are built from their forms, so the Define's name
  hole is known.
- **Every operator's spelling is written twice**: in the table of wap/preload/operators.wap and in `text(op: Op)` of
  wap/preload/nodes.wap. Reason: the Op enum is declared case for case as core/node.warp declares it, so old warp
  reads wap's source the same; the table can't build it, and naming each Op in its row would save about 15 nodes for a
  second mechanism (warp-simplifier measured it, 2026-11-19). Paid off when old warp no longer reads wap's source: Op
  then comes from the table.

- **The literals a template writes are named twice** (wap/kernel/forms.wap literal_named): `True()`, `False()` and
  `Empty()` are spelled out beside preload/nodes.wap's declaration of Node's fieldless cases. The general rule is that
  a template's glued call of an enum's fieldless case builds that case. Reason: templates are built before the
  preload's Records are declared, so they can't look the cases up. Paid off when templates see the declared Records:
  the table goes for that lookup.

- **A rule that writes nothing writes the mark `()`** (wap/core/rules/stages.wap NOTHING_WRITTEN): a symbol its list
  drops, where an empty statement sequence would say it. Reason: old warp, which runs wap's source, reads an empty
  List node as ø, which a rule may also write as a value, `rule ø => Empty()`. Paid off when old warp retires: the
  mark becomes the empty sequence.
- **Overloads rank by weights, not by subsumption** (wap/core/analyze/calls.wap most_specific, ranked: typing weight,
  written structure, derived), and `text(1 to 3)` is refused as reading two ways. Reason: paused for the self-build
  census (user 2026-11-19). The subsumption ranking under the one conversion is built and parked on branch
  subsumption-ranking: one order for the static and the dynamic choice, an unconverted fit and then the instance of
  the other's parameters breaking ties; it still leaves 4 more functions untyped than main. Paid off when wap compiles
  itself: that branch lands.
- **`{}` is its own kernel form, EmptyMap** (wap/kernel/forms.wap), not Empty typed as the map wanted, as `[]` is an
  empty list (decided 2026-11-19). Reason: Empty carries no facts in warp's Node, so no TYPE can sit on it while old
  warp reads wap's source. Paid off when old warp no longer reads wap's source: `{}` reads as Empty, the form goes.
- **An overloaded function is not a value yet**: `parts.map(text)` types only as `parts.map(each => text(each))`
  (shape_taking in analyze/function_values.wap). Reason: a value of a name with several definitions is refused until
  a written function type picks one; the self-build census needs the function typed now. Paid off when the expected
  `(Node) -> text` picks the overload: shape_taking goes back to `.map(text)`.
- **A local named like a method shadows it**: a parameter `map` made `fields_of(record).map(…)` call the Node, so
  instance_from_map (analyze/data.wap) names its map `source`. Reason: the self-build census needs the function typed
  now. Paid off when `xs.map(…)` finds the list's method past a local of that name: the parameter is `map` again.
- **A function read from a map traps when its parameter calls it twice** (found 2026-11-19, off-census):
  `steps: map<text, (int) -> int> = {"inc": inc}`, then `twice(f: (int) -> int, x: int) := f(f(x))` with
  `twice(steps["inc"], 3)` traps with a null reference; `f(x)` once works, and `twice(inc, 3)` does too. Paid off
  when the nested call of such a parameter runs.
- **A named function is refused where a function type is written for a variable** (found 2026-11-19, off-census):
  `g: (int) -> int = inc` says it gives fn<"… inc">, while the same `inc` in a `map<text, (int) -> int>` literal is
  taken. Paid off when a function's value fits a written function type wherever it is given.
- **stage.wap's `replaced` is `staged`'s Node form, named apart** (wap/core/stage.wap): old warp cannot tell a Node
  parameter from a function-typed one, so `staged(n, built: Node)` beside `staged(n, rewrite: (Node) -> Node)` is
  refused. Paid off when old warp retires: one overloaded `staged`.
- **A parameter's default of ø reads as no default** (wap/core/analyze/calls.wap defaults_given): old warp's
  `fact_of` gives ø for an Empty fact, so `pattern: Declared? = ø` and a parameter without a default look alike, and
  defaults_given reads a missing default as Empty. Paid off when old warp retires: a missing default is refused there.
- **`compact` lives in core/stage.wap, not the preload** (wap/core/stage.wap): old warp's prelude lacks it, and wap
  can't hold two same-arity definitions of one name. Paid off when old warp retires: it moves to
  preload/lists.wap with a test_lists line.
- **A shape's value is named a Node before it joins a list of Nodes** (wap/core/analyze/matching.wap shape_missing,
  list_missing): `[Call(…)] + inner` binds `+`'s T to Call and to Node, which bound_as refuses, so the witness is
  written `shape: Node = Call(…)` first. Paid off when a letter bound to a shape and to the type it narrows binds that type.
  **Paid 2026-11-20 (census-shape-join):** a letter bound to a shape and its base binds the base.
- **Helpers named apart from same-named ones** (wap/core/analyze: `within` became part_of beside names.wap's scope
  `within`, `valued` value_bearing beside the emitter's): a
  program's definitions dispatch across all its modules, and a Node parameter dispatches as any, so analyze's
  `valued(type: Node?)` beside emit's `valued(type: list<int>)` is refused though each call site's types tell them
  apart. Paid off when a call is resolved statically wherever its argument types decide it.
- **A preload file compiled as the program brings its definitions twice** (wap/core/pipeline.wap typed_file): the
  file is the program and also part of the library around it, so calls.wap refuses "two definitions share the key"
  (preload/floats.wap, host, texts, preload.wap, prelude.wap) and nodes.wap lists `text(n: Node)` twice. Off the
  census: a preload file is library, typed function by function, never a program of its own. Paid off when a program
  that is a preload file is bound inside the rest of the preload only.
- **`===` is value equality in the prelude** (wap/preload/prelude.wap `===(a: T, b: T) := a == b`), where warp's `===`
  is identity (the user, 2026-11-18; wiki/equality.md): two separately built `[1]` are `===` under wap. Off the
  census: wap's own compiler compares definitions by their DEFINER id instead. Paid off when `===` is ref.eq identity.
- **The binder forgets a definer with an Empty marker** (wap/core/analyze/scope.wap resolved_taking writes DEFINED_AT
  Empty beside a name's CANDIDATES; wap/core/analyze/types.wap one_definer is its one reader). Reason: old warp's core
  can drop no fact (its with_attribute only appends), so removing the fact breaks the reference compiler. Paid off when
  old warp can drop a fact: then without_fact, beside with_fact in core/stage.wap.
- **A number is compared with 0 by its narrowed type** (wap/core/format.wap below_zero, read by led_by_sign). Reason:
  under wap2 `value < 0` on a `number` raises "<: no definition takes these values' types" (typing or dispatch gives
  no `<` for a number and an int; reported to warp-wap-binding). Paid off when that compares: led_by_sign's arm is
  `Number(value) => value < 0` again and below_zero goes. Paid off: binding's number-compare (445d612b1c) widens the
  int, and led_by_sign reads `value < 0` again.
