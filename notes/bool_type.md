# bool: a shallow type of its own (cards bool-type, zero-false)

Decision (notes/decisions.md): true/false act as 1/0 in arithmetic and comparisons, `type(true)` is bool, they print
as yes/no (src/node/serialization.rs YES/NO; true/false stay accepted as input). `0 == false` is true, `0 === false`
is false.

## Representation
- A bool is an Int node marked in the high kind bits: `BOOL_KIND = (BOOL_INFO << 8) | Kind::Int` (src/type_kinds.rs).
  Runtime code masking the kind (KIND_MASK) sees an Int, so arithmetic keeps working.
- Runtime `new_bool(i64)` and `as_bool(node)` (src/wasm_emitter/constructors.rs); `new_bool` is exported (tasks).
- Readers: wasm_reader.rs and web.rs node_from_tree turn a bool-marked Int into Node::True/False.
- Run-time type tests of a value of unknown static type (`value:any`): node_kind_in gives a bool its own mask bit
  (BOOL_MASK_BIT), so it is no int; run-time values_equal takes a bool as a number (`false == 0`).
- Tasks: a bool crosses as 1/0; where a bool function's task is awaited the result is compared `!= 0`, a bool again
  (lowering/declarations.rs checked_await, card bool-crossing).

## Which values are bools
src/analyzer/booleans.rs `is_boolean(node, scope)`: literals, comparisons (incl. `===`), prefix `not`, and/or of bools,
bool variables (Local.type_node `bool`), blocks ending in one, type tests, bool-returning user functions
(`note_bool_functions`, a fixpoint). The emitter wraps such a value in `as_bool` (emit_node_instructions).

## `===` / `!==`
Op::Identical / Op::NotIdentical (appended to OP_CODES). The emitter rewrites `a === b` (equality.rs
identity_as_equality): `a == b` when both sides are bools or neither is, else the constant false (`!==` true).
Only boolness counts so far: `1 === 1.0` is true.

## Rust side
`Node::True == 1` and `Node::False == 0` (PartialEq<i64>), so `is!(…, 1)` tests keep passing; serialized output
changed to true/false (tests upgraded in their own commit).

## Open
- Literal `true + 1` keeps the analyzer error "arithmetic on a boolean" (older decision, notes/footguns.md).
- Cards true-lowers (`[true, 2]` lowers to ø) and bool-crossing (a task's bool result arrives as 1).
