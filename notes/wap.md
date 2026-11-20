# wap: the file tree of the new compiler

This is the specification of wap's files. A file is created only where this tree has a place for it; a file that
does not fit any place means the tree is wrong, and the tree is changed here first, then the code. Plan and
principles: notes/backbone.md. The kernel: notes/kernel.md. Rules for every file: notes/first_principles.md.

wap is the new compiler; warp, in core/, is the old one, frozen. wap is an ordinary warp program, compiled by warp
until it compiles itself, so its names never clash with warp's compiler. Its sources end in `.wap`; the language they
are written in is warp.

## The pipeline

A program passes through stages. Each stage has one job, reads one declared language and writes one declared
language (nanopass style). A stage that meets a form outside its input language refuses it loudly, naming the form
and the stage that should have removed it.

```
text
 │ read                       core/parse/        the notation: data and code alike, no keyword meanings
 ▼
notation (Node: atoms, keys, lists, phrases)
 │ shape rules                core/rules/        stage 1: rules that ask only the shape
 ▼
surface (any warp)
 │ declare                    core/analyze/declarations.wap
 ▼
surface + declarations
 │ declaration rules          core/rules/declared.wap  stage 2: rules that ask the declarations in force
 ▼
surface + declarations
 │ lower                      core/rules/lowering.wap   writes the kernel forms: glued, operator and phrase calls,
 │                                              := = =>, data and blocks; a phrase stays a call, typing decides it
 ▼
kernel (kernel/forms.wap)
 │ scope, typing, kind rules   core/analyze/, core/rules/typed.wap   stage 3 fires on each node as typed
 ▼
typed kernel
 │ emit                       core/emit/
 ▼
module → bytes
```

Every stage runs through one walker (core/stage.wap), which also carries the metadata: a node a stage builds gets
the position of the node it rewrote, and the name of the rule or stage that built it. Nodes reused from the input
(the holes of a pattern) keep their own. No code ever attaches a position by hand.

## The tree

Layers as in notes/backbone.md: kernel, core, preload (its sugar/ the surface rules every program gets), extensions,
library. A folder holds one topic; its name is that topic,
never two joined by an underscore.

