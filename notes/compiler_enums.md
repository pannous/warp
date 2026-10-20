# Enums instead of magic words in lib/compiler (card compiler-enums)

User (2026-10-20): "the warp compiler code has many magic strings, can we replace them all with enums?"
User decision relayed the same day: the compiler is independent of Rust, no conversion to and fro: the stages pass
warp values to each other directly (warp-standalone makes the pipeline one warp program). Only the wasm bytes and
user-facing messages leave the compiler. So the enums are designed end to end inside warp: no case ↔ text mapping
for a JSON boundary, no Rust spellings to match.

Survey (2026-10-20, bare word literals outside comments): emitter 750, words 410, analyzer 353, parser 278,
node_matches 124, runtime 57, serializer 52, the other 29 files 0-23 each. Top words: "int" 209, "node" 112,
"list" 109, "empty" 108, "text" 100, "float" 96, "bool" 35, "data" 31, "codepoint" 31, "char" 31, "symbol" 29,
"struct_get" 23, "struct_new" 15.

## Classes of literals and what each becomes

| class | examples | becomes |
|---|---|---|
| value kinds (analyzer, signatures, words call_kinds/type_kinds) | "int" "float" "text" "codepoint" "empty" "data" "rational" "" (unknown) | `enum Kind`, unknown is ø |
| held kinds (emitter: how a value sits in wasm) | "int" "float" "bool" "char" "text" "list" "node" "i32" "f32" | `enum Held` |
| node view tags (walk, serializer, plain_enums, emitter) | ["symbol", x], ["list", "{", ""], "key", "attribute", "template", "literal" | Node constructors and match arms (`Symbol(x)`, `List(items, Bracket.Curly, Separator.None)`), decision compiler-node-enum |
| operators, brackets, separators in comparisons | `op_of(n) == "then"`, `bracket_of(n) == "{"`, ["then", "then"] | `Op.Then`, `Bracket.Curly`, `Separator.Comma` (op_of/bracket_of/separator_of give the case, decision compiler-node-enum); node_matches' `spellings` table goes |
| wasm instructions | gc_code("struct_new", …), int_ops["+"] | the existing opcode tables read by field: `gc_code(gc_ops.struct_new, …)` like `op.i64_const`; a table keyed by Op where the key is an operator (`int_ops[Op.Add]`) |
| internal tags of the stages | KEYWORD_FORMS values "condition" "loop" "endless", scope's "function" "variable" "parameter" "field", signatures' "same" "element", tries' "finished" | one enum per table, declared beside its only user (`enum Form {…}` in parser.warp, `enum Binding {…}` in scope.warp) |
| analysis records | kinds["functions"], ["value"], ["variables"], ["elements"], {params, result} | classes (`class Analysis {functions; variables; value; elements}`, `class Signature {params: list; result: Kind}`), read by field |
| source vocabulary: words the parser matches in the user's text | KEYWORD_FORMS keys, "public" "private" (sugar), "import" "include" (uses), "push" "append" (list_methods), selector_words | stays text, once, in a word table (words.warp style): it is the language's spelling, data the parser compares source words with |
| names of runtime and library words | "collection_contains" "is_type" "print" symbol("return") | stay text: identifiers the compiler writes into the program it builds (a name, like any symbol's) |
| messages, test texts | refuse("a variable declared " + t), "abc" | stay text |

The kind words a user writes (`x: int`, `list of int`) are source vocabulary: words.warp type_kinds maps the written
word to its Kind (`{int: Kind.Int, integer: Kind.Int, …}`), the only place a kind's spelling and its case meet. The
analyzer's printed result (`f(int) -> float`, what test_self_analyzer reads) is a message: one `spelled(k: Kind)` in
the shared file writes a case lowercase (`Kind.Codepoint` → codepoint), no table.

## Kind and Node: the kinds are warp's types, no Kind enum (decided 2026-10-20)

User's question: "enum Kind: how does that relate to Node?" A Kind enum would re-list Node's shapes (Empty, Text,
Symbol, Key, List, Error, …) beside type-only cases (Int, Float, Rational, I32, F32, Function), two enums for one
idea. Neither shape works. "Kind is Node's cases plus refinements" makes Int/Float/Rational refinements of Number
and needs enum composition the language does not have. "Kind holds only what Node doesn't say" splits text + text →
text across two enums. The analyzer's kind is what the compiler knows about a value before it runs: a type.
Warp already has those values, the built-in types `int`, `float`, `rational`, `number`, `codepoint`, `text`,
`symbol`, `list`, `error`, `function`, `i32`, `f32`, `any`, and a Node case names the type of its values (`Text(v)`
is a text). So the kinds are written as the types themselves:

```warp
widen(a: type, b: type): type := if a == float or b == float then float else int
primitive_types = [int, float, codepoint, i32, f32]
```

Mapping of today's words: "int" int, "float" float, "rational" rational, "codepoint"/"char" codepoint, "text" text,
"symbol" symbol, "list" list, "error" error, "function" function, "i32"/"f32" i32/f32, "pointer" pointer, "data" (a
number of run-time kind) number, "empty"/"node" (any node) any, "bool" bool, "" (unknown) ø. "list of int" is
`list of int` once types carry their element type, until then `[list, int]`. The emitter's held kinds are the same
types (one vocabulary: Held goes too). The printed analysis (`f(int) -> float`) is "${t}". No new enum, nothing
duplicated, and no case name collides with a type (R1 is fixed anyway: rust-lane, enum-case-names).
A type value prints `class int` (decision type-text); the analysis text spells a kind with `t.name` → int.

Needs, probes/compiler_enums/f_type_values*.warp, f_type_match.warp (2026-10-20):
- T1 pure lane: a built-in type as a value, `k = int`: "the unknown name int" (card pure-lane-reads).
- T2 Rust lane (card rust-lane-int): a match on a value declared `type` with arms naming types (`match t { int: …;
  float: … }`) takes `_`, the arms test t's own type; `t: type = int` is "unknown type type". Membership
  (`[int, text] contains k`) works; the first probe's "no" was right.

## Declarations (superseded for Kind and Held by the section above)

lib/compiler/kinds.warp, used by every stage that names a kind (words, signatures, analyzer, emitter, serializer):

```warp
# the kind of a value as the analyzer infers it
enum Kind { Empty, Int, Float, Text, Codepoint, Symbol, Key, Block, List, Data, Meta, Error, Typedef, Pointer, I32,
	F32, Function, Uncertain, Instant, Rational, Node }
# how the emitter holds a value of a kind in wasm: unboxed numbers, or a reference
enum Held { Int, Float, Bool, Char, Text, List, Node, I32, F32 }
held(k: Kind): Held := match k { Kind.Int: Held.Int; Kind.Float: Held.Float; … ; _: Held.Node }
spelled(k: Kind): text := (k as text).lower()
```

Op, Bracket and Separator stay where they are (built in, declared in lib/compiler/node.warp). Enums used by one
stage only are declared in that stage's file.

Writing style: cases are written qualified where a case shares a type's name (`Kind.Int`, `Held.Text`), bare where
not (`StructGet`); sets of cases are lists (`primitive_kinds = [Kind.Int, Kind.Float, …]`, `kind in
primitive_kinds`), tables keyed by a case are maps (`{Kind.Int: …}`); a match on a Kind names every case or ends with
`_`.

## What the lanes must handle first (probes/compiler_enums/f_*.warp, 2026-10-20, main's build)

Rust lane (compiles the stage programs today):
- R1 a case sharing a built-in type's name is read as the type: `match k { Int: …; Kind.Int: … }` arms test the type
  int (wrong arm), `k == Float` hints "prefer is" (f_collide*.warp). Blocks Kind and Held.
- R2 `"Float" as Kind` errs "Float is no Kind case" (f_text_as.warp). Not needed any more (no text boundary).

Pure lane (WARP_SELF_HOSTED=pure-only):
- P1 a variable or result declared with an enum type, `w: Wasm = RefCast`, `g(): Wasm := …`: "a variable declared
  Wasm" (f_typed_var, f_returned).
- P2 an `other:` arm (f_match_other): "the unknown name match"; `_:` works.
- P3 `as index`, `1 as Wasm` (f_as_index, f_int_as); not needed by this plan.
- P4 `k == Float` with Float a case: "a test of the type Float" (f_collide).
Works in both: parameters typed with an enum, `==`, `contains`, map keys, `as text`, block and `;` matches with `_`.

## Order of work

By enum, not by file, since a kind crosses files: each step converts every user of one class, smallest first, each
its own EXPRESS branch.
1. wasm instruction tables read by field (emitter only, mechanical, no language feature needed) — announced first,
   emitter.warp is contended.
2. internal stage tags (scope, signatures, tries, for_loops, parser's Form) — small files first.
3. node view tags → Node constructors (walk, serializer, plain_enums, node_matches), and Op/Bracket/Separator
   comparisons once op_of gives the case.
4. kinds as types (words, signatures, analyzer) after T1/T2.
5. held kinds as types (emitter) after T1/T2.
analyzer.warp, parser.warp and emitter.warp are announced to warp-supervisor before each step touches them.
