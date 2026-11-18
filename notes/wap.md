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
 │ signature rules            core/rules/        stage 2: rules that ask declared signatures; every phrase decided
 ▼
kernel (kernel/forms.wap)
 │ scope, typing              core/analyze/
 ▼
typed kernel
 │ kind rules                 core/rules/        stage 3: rules that ask inferred types, output re-checked
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
  compiler.wap                the entry and the pipeline: the stages in order, nothing else
                              warp wap/compiler.wap <program> <module.wasm>

  kernel/                     the kernel layer: the emitter's whole input language
    forms.wap                 the kernel forms, each a named pattern of Node
    primitives.wap            the primitives: name → wasm instruction, conversion, constructor, runtime or host word

  preload/                    the standard language every program gets, whole definitions with their bodies
    prelude.wap               the built-ins (print …), each a function whose body may be a Prim

  core/                       the machinery: mechanisms only, no names of a domain
    stage.wap                 the one walker every stage runs through; positions and provenance of new nodes
    errors.wap                refusals: the user's line, the readings, the fix-it, the rule that expanded it
    format.wap                code → canonical warp text (`format`): operators in their notation, spacing spelled out
    paths.wap                 a used file's path beside the file using it, `.` and `..` resolved

    parse/                    text → notation
      reader.wap              the pure notation reader: atoms, keys, lists, phrases (milestone 1: warp's `parse`)
      lexemes.wap             the reader's state and its tokens
      atoms.wap               pass 1: names, numbers, glyphs, texts with their holes
      layout.wap              pass 1: brackets, separators, indentation
      cascade.wap             pass 2: tokens → notation by the separator cascade
      operators.wap           notation → operated by the table of preload/operators.wap: precedence groups sorted
                              topologically, the minus rule, aliases to canonical words; unrelated mixes refused

    rules/                    the rule engine
      patterns.wap            matching a rule's pattern: holes, typed holes, rests, guards
      templates.wap           building a rule's output: splicing, fresh names, free words resolved in the rule's module
      stages.wap              the stage a rule runs in, read off its pattern; rules indexed by head
      lowering.wap            the spellings milestone 1 lowers by hand, until the engine runs the rule files

    analyze/                  facts about the program, no rewriting
      declarations.wap        the prescan: the names, signatures and types a program declares
      scope.wap               the one binding analysis: what each name refers to
      variables.wap           which variable each assignment gives a value to, in written order; late reads
      types.wap               bidirectional typing of kernel code; overloads chosen by parameter or return type
      calls.wap               a call's function chosen among its overloads, typed once per tuple of argument types
      letters.wap             generic letters a call binds; an empty list's unknown item type, settled by a use
      conditions.wap          conditions, the narrowing they give their arms, type tests
      matching.wap            match: patterns typed, whether the arms fit every value
      fits.wap                how types meet: fitting, sameness, the joined type of two
      data.wap                lists, maps and instances written as data
      function_values.wap     function values and what they capture

    emit/                     typed kernel → bytes
      emitter.wap             one case per kernel form; any other node refused; blocks, if, loops, labels
      functions.wap           definitions waiting for their first call, function values, calls
      patterns.wap            Case: arms, pattern tests, a case's own value
      places.wap              a typed form's layout, fields and items reached, variables
      primitives.wap          what each Prim operation emits; a longer one is a helper function of the module
      closures.wap            Fn: closure conversion
      values.wap              the value layout (notes/self_hosting.md "Value layout"), reused as it is
      wasm.wap                the wasm binary format: modules, sections, instructions; knows no warp form

  preload/                    the standard language every program gets: data and rules, no mechanism
    operators.wap             the operator table: precedence groups and their relations, operators, aliases
    sugar/                    the rules of the surface spellings wap's own source uses, one file per topic
      loops.wap               for, until, loop, n times → While
      conditions.wap          unless, a if c, a ? b : c → If

  extensions/                 rule files no compiler needs, one per topic, loaded like modules: no hook table
    units.wap                 3km, 90km/h: a unit multiplied in
    durations.wap             2 weeks, 3ms
    …                         one file per topic as the forms arrive (inline html, other languages' spellings …)

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

Owners go by stage (user 2026-11-18, notes/wap_review.md "The workers"). Anyone may edit any wap file on a branch;
the owner of the file's stage reviews the change before the handoff, and owns the stage's contract: what its output
language is (notes/backbone.md "Stage languages"). Breadth stops until milestone 2: wap gets only what its own
source uses, and every landing names the functions of wap it turned green (scripts/wap-census.warp). Tests
(test/wap/) and notes/backbone.md "Debts" are append-only, open to all. A bug of the old compiler that blocks wap
goes to warp-host-fixer as a repro; work around it or go on meanwhile. A new file belongs to the stage of its folder.

| stage | owner | files (under wap/) |
|---|---|---|
| lead: the queue, the contracts | warp-lead | compiler.wap, core/pipeline.wap, core/stage.wap, core/errors.wap, kernel/forms.wap |
| self-build: wap compiled by wap, file by file | warp-wap-operators | test/wap/test_self.sh, scripts/wap-census.warp; edits any file to write its types and `use` lines |
| front: read, operators, rules, lowering | warp-wap-reader | core/parse/, core/rules/, core/format.wap, core/language.wap, preload/ |
| types: declare, bind, type | warp-wap-binding | core/analyze/, core/paths.wap |
| back: emitter, runtime | warp-wap-emitter | core/emit/ |
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

Milestone 1 in progress. Built: compiler.wap, kernel/forms.wap (Call, Block, as patterns), core/parse/reader.wap (warp's
`parse`), core/rules/lowering.wap (by hand: phrases and statements), core/emit/emitter.wap, core/emit/values.wap (the literals' value layout) and core/emit/wasm.wap (a general
module builder: GC struct and array types, locals, globals, passive data, instruction encoding); the emitter takes Define, Set, Call and names by their binding facts. `print "Hello"` and several such lines run through wap's own stages; `print` is the
preload's (wap/preload/prelude.wap), a built-in whose body is the primitive print_text.

Test: `bash test/wap/test_hello.sh` (WARP names the warp to run with).

Next, in order:
1. kernel/primitives.wap, the list of primitives, once there are several
2. core/stage.wap with positions, and core/errors.wap: a refusal names the user's line
3. Define and Name, then If and While, each with an outcome test
4. core/rules/ taking over lowering.wap; the first rule files in preload/sugar/ and extensions/

## Road to self-build

Measured 2026-11-18 (main 05ae82852f) by scripts/wap-census.warp, which runs the compiler's own stages
(wap/core/pipeline.wap, the shape rules included) on each of wap's 21 program files (762 top-level statements; the 8
rule files, kernel/forms.wap and preload/sugar/, pass every stage). Lowering and binding run statement by statement, each
file bound with the names its `use`d modules define; a statement stops at its first refusal, so each row counts the
statements a missing piece blocks first. Typing and emitting run on the whole file with its modules, as the compiler
does: 705 statements lower, 359 bind, no program file types yet.

| blocks first | stage that should handle it | what is missing |
|---|---|---|
| 15 of 21 files' typing | preload functions | the runtime's word `line_of`: errors.wap's `refused`, used by every stage, so it stops them all |
| ~90 statements | kernel + binding | a match arm of a constructor: `List(items, bracket, separator)` (39), `Symbol(name)` (16), `Key(…)` (16), Define, Call, Block, Arm, Get |
| ~75 | kernel | Record: enums (`Fixity`, `LexemeKind`, `Separator`, `Bracket`) and their cases `a, b`, class fields `name: text`, constructors `Code`, `Block`, `Text`, `Key` |
| ~40 | preload functions | operators `??` (19), `is` (17), `..`, `===` |
| ~55 | preload + closures | list words taking a function: `map` (14), `filter` (10), `contains` (10), `has`, `all`, `add`, `flat_map`, `find`, `ends_with` |
| ~30 | preload functions | the runtime's words: `symbol` (10), `text`, `line_of` (6), `attributes_of`, `positioned`, `is_digit`, `is_alpha`, `args` |
| 4 files' typing | lowering | the range `0 upto width` (wasm.wap) |

Rerun after a landing: `for f in $(git ls-files "wap/*.wap"); do echo "== $f"; warp scripts/wap-census.warp $f; done`.

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
