# wap: an independent review (2026-11-18)

Reviewer: warp-architect-review, a one-off session with no stake in the design. Subject: the new compiler in wap/
at main 6fa64cf537 (read at bc30daa9c8, every citation re-checked after merging main). `file:line` refers to that
commit. Nothing in wap/ or test/ was changed.

## Verdict

In two days and about 300 commits wap became a working compiler: 6800 lines, ten stages, every outcome check
green. **Its front half is sound and follows the first principles; its back half has grown by accretion; and the
whole effort is aimed at the wrong target.**

- **Front half, keep as it is.** A pure two-pass reader, operators as one table of data in the preload, a rule
  engine with a head index, specificity, hygiene and a termination bound, kernel forms as named patterns over Node,
  built-ins declared in the preload with `prim` bodies, one binding analysis, every refusal through one function,
  a wasm module builder that knows nothing of warp. This is the best code in the repository.
- **Back half, needs a second pass.** Typing (1026 lines) is in fact a rewriting stage; it knows about thirty
  library and surface words by their spelling and answers "does this value fit there" in more than fifteen
  functions that differ by construct. The emitter (1483 lines) infers types a second time on wasm layouts, emits
  functions in the order it meets them, and carries about 340 lines of hand-assembled runtime. The plan's middle
  rule stage was never built; its work sits in four files. The third rule stage is built but no rule of the
  language uses it.
- **Direction.** The milestone is "wap builds itself", and wap is not measured against it. It gains language
  breadth (protocols, `any`, named arguments, shortest float text) while it refuses the way its own source is
  written. Of the 30 files of wap that hold functions, wap types **none** today. The census that should show this
  stops at each file's first refusal, so the team sees eight missing words where there are several hundred places
  wap would refuse, its own most used helper (`fact_of`) among them.

My main recommendation: stop adding language, decide what wap's own source may look like (decision 1), fix the
five small typing defects that block wap's most basic code, and put one worker on compiling wap with wap file by
file, with a gauge that counts every refusal instead of the first.

## How this was checked

- Read: notes/first_principles.md, backbone.md, kernel.md, reader.md, wap.md; all of wap/ (50 files: parse 1294
  lines, rules 914, analyze 1514, emit 1483, sugar 95, core root 483, kernel 74, compiler.wap 28, preload 936);
  all 27 scripts of test/wap; scripts/wap-census.warp.
- Ran once: `bash test/wap/run.sh --wait` at bc30daa9c8: 376 ✓, 0 ✗ in 27 scripts, 82 s.
- Ran 57 small programs through wap to check each claim below that says "refused", "runs" or "invalid wasm"
  (appendix A), three programs through the old compiler, and the census on all 41 program files of wap
  (appendix B). Every such claim was run, not inferred.

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

wap's source is written in the old compiler's loose dialect: 318 parameters and fields typed bare `list`, 178
results written `as list`, about 40 class fields `xs: list`, 9 parameters typed `function`, 4 tables of functions,
10 places that build `[key, value]` pairs of two types, 5 global tables starting as `{}`. wap's type system is
static: every list has one item type, a function is typed once per tuple of argument types. It accepts a bare
`list` only at a parameter ("takes its argument's own type", core/analyze/calls.wap:386-389). Everything else is
refused:

| written in wap's source | example | what wap says |
|---|---|---|
| result `as list` | `joined(a: list, b: list) as list` (core/emit/emitter.wap:159) | "this gives list<int>, the function says it gives a list" |
| field `xs: list` | `class Reader { code: list; … indents: list = [] }` (core/parse/lexemes.wap:11) | "this gives list<α1>, the field holds a list" |
| a pair of two types | `[[entry#1, …]]` (core/analyze/calls.wap:229-231) | "a list's items are of one type" |
| a parameter `function` | `staged(n: Node, rewrite: function)` (core/stage.wap:21) | "this holds a function, no function to call: rewrite" |
| a table of functions | `HELPERS = {"print_text": printing_function, …}` (core/emit/primitives.wap:78) | "a map's values are of one type" |
| an empty map | `shape_rules = Rules({})` (core/rules/stages.wap:36) | "no Rules takes (ø)" |
| a name from a module used by a used module | `IMMUTABLE` in core/emit/values.wap (declared in opcodes.wap, reached through wasm.wap) | "nothing defines this name" |