```
wap/
  compiler.wap                the entry: the stages in order (core/pipeline.wap), the command line, nothing else
                              warp wap/compiler.wap <program> <module.wasm>

  kernel/                     the kernel layer: the emitter's whole input language
    forms.wap                 the kernel forms, each a named pattern of Node

  core/                       the machinery: mechanisms only, no names of a domain
    pipeline.wap              one function per step from source to typed program, shared by compiler and census
    language.wap              the language a program is read in: operator table, rule words, the rules of each stage
    stage.wap                 the one walker every stage runs through; positions and provenance of new nodes
    bindings.wap              the facts binding learns of a name: what it refers to, assignments, captures
    type_notation.wap         types as written: list<int>, int?, (int) -> text, built and taken apart
    errors.wap                refusals: the user's line, the readings, the fix-it, the rule that expanded it
    format.wap                code → canonical warp text (`format`): operators in their notation, spacing spelled out
    paths.wap                 a used file's path beside the file using it, `.` and `..` resolved

    parse/                    text → notation → operated
      reader.wap              the pure notation reader: atoms, keys, lists, phrases
      lexemes.wap             the reader's state and its tokens
      atoms.wap               pass 1: names, numbers, glyphs, texts with their holes
      layout.wap              pass 1: brackets, separators, indentation
      cascade.wap             pass 2: tokens → notation by the separator cascade
      operators.wap           notation → operated by the table of preload/operators.wap: precedence groups sorted
                              topologically, the minus rule, aliases to canonical words; unrelated mixes refused
      operator_table.wap      the operator table as the operator stage reads it
      tokens.wap              a phrase's tokens as the operator stage reads them: what each touches

    rules/                    the rule engine
      patterns.wap            a rule's pattern declared and compiled to a Matcher: holes, typed holes, rests, guards
      matcher.wap             running a compiled Matcher against code, its holes bound; and specificity
      templates.wap           building a rule's output: splicing, fresh names, free words resolved in the rule's module
      stages.wap              shape rules (stage 1): rules indexed by head, run over operated code
      termination.wap         the rules checked to end: no chain of them rewrites forever
      lines.wap               a program's lines as the rules read them: a line led by a rule word
      declared.wap            declaration rules (stage 2): holes that ask the declarations, after declare
      typed.wap               typed rules (stage 3): holes of value types, fired in the type walk, typed in place
      lowering.wap            declared → kernel by hand: phrases, statements, the forms no rule writes yet

    analyze/                  facts about the program, no rewriting
      declarations.wap        the prescan: the names, signatures and types a program declares
      definers.wap            the ids of the names that define something, given by the prescan and binding
      modules.wap             the modules a program uses, by file name, as binding sees them
      scope.wap               the one binding analysis: what each name refers to
      library.wap             the preload bound but for its functions, each bound once typing reaches it
      names.wap               what a name may mean: the scope's layers, each finding its definitions by name
      variables.wap           which variable each assignment gives a value to, in written order; late reads
      types.wap               bidirectional typing of kernel code; the dispatch over forms
      calls.wap               a call's function chosen among its overloads, typed once per tuple of argument types
      letters.wap             generic letters a call binds; an empty list's unknown item type, settled by a use
      dispatch.wap            calls on dynamic values: a run-time choice where the type is not known
      conditions.wap          conditions, the narrowing they give their arms, type tests
      matching.wap            match: patterns typed, whether the arms fit every value
      shapes.wap              shapes as types: an enum's case, `List` of Node, a declared pattern
      fits.wap                how types meet: fitting, sameness, the joined type of two
      data.wap                lists, maps and instances written as data
      function_values.wap     function values and what they capture

    emit/                     typed kernel → bytes
      emitter.wap             one case per kernel form; any other node refused; blocks, if, loops, labels
      functions.wap           definitions waiting for their first call, calls
      function_values.wap     Fn: function values, their captures, calls of them
      patterns.wap            Case: arms, pattern tests, a case's own value
      places.wap              a typed form's layout, fields and items reached, variables
      primitives.wap          what each Prim operation emits; a longer one is a helper function of the module
      host.wap                the primitives that ask the host (WASI): printing, raising, files, the command line
      lists.wap               growable lists and the maps built on them
      equality.wap            whether two values of one type are equal, for `==` and a map's key search
      values.wap              the value layout (notes/self_hosting.md "Value layout"), reused as it is
      wasm.wap                the wasm binary format: modules, sections, instructions; knows no warp form
      opcodes.wap             the format's codes: section ids, value and heap types, opcodes

  preload/                    the standard language every program gets: whole definitions, data and rules
    preload.wap               the word files below, included in place: the one library every program is bound in
    rules.wap                 the kernel forms' patterns and the sugar/ files, used in load order
    prelude.wap               the built-ins (print …), each a function whose body may be a Prim
    operators.wap             the operator table: precedence groups and their relations, operators, aliases
    nodes.wap                 warp's Node, case for case as core/node.warp declares it, and the words over it
    lists.wap                 list words over function values: map, filter, contains …
    texts.wap                 text words over a text's UTF-8 bytes
    numbers.wap               number words over the prelude's comparisons
    floats.wap                float, the fast variant of exact real numbers
    host.wap                  the host machinery the file and command-line words use, hidden from programs
    sugar/                    the rules of the surface spellings wap's own source uses, one file per topic
      calls.wap               x.f(y), x.f → calls and Get
      conditions.wap          unless, a if c, a ? b : c → If
      data.wap                xs#2 and other reaches → Get
      definitions.wap         definitions written other ways
      literals.wap            yes, no, ø → True, False, Empty; no x → x is empty
      loops.wap               for, until, loop, n times, break, return → While and Return
      matching.wap            match x { … } → Case
      ranges.wap              a to b, a upto b → Range
      records.wap             class, enum → Record
      types.wap               list of int, x as text → bracketed types and conversions

  extensions/                 rule files no compiler needs, one per topic, loaded like modules: no hook table; none yet
                              (units 3km, durations 2 weeks, inline html … as the forms arrive)

lib/                          the library, warp's own, shared and used as it is
test/wap/                     wap's outcome tests: a program compiled by wap, run, its output checked
```

