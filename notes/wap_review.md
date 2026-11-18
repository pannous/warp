# wap: an independent review (2026-11-18)

Reviewer: warp-architect-review, a one-off session with no stake in the design. Subject: the new compiler in wap/,
read at bc30daa9c8 and 6fa64cf537. While the report was written main moved on to 73b39f4ad5 (two features and
three commits that split the largest files), so every `file:line`, every count, the census and all the programs
were taken again there: `file:line` refers to 73b39f4ad5. Nothing in wap/ or test/ was changed.

Main did not stand still after that either: in the next 28 minutes 17 commits landed, most of them answers to
first refusals this report lists. The body stays the snapshot of 73b39f4ad5 (`git show 73b39f4ad5:wap/core/…`
shows a cited line as it was). The census taken again at b85307a431 (17:13) and at 47fd489566 (17:21) still typed
the same 11 files, now of 51, while eight first refusals were fixed: the test of finding 1. At 47fd489566, 37 of
the 40 untyped files stop at fix 2.4 (`Separator.None`, 21 files), decision 1 (13), fix 2.1 (2) or fix 2.5 (1).
Main took decision 5 as option B (2cc3f843e8). The section "Dynamic types" was added after a talk with the user
and replaces the recommendation of decision 1.

## Verdict

In two days and about 300 commits wap became a working compiler: 7000 lines, ten stages, every outcome check
green. **Its front half is sound and follows the first principles; its back half has grown by accretion; and the
whole effort is aimed at the wrong target.**

- **Front half, keep as it is.** A pure two-pass reader, operators as one table of data in the preload, a rule
  engine with a head index, specificity, hygiene and a termination bound, kernel forms as named patterns over Node,
  built-ins declared in the preload with `prim` bodies, one binding analysis, every refusal through one function,
  a wasm module builder that knows nothing of warp. This is the best code in the repository.
- **Back half, needs a second pass.** Typing (1100 lines) is in fact a rewriting stage; it knows about thirty
  library and surface words by their spelling and answers "does this value fit there" in more than fifteen
  functions that differ by construct. The emitter (1500 lines) infers types a second time on wasm layouts, emits
  functions in the order it meets them, and carries about 340 lines of hand-assembled runtime. The plan's middle
  rule stage was never built; its work sits in four files. The third rule stage is built but no rule of the
  language uses it.
- **Direction.** The milestone is "wap builds itself", and wap is not measured against it. It gains language
  breadth (protocols, `any`, named arguments, shortest float text) while it refuses the way its own source is
  written. Of the 39 files of wap that hold functions, wap types **none** today. The census that should show this
  stops at each file's first refusal, so the team sees nine refusals where there are several hundred places wap
  would refuse, its own most used helper (`fact_of`) among them.

My main recommendation: stop adding language, decide what wap's own source may look like (decision 1), fix the
five small typing defects that block wap's most basic code, and put one worker on compiling wap with wap file by
file, with a gauge that counts every refusal instead of the first.

## How this was checked

- Read: notes/first_principles.md, backbone.md, kernel.md, reader.md, wap.md; all of wap/ (57 files: parse 1304
  lines, rules 933, analyze 1622, emit 1502, sugar 95, core root 495, kernel 74, compiler.wap 28, preload 936);
  all 27 scripts of test/wap; scripts/wap-census.warp. Of the five commits that landed during the writing, the
  diffs and the new files' seams, not every moved line again.
- Ran once: `bash test/wap/run.sh --wait` at bc30daa9c8: 376 ✓, 0 ✗ in 27 scripts, 82 s.
- Ran 58 small programs through wap to check each claim below that says "refused", "runs" or "fails to load"
  (appendix A), seven generated ones for timing, three programs through the old compiler, and the census on all
  50 program files of wap (appendix B). Every such claim was run, not inferred.

## The findings at a glance

| # | finding | kind |
|---|---|---|
| 1 | wap cannot type its own source, and the gauge cannot see how far it is | decision 1 + fix 1 |
| 2 | five small typing defects block wap's most basic code, none has a test | fix 2 |
| 3 | function values need one representation before self-build, not after | fix 3 |
| 4 | the plan's stage 2 does not exist; readings that depend on names are spread over four files | decision 2 |
| 5 | typing is an elaboration stage with fifteen "fits" relations; the plan says it never rewrites | fix 4 |
| 6 | the emitter is a second type system and emits lazily | fix 5 |
| 7 | a program can write kernel forms, and `prim` operands are unchecked: invalid wasm | decision 3 + fix 6 |
| 8 | the operator stage reads the rules' words through a global; five global tables in all | fix 7 |
| 9 | about thirty library and surface words are known to core by spelling | fix 8 |
| 10 | the runtime is hand-assembled wasm in the emitter | fix 9 |
| 11 | the third rule stage has no user; the sugar leaks helper words and special-cases `for` | fix 10 |
| 12 | structure: grab-bag files, circular uses, duplicated constants, dead bits, kernel forms in messages | fix 11 |
| 13 | tests are real but live outside `warp test`; nothing tests wap on wap | fix 12 |
| 14 | breadth before depth: the last day's features are not what self-build needs | road |
| 15 | compile time grows with the square of the number of functions | fix 13 |
| 16 | the notes disagree with the code and with each other | fix 14 |
| 17 | workers own files by history, not by stage; no one owns the self-build | road |
| – | one-letter texts are characters by default; `[if c then a else b]` | decisions 4, 5 |

## Decisions for the user

Each names its options with their cost in core lines, my recommendation first, and can be parked.

### Decision 1. What does a bare `list`, `map` or `function` mean as a type, and so how is wap's own source written?

wap's source is written in the old compiler's loose dialect: 334 parameters and fields typed bare `list` (49 of
them class fields, `xs: list`), 187 results written `as list`, 9 parameters typed `function`, 4 tables of functions,
10 places that build `[key, value]` pairs of two types, 5 global tables starting as `{}`. wap's type system is
static: every list has one item type, a function is typed once per tuple of argument types. It accepts a bare
`list` only at a parameter ("takes its argument's own type", core/analyze/calls.wap:262-265). Everything else is
refused:

| written in wap's source | example | what wap says |
|---|---|---|
| result `as list` | `joined(a: list, b: list) as list` (core/emit/emitter.wap:160) | "this gives list<int>, the function says it gives a list" |
| field `xs: list` | `class Reader { code: list; … indents: list = [] }` (core/parse/lexemes.wap:11) | "this gives list<α1>, the field holds a list" |
| a pair of two types | `[[entry#1, …]]` (core/analyze/letters.wap:37-39) | "a list's items are of one type" |
| a parameter `function` | `staged(n: Node, rewrite: function)` (core/stage.wap:21) | "this holds a function, no function to call: rewrite" |
| a table of functions | `HELPERS = {"print_text": printing_function, …}` (core/emit/primitives.wap:78) | "a map's values are of one type" |
| an empty map | `shape_rules = Rules({})` (core/rules/stages.wap:36) | "no Rules takes (ø)" |
| a name from a module used by a used module | `IMMUTABLE` in core/emit/values.wap (declared in opcodes.wap, reached through wasm.wap) | "nothing defines this name" |

notes/backbone.md:331-334 notes that "a bare `list` is unreadable to wap" for one file and draws no consequence;
notes/wap.md:175-192 lists missing library words as the blockers.