notes/backbone.md:331-334 notes that "a bare `list` is unreadable to wap" for one file and draws no consequence;
notes/wap.md:167-184 lists missing library words as the blockers.

- **A (recommended): wap stays static, and a type written without its parts takes the type of the value given.**
  That is today's rule for parameters, extended to results and variables: `as list` means "a list, of whatever
  items the body gives". A class field and the result of a function that calls itself need the full type and are
  refused with that fix-it. wap's source then changes in about 40 fields (`indents: list<int>`), 10 pair lists
  that become small classes (better code: `entry.key` instead of `entry#1`), and the missing `use` lines. Tables
  of functions, empty maps and `function` parameters are compiler fixes, not source edits (fixes 2 and 3).
  Core cost: about 10 lines (core/analyze/calls.wap:377 and :389, one message). The old compiler accepts the
  typed spelling (probes h01, h02), so both compilers build the same source on the way.
- **B: bare means dynamic, `list` is `list<any>`**, items read back by run-time checks (the reading of
  backbone.md:333). wap's source compiles nearly as it is. Core cost: several hundred lines (a checked downcast
  wherever an `any` meets a typed place, in typing and emitter), every byte list of the emitter boxed, type
  errors moved to run time, and the language gets two regimes side by side.
- **C: every type written out in wap's source, no new rule.** Core cost 0; about 220 more source edits than A
  (the 178 results); the most explicit source.
- **Park:** continue file by file. Cost: the distance to self-build stays unmeasured.

Why A: it is the rule wap already has, said once instead of at parameters only; it keeps every type error at
compile time; and the edits it asks of wap's source make that source better.

Sub-point, same decision: a file sees only the names of the modules it uses itself, not of the modules those use
(core/pipeline.wap:40). The old compiler passes them on, and wap's source relies on it in dozens of places (17
of the 41 census lines). Keep wap's rule (it is what Python, JavaScript and Rust do), add the `use` lines, and let
the refusal name the module: "IMMUTABLE is opcodes.wap's: add `use "opcodes.wap"`".

### Decision 2. Where do readings that depend on what a name means live?

The plan has three rule stages; the middle one, "signature rules: declared → kernel" (notes/backbone.md:56,
notes/first_principles.md:123-128), was never built. Its work is done by hand in four files:

- core/rules/lowering.wap:65-101: any phrase led by a word becomes a call, without asking what the word is;
- core/analyze/scope.wap:124-126: `x is w` becomes a type test or the call `w(x)`; :283 `Shape.Dot` becomes a case;
- core/analyze/types.wap:307-309: a field named bare in a method becomes `self.field`; the name of a function
  of no parameters becomes its call;
- core/emit/emitter.wap:122: a `Get` that carries a definition is emitted as a call.

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
p14; core/rules/lowering.wap:83, core/rules/stages.wap:300). notes/first_principles.md:52 says so: a pattern
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
fields, list items and generic letters (core/analyze/calls.wap:83, :304, :340; core/analyze/data.wap:50-51) and
missing elsewhere:

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
(core/analyze/types.wap:257, :296, core/rules/stages.wap:333).

- **A (recommended): keep wap's reading and refuse with both readings named**: "write `[(if c then 1 else 2)]` for
  one value". About 2 core lines for the message, 3 source edits.
- **B: a row in square brackets led by a word that leads a rule is one phrase.** About 5 core lines; rows in lists
  then read two ways depending on their first word.
- **Park.**

<!-- part 2 follows: fixes for the team, plan conformance, the road to self-build, appendices -->
