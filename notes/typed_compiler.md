# The typed compiler (card typed-compiler: types-front, types-back)

Every parameter, field and result in lib/compiler is typed (user 2026-10-20), with small domain types instead of any
or plain text.

## Shared types: lib/compiler/types.warp
- `Node` (node.warp's enum) for every node of the program; `any` stays only where a value really is any value.
- `Name` (a word: variable, function, field, type), `KindName` (the analyzer's kind names: "int", "data", "empty", …),
  `TypeWord` (`int`, `list of int`, `map of text`): type aliases of text.
- Plain words where they are exact: `int`, `bool`, `text`, `list`, `texts`/`ints` (a list of them), `map`,
  `function`, `T?` for a value that may be ø.
- A domain type with behaviour or fields is a small class in its stage: analyzer.warp's `Scope`, `Usages`, `Flow`
  (with `enum FlowWay`); front-only classes stay in lexer.warp/parser.warp.

## What the lanes needed (types-back)
- Rust: a used file's `type X = T` reaches the program's annotations (modules.rs insert_module_classes brings aliases
  with the classes), a class field typed `map` holds a node (type_kinds.rs field_storage), `f(): Name? := …` of a
  type the parser does not see declared is an optional result, not a ternary (lookahead.rs declaration_follows).
- emitter.warp: `type X = T` is recorded (collect_types) and compiles to nothing; parameters, declarations, results
  and class fields read aliases (`aliased_type`), plural type words hold lists, `map`/`Node`/`function` hold nodes
  (`annotated_kind`, `field_kind`); a character given to a text variable is that text.
- Runtime words take and give `Node` (Rust lowers `Node` to `any`, so their code is unchanged).

## Open (cards)
- optional-text-slice: `t[1..]` of a `text?` picks the list overload in the Rust lane, so analyzer.warp's
  `joined_list_type(known)` and `typed_element_kind(type_name)` stay `any`.
- alias-list-type: `type Code = ints` / `list of int` for byte lists (emitter.warp's Code).
- count-of-field: `#s.names` of an instance's list field.
- emitter.warp itself and host_words.warp wait for the emitter split (card emitter-split); Function/Signature are
  warp-reviewer's (card function-signature).
