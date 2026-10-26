# Node types and refinements (card beautiful-types, decision node-refinements)

The compiler's signatures say which node they take, not bare `Node`/`any`.

## Step 1: the cases are types
`left: Key`, `items: List?`, `m: Meta` hold a node of that case (branch beautiful-types). Symbol, Text, Number, Char
and Error were type words already. Both compilers hold the node as is; the case is not checked yet
(Rust: node_cases.rs NODE_CASE_TYPES, warp: node_cases.warp node_case_types). A program's own type of that name wins.

## Step 2: named shapes are named patterns
A shape several functions take gets a name with the user's keyword of named patterns (card patterns-values):

```warp
pattern Definition = Key(head, Op.Define, body)
body_of(d: Definition): Node := value_of(d)      // a type: any node, unchecked yet
if n is Definition { … }                         // a test
match n { Definition: head; _: ø }               // an arm binding head and body
```

The compiler's shapes live in core/node_patterns.warp: Assignment, Pair, Definition, Member, Lambda, Call,
Phrase, Block. A used file brings its patterns like its functions. Guarded shapes (`Operation`: any operator, `Call`
with a name first) wait for pattern guards. Missing shapes are asked of warp-types-back; no branch declares its own.

Where: warp node_matches.warp (no new pass), Rust named_patterns.rs and modules/declarations.rs (bootstrap only).