- **A (recommended): wap stays static, and a type written without its parts takes the type of the value given.**
  That is today's rule for parameters, extended to results and variables: `as list` means "a list, of whatever
  items the body gives". A class field and the result of a function that calls itself need the full type and are
  refused with that fix-it. wap's source then changes in about 50 fields (`indents: list<int>`), 10 pair lists
  that become small classes (better code: `entry.key` instead of `entry#1`), and the missing `use` lines. Tables
  of functions, empty maps and `function` parameters are compiler fixes, not source edits (fixes 2 and 3).
  Core cost: about 10 lines (core/analyze/calls.wap:253 and :265, one message). The old compiler accepts the
  typed spelling (probes h01, h02), so both compilers build the same source on the way.
- **B: bare means dynamic, `list` is `list<any>`**, items read back by run-time checks (the reading of
  backbone.md:333). wap's source compiles nearly as it is. Core cost: several hundred lines (a checked downcast
  wherever an `any` meets a typed place, in typing and emitter), every byte list of the emitter boxed, type
  errors moved to run time, and the language gets two regimes side by side.
- **C: every type written out in wap's source, no new rule.** Core cost 0; about 190 more source edits than A
  (the 187 results); the most explicit source.
- **Park:** continue file by file. Cost: the distance to self-build stays unmeasured.

Why A: it is the rule wap already has, said once instead of at parameters only; it keeps every type error at
compile time; and the edits it asks of wap's source make that source better.

Sub-point, same decision: a file sees only the names of the modules it uses itself, not of the modules those use
(core/pipeline.wap:40). The old compiler passes them on, and wap's source relies on it: in 20 of the 50 files the
first names wap cannot bind are another module's that the file does not use itself (eight files of core/emit/
read opcodes.wap through wasm.wap; core/parse/operator_table.wap even builds a `Use`, a class of operators.wap, a
module that uses *it*). Six of the 20 are of the day of this review: the split of the typing files made new
files that call each other without a `use`, and nothing could tell the author. Keep wap's rule (it is what
Python, JavaScript and Rust do), add the `use` lines, and let the refusal name the module: "IMMUTABLE is
opcodes.wap's: add `use "opcodes.wap"`".

### Decision 2. Where do readings that depend on what a name means live?

The plan has three rule stages; the middle one, "signature rules: declared → kernel" (notes/backbone.md:56,
notes/first_principles.md:123-128), was never built. Its work is done by hand in four files:

- core/rules/lowering.wap:65-103: any phrase led by a word becomes a call, without asking what the word is;
- core/analyze/scope.wap:130-132: `x is w` becomes a type test or the call `w(x)`; :220 `Shape.Dot` becomes a case;
- core/analyze/types.wap:191-193: a field named bare in a method becomes `self.field`; :228-229: the name of a
  function of no parameters becomes its call;
- core/emit/emitter.wap:123: a `Get` that carries a definition is emitted as a call.

A rule stage before binding cannot do this work anyway: whether `f x` calls `f` depends on what `f` is bound
to here, and only binding knows.

- **A (recommended): one small fixed pass `resolve`, right after binding**, that reads the binding facts and
  decides all of these in one file; binding goes back to learning facts only, lowering loses its guess, the
  emitter its special case. Core cost: no new lines, about 60 lines move into one place. The plan's text changes:
  two rule stages (shape, typed) and one resolve pass.
- **B: build stage 2 as rules, one stage later than planned (bound, not declared).** Needs a hole condition "this
  word names a function / a type / a field" in the matcher, about 40 core lines; then about 8 rule lines in the
  preload replace the hand code. The most principled; I would do it after self-build, with A's cases as its list.
- **C: leave it.** Cost: every new name-dependent form adds a case in whichever stage its author owns.
- **Park.**

### Decision 3. Is the kernel notation public?

A program that writes `(If yes (print "a") (print "b"))` or `(Call print "hi")` gets the kernel form (probes p13,
p14; core/rules/lowering.wap:83, core/rules/stages.wap:250). notes/first_principles.md:52 says so: a pattern
matches "every Node of that shape, whoever built it". wap/kernel/forms.wap:7-8 and notes/kernel.md promise the
opposite ("a program's own `(Call …)` group never reaches the emitter as one"). Everyday names are safe: a case
`Block`, a class `Item` or `Record`, a function `Set` work (probes s01-s04).

- **A (recommended): public.** The kernel is a notation anyone may write; rules already write it in their
  results. Core cost 0: correct the two sentences. What must not stay is that `(Prim print_text "x")` or
  `prim print_text(3)` gives **invalid wasm** instead of a refusal (probes p12, q14): fix 6 closes that for the
  preload's own typos too.
- **B: private.** Only the lowering may build kernel nodes: a mark on every node a rule or the lowering builds,
  checked by every stage. About 10 core lines plus a mark in every rule result, and it contradicts the principle.
- **Park.**

### Decision 4. Is a one-letter text a character or a text when nothing says which?

Today `"a"` is a character, turned into a text where a text is wanted. The turning is patched in at arguments,
fields, list items and generic letters (core/analyze/calls.wap:85, :216; core/analyze/letters.wap:112;
core/analyze/data.wap:68-69) and missing elsewhere:

```
xs = ["a", "b"]
xs += ["cd"]        # refused: no + takes (list<char>, list<text>)
```

```
names(nodes: list) as list<text> := nodes.map(n => match n { Symbol(name) => name; _ => "?" })
# refused: no map takes (list<Node>, fn<…>); with "other" instead of "?" it runs
```