What each file may know:
- **kernel/** knows Node and nothing of the stages.
- **core/parse/** knows characters and the operator table, no keyword: `if`, `for` and `class` are words to it.
- **core/rules/** knows patterns and templates, no particular form: every form it handles comes from a rule file.
- **core/analyze/** reads, never rewrites.
- **core/emit/** reads only the kernel; `wasm.wap` reads not even that.
- **preload/sugar/** and **extensions/** hold rules only; a rule that needs code calls a function in lib/.

Which of the two a rule file belongs in is measured, not argued: wap compiles itself with extensions/ switched off.
A rule file is in preload/sugar/ exactly when wap's own source uses its spelling; when that changes, the file moves.

## Owners

Each wap file has one expert owner, whose duties are notes/agents/wap-owner.md (user 2026-11-18); ownership changes
only through warp-lead, in this table. Owners go by stage (user 2026-11-18, notes/wap_review.md "The workers").
Anyone may edit any wap file on a branch; the owner of the file's stage reviews the change before the handoff, and owns the stage's contract: what its output
language is (notes/backbone.md "Stage languages"). Breadth stops until milestone 2: wap gets only what its own
source uses, and every landing names the functions of wap it turned green (scripts/wap-census.warp). Tests
(test/wap/) and notes/backbone.md "Debts" are append-only, open to all. A bug of the old compiler that blocks wap
goes to warp-host-fixer as a repro; work around it or go on meanwhile. A new file belongs to the stage of its folder.

Since wap compiles itself (2026-11-20), wap work comes from notes/backbone.md "Architecture changes, in order" and the
open milestone-2 points (user 2026-11-20); anything else is one line in that file's "Debts".

Trim before handoff (user 2026-11-18): before handing a branch off, try deleting each of its edits, one at a time, after
the change is done; every edit whose removal keeps test/wap/run.sh and test/wap/test_guardrails.sh green goes. Only
deletions, nothing rewritten; the handoff names what was trimmed.

| stage | owner | files (under wap/) |
|---|---|---|
| lead: the queue, the contracts | warp-lead | compiler.wap, core/pipeline.wap, core/stage.wap, core/bindings.wap, core/type_notation.wap, core/errors.wap, kernel/forms.wap; the test/wap/ scripts no other row names |
| self-build: wap compiled by wap, file by file | warp-wap-operators | test/wap/test_self.sh, test/wap/ratchet_census.sh, scripts/wap-census.warp; edits any file to write its types and `use` lines |
| front: read, operators, rules, lowering | warp-wap-reader | core/parse/, core/rules/, core/format.wap, core/language.wap, preload/ |
| types: declare, bind, type | warp-wap-binding | core/analyze/, core/paths.wap |
| back: emitter, runtime | warp-wap-emitter | core/emit/; test/wap/test_guardrails.sh, test/wap/guardrail_shapes.awk, test/wap/guardrails_allowed.txt |
| simplification: wap smaller by its Node count (Meta excluded) | warp-simplifier | any file, by review of its stage owner |

## Rules for every file

- One job, said in its first comment line; a second job is a second file in its folder.
- About 300 lines at most: a file past that is split by its owner at the next quiet point, along a real seam (one
  concern per file, never a mechanical cut), in a commit of its own that also updates the Owners table.
- Its input and output language named in that comment.
- Total over its input: every case handled or refused loudly, never skipped.
- No surface spelling outside parse/ and extensions/; no name of a domain (units, colors, keys) in core/.
- A compromise is a debt in notes/backbone.md "Debts", never silent.

## State

Milestone 1 (Hello) reached 2026-11-17; milestone 2 (wap builds itself, notes/backbone.md "Milestones") in progress.
Every stage of the pipeline runs (core/pipeline.wap); the sugar's rules write the kernel forms of words
(preload/sugar/), lowering.wap the general ones (calls, definitions, data, blocks).

Tests: `bash test/wap/run.sh` (every test/wap script), `bash test/wap/test_guardrails.sh`.

## Road to self-build

The gauge is scripts/wap-census.warp: it runs the compiler's own stages on every wap file and counts the functions
typed alone; it also ranks the refusals that block the rest, the first a function meets, by family. On 2026-11-19
(main 4d51aa08b2) wap types 997 of 1157 functions, 15 of 69 files whole, and no place of its own source is typed any.
test/wap/ratchet_census.sh holds those numbers: the files typed whole may only rise, the untyped functions (cap 160)
may only fall. Rerun after each landing: `test/wap/run.sh --wait test/wap/ratchet_census.sh`; the census's ranking names what to build next.

## Size

wap's size is the number of nodes its own reader makes of every wap/*.wap file, Meta excluded
(scripts/wap-size.warp): comments, blank lines, line breaks and the length of names count nothing, so only a real
simplification makes it smaller. warp-simplifier lowers it; each landing's commit says the count before → after.
`warp --no-window scripts/wap-size.warp $(git ls-files "wap/*.wap")`

Baseline, origin/main 499e1d6abe (2026-11-18): **82145 nodes**.

| folder | nodes |
|---|---|
| . | 113 |
| core/analyze | 25641 |
| core/emit | 18791 |
| core | 5339 |
| core/parse | 12266 |
| core/rules | 10114 |
| kernel | 189 |
| preload | 8814 |
| preload/sugar | 878 |

The 20 largest top-level statements:

| file and name | nodes |
|---|---|
| wap/core/analyze/scope.wap bound | 717 |
| wap/preload/nodes.wap text | 625 |
| wap/core/analyze/calls.wap call_typed | 584 |
| wap/core/analyze/calls.wap function_typed | 560 |
| wap/core/rules/lowering.wap lowered_form | 534 |
| wap/core/analyze/types.wap typed_form | 509 |
| wap/core/emit/host.wap arguments_function | 478 |
| wap/core/emit/emitter.wap code | 464 |
| wap/preload/floats.wap float_digits | 438 |
| wap/preload/operators.wap operators | 410 |
| wap/core/parse/atoms.wap quoted | 406 |
| wap/core/parse/atoms.wap number_read | 390 |
| wap/core/parse/reader.wap read_token | 390 |
| wap/core/emit/primitives.wap primitive_code | 359 |
| wap/core/parse/operators.wap tokens_read | 354 |
| wap/core/format.wap call_text | 337 |
| wap/core/emit/primitives.wap int_text_function | 324 |
| wap/core/analyze/calls.wap chosen_definition | 308 |
| wap/core/parse/layout.wap indentation | 308 |
| wap/core/emit/host.wap path_opened | 307 |
