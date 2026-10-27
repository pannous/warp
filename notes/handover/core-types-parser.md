# Handover: core types (parser, lexer, rewrites, the remaining core files)

Session warp-core-types-parser, 2026-10-26/27. Task from the user: "beautify the compiler by adding more types,
classes, enums". No behaviour change except the bugs fixed on the way. Every card below is Done.

## Done
- core-types-parser (6cd34f8d12): lexer.warp lost its dead Rust-twin words; the parser passes brackets and separators
  as the `Bracket` and `Separator` enums (`listed(Bracket, Separator, nodes)`, `call_of`, `spaced`, precedence as a
  match); group state is one `GroupContext`, what ends `items_until` the enum `ItemsEnd`; declared operators are
  `DeclaredOperator { glyph, fixity: Fixity, level }`.
- parser-lexer (6e02fceaf8): parser, lexer and parse helpers typed; clauses.warp reads a `Clause` class.
- once-seed (fc4a736529): the bracket-glyph workaround went once seed e59091d8 carried the enum-parameter fix.
- rewrites-typed (8753815289, b082cae93d, 636fd3c681): rewrites.warp typed; a rule's place is `enum Place`
  (PairKey/PairValue, not Key/Value: those would collide with Node's Key case); `TypedHole`, `Membership` classes.
- core-typed-rest (ce1bd26b34): node_matches, plain_enums, node_cases, source_checks, arity_overloads,
  index_positions, property_accessors, definitions at 0 anys; property_accessors reads a `Property` into a
  `PropertyTable` class.
- Bugs: map-class (0f4cc89d98, `{}` field default read as ø by with_changed_members' node_of round trip; stage_node
  keeps it), prefix-operator-dispatch and function-literal (1c39cb7e88: a declared glyph is no free symbol;
  parameter_name of a literal names nothing).
- Gate probes added: brackets, declared_operators, properties, class_default_map.

## Left
- rewrites-returns: `split_at_where` back to a `WhereCut?` class once a seed carries 97e1e605ef (recursive-class-return);
  with the old seed `warp rebuild pure` fails "the field conditions of a ?".
- pattern-clause-refusal: list and data patterns in clauses (`len([h, t...])`, `data($a * 1)`), the 2 of 7 left in
  test/functions/test_pattern_clauses.
- The honest anys: walk (20) and serializer (16) take views, a list or an atom; lexer (10) takes a source that is a
  text or its code points. Without union types they stay `any`.

## Lessons
- Smoke + self test passed a change that broke `[]` and `{}` (47 failures in test/lists): add a gate probe for the
  outcome a refactor touches, and compare one test folder against a baseline build when the change is wide.
- A compiler change only reaches compiler code through the seed: a language fix on main (enum-parameter-conversion,
  recursive-class-return) helps compiler code only after scripts/refresh-seed.sh.
- Use scripts/rebuild.sh, never `warp rebuild pure` in a worktree: once it wrote main's installed warp.wasm
  (card rebuild-pure).
- Never chain `| tail -1 && git push`: the pipe hides the failing exit code (a push once went out on a red gate).
- Class field defaults: give a mutable map explicitly where a local aliases it, until map-class is long settled.
- An enum case named Key or Value can collide with Node's cases in bare patterns: name them distinctly.