- **A (recommended): a text unless a character is wanted** (Swift's rule): `x = "a"` is a text, `c: char = "a"` a
  character, `c == "a"` compares characters. One line in the single fit relation of fix 4, about 20 core lines
  fewer, no list<char> surprises. It changes the wiki's sentence "a one-character text is a character".
- **B: keep "a character by default"**, and make the turning one line of the same fit relation, so it holds
  everywhere. Core cost about the same; lists of one-letter texts stay lists of characters.
- **Park:** B falls out of fix 4 anyway.

### Decision 5. `[if c then 1 else 2]`

The old compiler reads one value; wap reads six items and refuses with "nothing defines this name: if" (probe t04),
because square brackets hold items separated by spaces. wap's own source writes it three times
(core/analyze/matching.wap:68, core/analyze/types.wap:180, core/rules/stages.wap:283).

- **A (recommended): keep wap's reading and refuse with both readings named**: "write `[(if c then 1 else 2)]` for
  one value". About 2 core lines for the message, 3 source edits.
- **B: a row in square brackets led by a word that leads a rule is one phrase.** About 5 core lines; rows in lists
  then read two ways depending on their first word.
- **Park.**

Main took B while this was written (2cc3f843e8, 17:11; 9 lines). It is still the user's to confirm or turn back:
"After the review" at the end.

## Fixes for the team, by importance

### Fix 1. A gauge that sees every refusal, and wap-on-wap as tests

- **Where:** scripts/wap-census.warp lowers and binds statement by statement but types and emits the file as a
  whole: the first refusal ends it. notes/wap.md:175-192 is read off it.
- **What it shows today** (appendix B): of 50 files, 11 type and emit: kernel/forms.wap (patterns),
  core/emit/opcodes.wap (120 constants) and the 9 rule files of core/sugar/. None of the 39 files that hold
  functions does. Those 39 show 9 distinct refusals between them, most of them in another file than the one
  asked for: `field_checked` (core/analyze/data.wap:27) stops 14 files, `float_bits` (core/emit/wasm.wap:59) 9,
  `Use` (core/parse/operator_table.wap:155) 6, `Separator.None` (core/parse/layout.wap:10) 4.
- **Why it matters:** the team works a queue of nine that refills from a backlog nobody has counted. It refilled
  while this review was written. At 6fa64cf537 `tokens_of` stopped five files in core/parse/operator_table.wap;
  affe088779 gave that file its `use`, and the same files now stop a hundred lines further down, at `Use`. On
  the same day the split of the typing files put `field_checked` and `same_type` into fits.wap, which data.wap and
  letters.wap call without using it: two new entries, and the queue is nine again. Fixing `float_bits` will
  likewise reveal the next line of wasm.wap, not the distance left.
- **Do:** (1) the census types each function on its own once the file is bound, so one refusal costs one function;
  it prints "functions typed / functions" per file and the distinct refusals ranked by how many functions each
  stops. (2) scripts/vitals.py already runs the census (:55-69): make that ranked list the daily queue.
  (3) test/wap/test_self.sh: "wap compiles core/emit/opcodes.wap and the result loads" today, one line more for
  every file that turns green: a ratchet that only moves forward.

### Fix 2. Five small typing defects that stop wap's most basic code

None has a test; each fix is a few lines; each needs one check line in test/wap.

1. **A name written with an optional type and first given ø takes no value afterwards.** `found: int? = ø` then
   `found = 3` → "this gives a int, the name holds a ø" (probes t01, t05, t06). core/analyze/types.wap:204
   remembers the value's type, not the written one (`wanted`, :197). wap's most called function is written this
   way (`fact_of`, core/stage.wap:63-69). Do: remember the written type when there is one.
2. **An early `return` in a function whose written result is optional is refused.**
   `first_big(xs: list<int>) as int? := { for x in xs { if x > 1 { return x } }; ø }` → "this function gives back
   values of different types" (probes q03, q04, p16). core/analyze/calls.wap:224-228 compares each returned type
   with the body's last value, never with the written result (known at :242). 31 of wap's 37 functions with an
   optional result and a block body return early. Do: where a result type is written, every returned value must
   fit it, and it is the result.
3. **`{}` cannot be an empty map.** `m: map<text, int> = {}` → "this gives a ø, the name holds a map<text, int>";
   `m = {}` then `m.a = 1` → "no field or function a for a value of type α1 ?"; `Rules({})` → "no Rules takes (ø)"
   (probes r02, r03, p11, q11). The reader makes `{}` Empty (notes/backbone.md:79). Do: an Empty written `{}`
   where a map is wanted is the empty map, as `[]` is the empty list of the type wanted; with nothing wanted, a
   map its first entry settles.
4. **A case named like an enum hides the enum.** With `enum LexemeKind { Atom, Open, Close, Separator, Key }`
   (core/parse/lexemes.wap:8), `Separator.None` (core/parse/layout.wap:10) is read as a field of that case: "no
   field or function None for a value of type LexemeKind" (probe t03). core/analyze/scope.wap:220. Do: in `A.b`,
   where `A` names an enum that has a case `b`, it is that case; if a second reading holds too, refuse with both.
5. **A parameter typed `function` cannot be called** (probe p17; core/stage.wap:21 `staged(n: Node, rewrite:
   function)` is the first typing refusal of core/errors.wap). core/analyze/calls.wap:265 lets a bare `list` or
   `enum` take its argument's type, not a bare `function`. Do: with fix 3, `function` is the bare function type.

A sixth, in the lowering, met in code that landed while this was written: **the index of an assigned place is
not lowered.** `xs[double(1)] = 9` and `bindings["$" + sign("b")] = 2` → "the binding analysis reads only kernel
forms; the lowering should have removed this: double(1)" (probes v01-v03); the same index read, and the same
place written `xs#(1 + double(1)) = 9`, work (probes v04, v05). core/rules/lowering.wap:116-117 lowers a place
that is a `Get` and passes any other on as written, so a call inside square brackets reaches binding unlowered.
wap's source is written so at core/rules/patterns.wap:148. Do: lower the index of every place.

### Fix 3. One representation for function values, before self-build

- **Where:** core/analyze/function_values.wap:13-14, :30-35, :46, :57-58, :65; core/analyze/letters.wap:115-124;
  the debt at notes/backbone.md:308-313.
- **What:** every function value is a type of its own, `fn<place, captures…>`, and is called directly; a
  parameter of a function type makes one copy of the function per function value it is given.
- **Why it matters now:** a list or a map of function values, a field holding one, an `if` choosing between two
  are all refused (probes p05-p08, q07, q08). wap's source has 4 tables of functions (core/emit/primitives.wap:78,
  :81, :82, core/emit/lists.wap:26) and 9 `function` parameters, among them the two walkers every stage calls
  (core/stage.wap:21, :49). And size: wap's source gives a function to a list word 304 times (`map` 153, `filter`
  63, `flat_map` 43, `any` 16, `all` 15, `fold` 14); each is its own copy of that word today.
- **Do:** a function value is one struct holding a function reference and its captures; its type is its
  signature, `(T) -> U` (a base struct per signature, a capturing value a subtype adding its fields, a call a
  `call_ref`). `(T) -> U` and bare `function` are ordinary types, `map` is typed once per (T, U). The `fn<…>`
  types, the copy per value and the "takes its own type" clause for function types go.

### Fix 4. Call typing what it is: the one elaboration stage, with one fit relation

- **The plan contradicts itself.** notes/backbone.md:112-116: "A stage that only learns facts (declare, bind,
  type) never rewrites", and notes/wap.md:126. But notes/backbone.md:105-110 defines the typed kernel as one
  Define per tuple of argument types, which is a rewrite. The code follows the second: core/analyze/types.wap:108
  appends `print(show(it))`, :191-193 writes `self.field`, :226 inserts `required`, :228-229 turns a name into a
  call, :260-265 writes the dispatch on a boxed value; core/analyze/conditions.wap:33-43 narrows by the spelling
  of `==` and `!=`; core/analyze/data.wap:106-114 rewrites item reads; core/analyze/function_values.wap:30-35
  lifts function values. The drift is right and the sentence is wrong.
- **"Does it fit" is answered in more than fifteen places:** `conforms`, `fits`, `fits_exactly`, `any_list`,
  `any_enum`, `reads_as_text`, `takes_own_type` (core/analyze/calls.wap:179, :205, :206, :208, :211, :216, :265);
  `takes_any_item`, `settles`, `reads_as` (core/analyze/letters.wap:28, :30, :112); `fits_type`, `gives_fitting`,
  `type_fits`, `boxed_fit`, `same_type` (core/analyze/fits.wap:28, :37, :40, :42, :44); `read_as`, `all_fit`
  (core/analyze/data.wap:68, :69); and again in the emitter: `fitted` (core/emit/values.wap:80), `passes`,
  `held_in_any` (core/emit/functions.wap:123, :84). They differ by construct: `if c then name else "?"` joins a
  character and a text, the same two arms of a `match` inside a function value do not (probes r05, r01).
- **Why it matters:** every new kind of type must be taught to each of them, and each miss is a refusal a user
  cannot explain.
- **Do:** (1) Write the plan as built: declare and bind learn facts; type elaborates the bound kernel into the
  typed kernel, in which every call names one Define, every conversion is written out and no function value is
  left unlifted. (2) One function `conversion(given, wanted)` answering *how* the value gets there (as it is,
  boxed into `any` or `number`, wrapped into `T?`, a character as a text, an empty `[]` or `{}` as the list or map
  wanted, never, or not at all), used alike for arguments, results, fields, items, variables and the joining of
  branches. Typing writes the conversion into the kernel once, so no later stage converts. (3) The files along
  their real seams: main did this while the review was written (d9b71cf23e, a848338f66: letters.wap,
  conditions.wap, matching.wap, fits.wap, variables.wap), and well. fits.wap, "how types meet", is the home of
  `conversion`: today it holds five of the relations above, and the other fifteen still live in five files.

### Fix 5. The emitter reads facts and nothing else

- **Where:** core/emit/emitter.wap:9-12 says it: "The emitter reads only the parameters' types yet: a value's type
  is known once its code is emitted, so a function's body is emitted at its first call". Hence the pending
  functions (:135-140), the joining and checking of branch types on wasm layouts (:147-162),
  core/emit/functions.wap:99-101 repeating core/analyze/calls.wap:224-228 down to the message,
  core/emit/places.wap:61-67 ("The first value given decides the type"), and core/emit/values.wap:80-87.
- **A hazard:** core/emit/values.wap:87 turns an `anyref` given where a reference is wanted into null
  (`given.instructions + DROP + null_of(…)`). If typing ever lets such a value through, the program goes on with
  ø: the plausible wrong value the principles call the worst bug. Make it a refusal.
- **Why it matters:** two type systems must agree forever, and the emitter's order of emission makes refusals
  depend on call order (functions.wap:127: "a function that calls itself needs its result type").
- **Do:** after fix 4 every value carries its TYPE and every call its SIGNATURE. The emitter maps types to
  layouts and emits functions in program order; `joined`, `checked_condition`, `result_type`, the inference in
  `set_code` and the conversions of `fitted` go, roughly 150 lines. To make trusting the facts safe, state each
  stage's output language as a check (`is_bound`, `is_typed`: every name read has a BINDING, every value a TYPE,
  every call a SIGNATURE, no function value unlifted), run by the tests after each stage. Today the stage
  languages of notes/backbone.md:50-60 exist only as catch-all refusals (core/analyze/scope.wap:75,
  core/analyze/types.wap:152, core/emit/emitter.wap:129).

### Fix 6. A table of primitives, checked (with decision 3)

- **Where:** notes/wap.md:58 promises wap/kernel/primitives.wap; it does not exist. What a primitive is sits in
  the emitter in six tables and a chain of lookups: core/emit/primitives.wap:52 `INSTRUCTIONS`, :64 `HOST_CALLS`,
  :66 `HOST_FUNCTIONS`, :78 `HELPERS`, :81-82, :88-105 `primitive_code`, and :107 `ON_LISTS`, three primitives
  that mean something else when the operand is a list.
- **Why it matters:** nothing checks a primitive's operands. `prim print_text(3)` and a program's own
  `(Prim print_text "x")` produce wasm that fails to load (probes q14, p12); so would a typo in the preload.
- **Do:** one table, a row per primitive: name, operand types, result type, how it is emitted (an instruction, a
  host call, a helper). Typing checks every `Prim` against its row and takes the result type from it; the emitter
  looks the row up; the three of `ON_LISTS` become rows of their own.

### Fix 7. One `Language` value instead of five global tables

- **Where:** core/parse/operators.wap:178-179 `phrase_words`, read at :138 and :181, filled by the rule loader
  (core/rules/stages.wap:100-103); core/rules/stages.wap:36-37 `shape_rules`, `typed_rules`;
  core/rules/patterns.wap:39 `declared_patterns`; core/rules/templates.wap:97 `fresh_count`;
  core/parse/operator_table.wap:32 `looser_than`.
- **Why it matters:** the operator stage reads what a later stage loaded, so the stage order holds only by load
  order. A module's rules reach files that do not use it (the debt at notes/backbone.md:304). Two programs
  compiled in one process share everything, which is what the playground, the tests and a self-built wap do.
- **Do:** a prescan of the `rule`, `pattern` and operator declarations of a file and of what it uses builds one
  value, `Language { operators, phrase words, shape rules, typed rules, patterns }`, which every stage takes as a
  parameter. Scoping by `use` then needs no code of its own.

### Fix 8. Words that core knows by their spelling

- **Where, library and surface words:** core/rules/lowering.wap:40 `"prim"`, :43 (`yes`, `no`, `ø`, `break`,
  `continue`, `return`), :46 (`let`, `global`); core/analyze/declarations.wap:25 `"use"`; core/stage.wap:120
  `"self"`, :132 `"is"`, :142 `"_"`; core/analyze/types.wap:40 (`text`); core/analyze/conditions.wap:42-43 (`==`,
  `!=`); core/analyze/calls.wap:14 `"#"`, :16 `"show"`, :17 `"print"`, :19 `"!"`; core/analyze/data.wap:11 `"+"`,
  :13 `"Range"`, :14 `"slice"`; core/analyze/function_values.wap:13-14; core/emit/values.wap:43 `"text"`;
  core/parse/atoms.wap:25, :173-183 (interpolation writes calls of `text` and `+`).
- **Where, the type words, declared five times:** core/analyze/types.wap:34-50, core/stage.wap:195-226,
  core/emit/values.wap:20-24, and one by one: `"enum"` (core/analyze/calls.wap:210 and core/emit/values.wap:24),
  `"never"` (types.wap:43 and core/emit/emitter.wap:31), `"ø"` (types.wap:41 and core/emit/functions.wap:53),
  `"dispatched"` (core/stage.wap:127 and core/emit/functions.wap:133), `"->"` (types.wap:50 and stage.wap:195).
- **Why it matters:** notes/first_principles.md:25-26 ("The compiler knows no built-in by name") and :132-133
  ("no domain names, no surface keywords"). Each word is a place where renaming a library function breaks the
  compiler without a refusal.
- **Do:** the type words once, in wap/kernel/types.wap. The library words as *roles* the preload declares on its
  definitions ("the program's value is shown by this word", "a value's text", "the item of", "required"), read
  by core through one file, never by name. The surface words go to the preload as shape rules, one line each
  (`rule yes => True`).

### Fix 9. The runtime in warp

- **Where:** core/emit/primitives.wap:140-239 assembles printing, raising, reading, int-to-text and char-to-text
  instruction by instruction; core/emit/lists.wap (136 lines) does lists and maps (`key_searched`, :113, is a
  linear search); core/emit/equality.wap (100 lines; :72 and :90 are the same line).
- **Why it matters:** notes/kernel.md:111 says runtime words are "functions of the runtime written in warp", and
  wap/preload/texts.wap shows it works: the text words stand on three primitives. Hand-assembled wasm is the
  slowest code to write and to review, and where modules that fail to load come from.
- **Do:** primitives are only what wasm gives directly (array new, get, set, length, copy; struct get and set;
  the host calls). Lists, maps, int-to-text and equality are preload functions in warp. Equality of a class or an
  enum can be written by the rule that already writes its `show` (core/sugar/records.wap:7).

### Fix 10. Rules: prove stage 3 or park it, and tidy the sugar

- **Stage 3 has no user.** core/rules/typed.wap binds and types the whole program again after every round in
  which a typed rule fired (:27, up to `ROUNDS_LIMIT = 100`, :18; the debt at notes/backbone.md:258). Neither
  core/sugar/ nor wap/preload/ holds a typed rule; test/wap/test_typed_rules.sh has 4 checks. Before more work
  goes into it, move two hand-written typed rewrites into it: the character read as a text, and `x is w`. If they
  read well as rules the stage has earned its place; if not, the plan's third stage is wrong and should go.
- **Helper words leak.** core/sugar/records.wap:7 (316 characters) tells its helper rules apart by text tags,
  `member_as "field"`, `member_as "parameter"` (:11-12): ordinary words a program can write too.
- **`for` is two special cases.** core/sugar/loops.wap:10-11: a `to` range counted by hand, and a list walked by
  index through the names `held` and `at`; notes/kernel.md:82 says iterator, has_next, next. A third kind of
  thing to walk needs a third rule.
- **`as` has two meanings since today:** the result type (`f(x) as int := …`) and a conversion (`x as T`,
  core/sugar/types.wap:4).
- **Layer:** notes/first_principles.md:144-150 puts the rules for surface forms in the preload; they sit in
  wap/core/sugar/, and notes/wap.md:48 calls the layer "extensions". Choose one. I would move them to
  wap/preload/, so that core/ holds machinery only and a `grep rule wap/core` finds nothing.

### Fix 11. Structure and names

- **Names that hide what a file is.** core/stage.wap (walkers, about 35 fact names, type notation, record
  lookups: four jobs) and core/rules/stages.wap (rule loading, phrase words, the matching driver, splicing)
  differ by one letter; core/rules/typed.wap sits beside core/analyze/types.wap and core/sugar/types.wap;
  core/rules/patterns.wap beside core/emit/patterns.wap. Split stage.wap into the walker, the facts and (fix 8)
  the type words; name each file by its one job.
- **File sizes: done on main.** At 6fa64cf537 five files exceeded the 300 lines of notes/wap.md:150-156 (types.wap
  450, calls.wap 413, scope.wap 366, parse/operators.wap 336, rules/stages.wap 334). Three commits split them
  while this was written; the largest is now calls.wap with 289. What is left: 28 lines of core longer than 250
  characters, which no rule counts.
- **Uses run in circles, and the split added to them.** Every file of core/emit/ uses emitter.wap and most of its
  siblings; in analyze, each of the new files uses types.wap and calls.wap, and those two use the new files back
  (core/analyze/types.wap:23-28, core/analyze/calls.wap:5-11); core/rules/lowering.wap:34 uses
  core/analyze/declarations.wap, which uses lowering (:19); in parse, atoms ↔ layout ↔ cascade ↔ reader. And a
  file calls what it does not use: fits.wap calls `settles` of letters.wap, data.wap and letters.wap call
  `field_checked` and `same_type` of fits.wap, conditions.wap and matching.wap call `joined_settling` and
  `joined_type` of fits.wap. The old compiler lets it pass, so nothing told the author. A stage order the files
  do not show is not a stage order. Rule: a file uses only files of its own stage or an earlier one, and uses
  every file whose names it writes; a ten-line script checks both until wap does.
- **The same thing twice:** `result_given` (core/analyze/calls.wap:224) and `result_type`
  (core/emit/functions.wap:99); `counted` with three parameters (core/emit/primitives.wap:184) and with one
  (core/emit/wasm.wap:85); the operators listed three times: `enum Op` (wap/preload/nodes.wap:24), the operator
  table (wap/preload/operators.wap:65) and, since today, `text(op: Op)` (wap/preload/nodes.wap:101), arm for arm.
- **One name, two functions:** core/format.wap:124 asks `is_letter` of a character, a word only the old compiler
  has; core/analyze/letters.wap:14 defines another `is_letter`, of a type and the records. Until the split wap
  bound the first to the second by name, through a `use` that serves nothing else (core/format.wap:9); since the
  split it binds to nothing.
- **Dead or stale:** `retyped` (core/analyze/types.wap:98) and `I32_NE` (core/emit/opcodes.wap:80) are never
  used; comments doubled or parted from their function at core/analyze/letters.wap:74-76,
  core/analyze/calls.wap:221-223 and core/emit/places.wap:14-15; stale ones at core/emit/functions.wap:127 and
  core/parse/reader.wap:9; `# todo: rename to Lambda` (wap/kernel/forms.wap:27); two bare `raise`
  (core/parse/operator_table.wap:18, :24) against core/errors.wap:3; the fact name `"meta"` as a bare text
  (core/format.wap:32).
- **Messages show the kernel, not the program:** "this holds a nothing, no function to call: (Call pick yes)",
  "the field holds a (Call -> (int) bool)", "no map takes (list<Node>, fn<"p15.wap:1:43 λ">): map: list<U>(xs:
  list<T>, f: (Call -> (T) U))". The user wrote `pick(yes)` and `(int) -> bool`. A refusal should quote the
  source at its position (Meta has line and column) and print types as they are written; core/format.wap can do
  both. Also "this gives a int".

### Fix 12. Tests

- **Keep:** they are real outcome tests, program in and output out, and every script names its source.
- **Move into the system:** they are bash scripts (`check "…" "$(ran name '…')" "…"`), so `warp test`, the stored
  baseline and the cloud suite cannot see them. Write them as pairs files, with test/wap/run.sh as their runner
  for now. After self-build the same files run under `wap test`, and the old corpus under test/ becomes wap's
  breadth gauge for nothing: "which share of the old suite passes under wap", in the cloud run only.
- **Missing:** the six defects of fix 2; a function value in a list, in a map, in a field; an empty map; a
  primitive given a wrong operand; a kernel form written by a program (once decision 3 is taken); any file of wap
  compiled by wap (fix 1).
- **A symptom:** three separate checks pin the character-to-text patch at three sites (test/wap/test_lists.sh:53,
  test/wap/test_types.sh:100-105, test/wap/test_texts.sh:13-17).
- **Tests of intermediates:** programs in the tests write `prim` themselves (test/wap/test_texts.sh:18-22,
  test/wap/test_data.sh:32, :58, :60, :74). They become calls of preload words when fix 6 lands.

### Fix 13. Speed, measured before it hurts

- **Measured:** a program of N functions, each calling the one before (`f2(x: int): int := f1(x) + 1`): 200 take
  1.5 s, 400 take 3.7 s, 800 take 11.3 s: three times the time for twice the functions. N plain statements stay
  flat (1 s up to 400).
- **Why it matters:** wap has about 780 functions; with one copy of a list word per function value (fix 3) and one
  per tuple of types, well over a thousand instances. On that curve a build of wap by wap is half a minute before
  any body costs anything, and every worker's loop is that build.
- **Likely causes, all lookups in lists:** `fact_of` walks a node's attributes on every question
  (core/stage.wap:63); typed and placed functions are searched in lists (core/analyze/types.wap:53,
  core/emit/emitter.wap:80); `added_type` compares every type (core/emit/wasm.wap:102); `settled_everywhere` walks
  the whole program again (core/analyze/letters.wap:60); lists grow by `out + [x]` (wap/preload/lists.wap:26-42),
  which copies.
- **Do:** nothing yet. When the first large file of wap compiles under wap, profile that build and turn the three
  or four hot lists into maps. Put a time budget into the definition of done.

### Fix 14. Notes that disagree with the code or with each other

- notes/wap.md: the tree lists kernel/primitives.wap (:58) and core/emit/closures.wap (:106), which do not exist;
  "State" (:160-171) still says milestone 1 is in progress; the layers are "kernel, core, extensions, library"
  (:48) against notes/first_principles.md:144-150 "kernel, core, preload, library"; the census table
  (notes/wap.md:175-192) names blockers that are built and misses the ones of decision 1 and fix 2.
- Who owns typing: notes/backbone.md:58 says wap-reader, notes/wap.md:146 says warp-wap-binding.
- notes/backbone.md:281 "Function values and captures are refused by the typing stage": they are built.
- notes/backbone.md:79 makes `{}` Empty and leaves the empty map without a spelling (fix 2.3).
- notes/kernel.md:114 has a stray "??"; wap/compiler.wap:12 says "declare operated → declared";
  notes/first_principles.md:57 writes `$called: Symbol` where wap/kernel/forms.wap takes any Node.

## Plan conformance

| the plan (notes/backbone.md, first_principles.md) | as built | judgment |
|---|---|---|
| a pure reader, Node out | core/parse/: reader, lexemes, atoms, layout, cascade | follows |
| operators: one table, data in the preload | core/parse/operator_table.wap, wap/preload/operators.wap; reads the rules' words through a global | follows; fix 7 |
| stage 1, shape rules | core/rules/ (stages, patterns, templates), 95 lines of rules in core/sugar/ | follows; their layer is undecided |
| declaration prescan, facts only | core/analyze/declarations.wap | follows |
| stage 2, signature rules | not built; lowering by hand, cases in bind, type and emit | drifted; the planned place (before binding) cannot work: decision 2 |
| one binding analysis, facts only | one analysis: yes. It also rewrites `x is w` (core/analyze/scope.wap:131) | small drift |
| typing learns facts, never rewrites | it elaborates: rewrites throughout | the drift is right, the plan's sentence wrong: fix 4 |
| stage 3, typed rules | built, used by no rule, retypes the whole program | on paper; unproven: fix 10 |
| kernel forms as named patterns | wap/kernel/forms.wap, 16 patterns, each also its builder | follows |
| a table of primitives | missing; six tables in the emitter | drifted: fix 6 |
| the emitter accepts only the kernel | yes (core/emit/emitter.wap:129) | follows |
| the emitter reads types as facts | it infers them again and emits lazily | drifted: fix 5 |
| runtime words written in warp | texts yes; lists, maps, equality, printing in hand-assembled wasm | drifted: fix 9 |
| function values: a pass of their own (emit/closures.wap) | inside typing, one type per value | drifted: fix 3 |
| facts live in Meta | yes, `with_fact` and `fact_of` everywhere; five global tables beside | follows; fix 7 |
| core knows no built-in by name | about thirty words by spelling | drifted: fix 8 |
| every stage total, refusals loud | yes: a catch-all refusal per stage, no `try` in core, one `refused` | follows |
| a language per stage (nanopass) | implied by the catch-alls; no stage states or checks its output | missing: fix 5 |
| one file one job, about 300 lines | sizes follow since the splits of 2026-11-18; two grab-bags remain (core/stage.wap, core/rules/stages.wap) | follows; fix 11 |

Where the drift is better than the plan: typing as elaboration; refusing at once instead of carrying Error
nodes; a file seeing only the modules it uses itself (cleaner than the old compiler).

Where the plan itself is wrong or silent:

1. "type never rewrites" (notes/backbone.md:115-116) contradicts the plan's own typed kernel (:105-110).
2. Stage 2 placed before binding cannot decide what depends on scope (decision 2).
3. The promise that a program cannot write kernel forms (notes/kernel.md, wap/kernel/forms.wap:7-8) contradicts
   "whoever built it" (notes/first_principles.md:52) (decision 3).
4. One type per function value is written down as a debt for later (notes/backbone.md:308-313); the compiler's own
   source needs it paid first (fix 3).
5. Milestone 2 is one line, "The new compiler builds itself." (notes/backbone.md:190), with no test for done.
6. Nothing in the plan says what wap's own source may use. That is decision 1, and it should have been the
   plan's first paragraph.

## The road to self-build

**Where it stands.** 50 program files. wap types 11, none of which holds a function. In front: 9 distinct first
refusals. Behind them, uncounted: the seven kinds of decision 1 (several hundred places), the six defects of fix
2, words of the old compiler that wap's preload lacks (`float_bits`, `hint`, `split`, `is_letter` of a character,
`normalize_path`, `path_join`, `dirname`, `use path`, `program.folder`, `args`, `write_bytes`), and `[if …]`
three times.

**Is the current path the shortest? No.** The team adds language in the order the census shows one refusal per
file, plus breadth of its own choosing. The features of the last day: protocols, `any` with dispatch at run time,
named arguments, defaults, the shortest text of a float (184 lines), powers, `map_remove`, `x as T`, sorting by
key. Of these wap's own source needs almost none; what it needs (`found: T? = ø`, an early return, an empty map,
a table of functions) is refused. Breadth is cheap to add later on a compiler that builds itself and expensive
now: every feature added before the back half is cleaned has to be added to fifteen fit relations and two type
systems.

**The shortest path, in order:**

1. Take decision 1. Every day without it is spent on the wrong queue.
2. Fix 1: the gauge. Half a day, and it makes every later step measurable.
3. Fix 2 and the missing words of the preload. A day; wap's most basic helpers then type.
4. Compile wap with wap from the leaves of the use graph upward: opcodes.wap (done) → wasm.wap → errors.wap and
   stage.wap → lexemes.wap → the rest of parse/ → rules/ → analyze/ → emit/. Per file: add its `use` lines,
   write the types decision 1 asks for, compile, compare what the file's functions give under both compilers,
   add its line to test_self.sh. Files that use each other in a circle count as one step, which is one more
   reason for the layering rule of fix 11.
5. In parallel, fix 3 (function values): needed from stage.wap on, since every stage walks nodes with a function.
6. Fix 4's fit relation before analyze/ and emit/ reach themselves: there is no point in writing types into
   fifteen functions that are about to become one. Fix 5 while the emitter's files are being typed anyway.
7. After the fixpoint: fixes 6 to 10, decision 2 option B, and then breadth, measured by the old test corpus.

**The biggest risk** is the one in the verdict: an uncounted gap between the language wap is written in and the
language wap accepts, hidden by a gauge that stops at the first refusal. The second: a slow loop. If wap built by
wap needs minutes to build wap, five workers stand still; measure it at step 4, not at the end. The third is a
gift in disguise: the old compiler's looseness hides real mistakes in wap's source (core/format.wap:124 calls
an `is_letter` that core/analyze/letters.wap:14 redefines with other parameters). Typing wap with wap will find
them.

**A definition of done for milestone 2** (notes/backbone.md:190 has none):

1. wap, run by the old compiler, compiles wap/compiler.wap to wap1.wasm.
2. wap1 compiles wap/compiler.wap to wap2.wasm, wap2 compiles it to wap3.wasm, and wap2 and wap3 are the same
   bytes.
3. test/wap passes with wap2 as the compiler.
4. Step 2 takes under a minute on the Mac.
5. wap's source uses no word that only the old compiler knows.

**The workers.** Today five sessions own lists of files that follow history, not stages (notes/wap.md:142-146):
the reader's owner also has the driver of the typed rules, two sugar files and four preload files; the operators'
owner has the rule engine, the formatter, core/analyze/data.wap and four sugar files; the emitter's owner two
sugar files; one worker alone has all of binding and typing, the hardest 1500 lines; and the two notes disagree
on who owns typing. "Only the owner edits it; anyone else sends the owner the change" (notes/wap.md:135-138): so
a feature that crosses stages is cut into requests and each owner patches its own stage, which is how a
character becomes a text three times in typing and once more in the emitter. And nobody owns the goal.

What I would change:

- **Lead: the queue and the stage table.** Each morning the ranked refusal list of fix 1 decides the work. Every
  landing is reviewed against two questions: which function of wap did this turn green, and does core know a new
  word by its spelling. Until milestone 2 nothing lands that turns nothing green.
- **Dogfood: a new role, for the strongest worker.** It compiles wap with wap, file by file (step 4). It may edit
  any file of wap to write its types and its `use` lines, reports each defect it meets as the smallest program
  that shows it, and owns test_self.sh.
- **Front: reader, operators, rules, lowering and resolve under one owner.** The front is nearly done; this
  worker takes fix 7, decision 2, fix 10 and the preload's missing words.
- **Types: binding and typing.** Fix 2, the fit relation of fix 4, the typing side of fix 3. This is the
  critical path: give it the second-strongest worker and no side work.
- **Back: emitter and runtime.** The emitting side of fix 3, then fixes 5, 6 and 9.
- **Ownership becomes review duty.** In 7000 lines nobody needs a lock: anyone edits any file on a branch, the
  owner of the stage reviews. What is exclusive is the stage's *contract*: the check of fix 5 that says what a
  stage's output is.

## Dynamic types: how to build them (user, 2026-11-18)

The design is the user's and is not in question: warp is optionally typed, dynamic where a type cannot be inferred,
and the compiler's own source stays fully typed. wap today refuses where it cannot infer. The advice below is how to
get from one to the other without a second type system. Each point has a `# REVIEW` remark at the place it names.

1. **`any` is the dynamic type, and inference falls back to it.** `joined_type` (core/analyze/fits.wap) gives ø
   when two types do not meet, and its callers refuse ("a list's items are of one type"). It should give `any`.
   A name whose type is never settled is an `any`. That is the whole of "dynamic where we can't infer".
2. **One relation, `conversion(given, wanted)`, with one new answer.** Today "does it fit" is yes or no in more
   than fifteen functions (fix 4). It should say how: as it is, boxed into `any`, wrapped into `T?`, **checked out
   of `any`**, or not at all. The checked conversion is the only new machinery dynamic typing needs: a cast that
   raises with both types and the place. Typing writes the conversion into the kernel; the emitter emits it.
   Today the emitter meets that case and gives ø instead (core/emit/values.wap, `fitted`): fix this first.
3. **Calls on dynamic values are elaboration, not an emitter feature.** `show(x: any) := prim dispatched(x)`
   (wap/preload/prelude.wap, core/analyze/types.wap) is the idea, by hand, for one word and one parameter. General
   rule: a call with an `any` argument that no overload takes as `any` makes typing write the instance `f(any)`
   once, as plain kernel: a chain of `is` tests over the overloads, most specific first, each arm a checked
   conversion and a static call, the last arm a raise. The order is the one the static choice uses, from one
   function. A field read on an `any` is the same chain over the classes that have the field. No new kernel form,
   no new emitter code; `dispatched`, the OVERLOADS fact and the hand-assembled `any_equality_function` go.
4. **Containers hold references.** A `list<int>` is an array of i64 and a `list<Node>` an array of references
   (core/emit/lists.wap), so a typed list can stand where a bare `list` is wanted only by a copy, and lists are
   shared (`b = a` ⏎ `b.add(2)` changes `a`, probe y01), so a copy changes meaning. With every list and map
   holding references (small ints as i31, larger ones boxed), `list<int>` is a promise typing checks with a cast
   at the read; any list fits a bare `list`; the list words are compiled once instead of once per item type. wap's
   own lists hold Nodes and texts and pay nothing. This replaces decision 1: a bare `list` is a list of anything,
   and what was option A (write the item types into wap's source) is only the rule that the compiler's source
   stays typed.
5. **Say what was left dynamic.** `warp format` prints an inferred `any` where it matters, and one option lists
   the dynamic places of a file; the self-build runs with it and expects none.
6. **Order of work.** (a) point 2's checked cast in place of the ø; (b) `conversion` in fits.wap, callers moved
   one construct at a time, each with a test; (c) the fallback to `any`; (d) the generated `f(any)`; (e) the
   layout of containers, the one large change, best before the emitter's second pass (fix 5).

Found on the way, and a test of point 3: `[1, 2] == [1, 2]` and `Point(1, 2) == Point(1, 2)` compile and run out
of stack (probes y05, y07). No `==` takes two lists or two instances, so the choice falls to the `==` of optionals
(wap/preload/prelude.wap), whose `a! == b!` chooses itself again. No test compares two lists.

## What to keep

- The reader: pure, two passes, layout and brackets by one cascade, and tested by what it reads.
- Operators as one table of data in the preload, with a refusal for a spelling the table does not hold.
- The rule engine: a head index, the most specific rule wins, fresh names for the names a rule makes, a bound on
  rounds, a rule that may refuse a spelling with a fix-it (core/rules/stages.wap:135).
- Kernel forms as sixteen named patterns that are also their builders.
- One `refused` for every stage (core/errors.wap:13), a catch-all refusal per stage, no `try` in core.
- Refusals that name both readings: "this could change the global total or make a new variable here: write `let
  total = …` for a new one, `global total = …` for the program's" (probe p18). That is the standard for all.
- The check that a `match` covers every case (core/analyze/matching.wap:57).
- The text words written in warp over three primitives (wap/preload/texts.wap), the model for fix 9.
- core/emit/wasm.wap, a module builder that knows nothing of warp.
- The "Debts" list of notes/backbone.md: candid, and nearly every structural finding here starts from one of
  its entries. What it lacks is an order; this review proposes one.

## Limits of this review

The old compiler was not reviewed. The old test corpus was not run under wap. Timings were taken once, at
bc30daa9c8, on a Mac that other sessions were using. The programs and the census are those of 73b39f4ad5;
programs p to u ran before at bc30daa9c8 or 6fa64cf537 with the same results, t01 to t06 written anew for the
second run. test/wap/run.sh was run at bc30daa9c8 only. The causes named in fix 13 are read from the code, not
profiled.

## Appendix A: the programs run

`⏎` stands for a new line. "refused" quotes wap's message without the position.

| id | program | wap |
|---|---|---|
| p01 | `f() as list := [1, 2]` ⏎ `print text(f()#1)` | refused: this gives list<int>, the function says it gives a list |
| p02 | `class S { xs: list }` ⏎ `s = S([1, 2])` | refused: this gives list<int>, the field holds a list |
| p03 | `entry = ["a", 1]` | refused: a list's items are of one type |
| p04 | a lookup over `[["a", 1], ["b", 2]]` | refused: the same |
| p05 | `class Test { fit: function }` ⏎ `t = Test(x => x > 1)` ⏎ `t.fit(3)` | refused: nothing defines this name: fit |
| p06 | `HELPERS = {double: x => x * 2, triple: x => x * 3}` | refused: a map's values are of one type |
| p07 | `fs = [x => x + 1, x => x * 2]` | refused: a list's items are of one type |
| p08 | `pick(c: bool) := if c then (x => x + 1) else (x => x * 2)` ⏎ `pick(yes)(5)` | refused: this holds a nothing, no function to call: (Call pick yes) |
| p09 | `n = Symbol("a")` ⏎ `print match n { Symbol(name) => name; _ => "other" }` | `a` |
| p10 | a global `count = [0]` raised by a function | `2` |
| p11 | `class Rules { by_head: map }` ⏎ `r = Rules({})` | refused: no Rules takes (ø) |
| p12 | `(Prim print_text "x")` | wasm that fails to load: expected (ref null $type), found i32 |
| p13 | `(If yes (print "a") (print "b"))` | `a` |
| p14 | `(Call print "hi")` | `hi` |
| p15 | `names(nodes: list) as list := nodes.map(n => match n { Symbol(name) => name; _ => "?" })` | refused: no map takes (list<Node>, fn<…λ>) |
| p16 | `class Entry { key: text; value: int }` ⏎ `keyed(key: text, entries: list<Entry>) as int? := { for entry in entries { if entry.key == key { return entry.value } }; ø }` | refused: this function gives back values of different types |
| p17 | `apply(f: function, x: int) := f(x)` ⏎ `apply(x => x + 1, 4)` | refused: no apply takes (fn<…λ>, int) |
| p18 | a function value that sets a variable of the program | refused, naming both readings (`let total`, `global total`) |
| q01 | p15 with `"other"` and `as list<text>` | `a,other` |
| q02 | `name(n: Node) as text := match n { Symbol(name) => name; _ => "?" }` | runs |
| q03 | `first_big(xs: list<int>) as int? := { for x in xs { if x > 1 { return x } }; ø }` | refused: this function gives back values of different types |
| q04 | `less_one(x: int) as int? := { if x < 0 { return ø }; x - 1 }` | refused: the same |
| q05 | `entry: list<any> = ["ab", 1]` ⏎ `key = entry#1` ⏎ `if key is text { print key }` | `ab` |
| q06 | `apply(f: (int) -> int, x: int) := f(x)` ⏎ `print text(apply(x => x + 1, 4))` | `5` |
| q07 | `class Test { fit: (int) -> bool }` ⏎ `t = Test(x => x > 1)` | refused: this gives fn<…λ>, the field holds a (Call -> (int) bool) |
| q08 | two function values for the field of q07 | refused: the same |
| q09 | `rest(xs: list) := xs.drop(1)` ⏎ `print show(rest([1, 2]))` | `[2]` |
| q10 | `size(n: Node) as int := match n { List(items, bracket, separator) => 1 + items.map(size).sum(); …; _ => 1 }` | `5` |
| q11 | `class Rules { by_head: map<text, int> }` ⏎ `Rules({})` | refused: no Rules takes (ø) |
| q12 | `[1, 2].map(double)` | `[2, 4]` |
| q13 | a chain of `??` over optionals | `8` |
| q14 | `shout(n: int) := prim print_text(n)` ⏎ `shout(3)` | wasm that fails to load: expected (ref null $type), found i64 |
| q15 | a `while` loop joining texts | `012` |
| q16 | a class with a list field given a default and a method adding to it | `2` |
| r01 | p15 with `as list<text>`, the arm still `"?"` | refused as p15 |
| r02 | `m: map<text, int> = {}` | refused: this gives a ø, the name holds a map<text, int> |
| r03 | `m = {}` ⏎ `m.a = 1` | refused: no field or function a for a value of type α1 ? |
| r04 | `class Reader { indents: list = [] }` | refused: this gives list<α1>, the field holds a list |
| r05 | `pick(c: bool, name: text) := if c then name else "?"` | runs |
| r06 | `["=", ":=", ":"].join(" ")` | `= := :` |
| s01 | `enum Shape { Block(w: int), Dot }`, `Block(3)` matched | `3` |
| s02 | `class Item { name: text }` ⏎ `i = Item("pen")` ⏎ `print i.name` | `pen` |
| s03 | `Set(x: int) := x + 1` | `3` |
| s04 | `class Record { title: text }` ⏎ `print Record("a b").title` | `a b` |
| t01 | `found: int? = ø` ⏎ `for x in [1, 2, 3] { if x > 1 { found = x } }` | refused: this gives a int, the name holds a ø |
| t02 | the same inside a `match` arm of a function | refused: the same |
| t03 | `enum Color { Red, None }` ⏎ `enum Kind { Atom, Color }` ⏎ `c = Color.None` | refused: no field or function None for a value of type Kind |
| t04 | `c = yes` ⏎ `xs = [if c then 1 else 2]` | refused: nothing defines this name: if |
| t05 | `found: int? = ø` ⏎ `found = 3` | refused: this gives a int, the name holds a ø |
| t06 | the same inside a function's `if` | refused: the same |
| u01 | `xs = ["a", "b"]` ⏎ `xs += ["cd"]` | refused: no + takes (list<char>, list<text>) |
| u02 | `print ["a", "b"].join(",")` | `a,b` |
| u03 | `x = "a"` ⏎ `print x + "bc"` ⏎ `print text(#x)` | `abc`, `1` |
| v01 | `double(x: int) := x * 2` ⏎ `xs = [1, 2, 3]` ⏎ `xs[1 + double(1)] = 9` | refused: the binding analysis reads only kernel forms; the lowering should have removed this: double(1) |
| v02 | `sign(x: text) := "$" + x` ⏎ `bindings = {"$a": 1}` ⏎ `bindings["$" + sign("b")] = 2` | refused: the same, of sign('b') |
| v03 | v01 with `xs[double(1)] = 9` | refused: the same |
| v04 | v01 with the index read: `print text(xs[0 + double(1)])` | `3` |
| v05 | v01 with `xs#(1 + double(1)) = 9` ⏎ `print show(xs)` | `[1, 2, 9]` |

Under the old compiler: `apply(f: (int) -> int, x: int)` runs (h01); typed lists, a field `entries: list<Entry> =
[]`, `as int?` with an early return and `as list<int>` run (h02); `class Test { fit: (int) -> bool }` with `tests =
[Test(x => x > 1), Test(x => x < 1)]` is refused, "an instance of Test given more values than fields" (h03).

## Appendix B: the census at 73b39f4ad5

`warp scripts/wap-census.warp <file>` on each of the 50 files.

- Typed and emitted, 11: wap/kernel/forms.wap (16 patterns), wap/core/emit/opcodes.wap (120 constants), the 9
  files of wap/core/sugar/ (rules; records.wap with one statement it does not lower).
- Not typed, 39. The refusal that stops the typing of each, by how many files it stops:

| files | where typing stops | what |
|---|---|---|
| 14 | core/analyze/data.wap:27 | `field_checked`, of fits.wap, which data.wap does not use (new with the split) |
| 9 | core/emit/wasm.wap:59 | `float_bits`, a word of the old compiler |
| 6 | core/parse/operator_table.wap:155 | `Use`, a class of operators.wap, which uses operator_table.wap |
| 4 | core/parse/layout.wap:10 | `Separator.None` read as a field of the case `Separator` (fix 2.4) |
| 2 | core/stage.wap:66 | `found: Node? = ø` then `found = value` (fix 2.1) |
| 1 | core/stage.wap:21 | a parameter typed `function` called (fix 2.5) |
| 1 | core/rules/lowering.wap:83 | `pattern_called`, of rules/patterns.wap, not used by lowering.wap |
| 1 | core/analyze/letters.wap:100 | `same_type`, of fits.wap, which letters.wap does not use (new with the split) |
| 1 | core/analyze/declarations.wap:23 | `use path`, a library module named by a bare word |

- The same census five commits earlier, at 6fa64cf537 (43 files, 32 not typed): `float_bits` 9, `is_named` of
  scope.wap in calls.wap 8, `tokens_of` in operator_table.wap 5, `Separator.None` 4, `found` 2, and one each for
  `rewrite`, `pattern_called`, `[if … then … else …]` and `use path`. Three of the nine entries changed within the
  day and one of the three was fixed (`tokens_of` got its `use`): `is_named` (core/analyze/calls.wap:30) and
  `[if …]` (core/analyze/matching.wap:68) are written as they were and wait behind the new first refusals.
- Names wap cannot bind in a file because their module is not used by that file itself: in 20 files. 8 of
  core/emit/ (the opcodes); 7 of core/analyze/ (calls.wap: `type_test_typed`, `is_named`; conditions.wap:
  `joined_settling`, `same_type`; data.wap: `field_checked`, `unknown_type`; fits.wap: `settles`, `refused`;
  letters.wap: `same_type`; matching.wap: `joined_type`, `same_type`; types.wap: `all_settled`);
  core/parse/operator_table.wap (`Use`); core/rules/lowering.wap (`pattern_called`), patterns.wap
  (`shaped_program`), stages.wap (`top_level_statements`) and templates.wap (`REST_MARK`, `phrase_of`).
- A form wap does not lower, in its own source: the place `bindings[HOLE_SIGN + hole_variable(…)] = …`
  (core/rules/patterns.wap:148; the sixth defect of fix 2).
- Words of the old compiler met on the way: `float_bits`, `hint` (core/parse/operators.wap:119), `is_letter` of a
  character (core/format.wap:124), `normalize_path`, `path_join`, `dirname` (core/analyze/declarations.wap:35),
  `split` (core/analyze/calls.wap:219), `program`, `args` (wap/compiler.wap:24, :27).
