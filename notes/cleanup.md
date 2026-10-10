# Cleanup (user card `cleanup`: major code cleanup and simplification)

Behaviour-preserving only: shared helpers instead of duplication, dead code and stale comments out, flatter control
flow, meaningful names, no test edits. One area per session, small themed branches through the Integrator.

## src/analyzer/, src/warp_parser/, src/node/ (card cleanup-analyzer, session warp-types)

Done (branches cleanup-dead, cleanup-arith, cleanup-names, cleanup-parser, cleanup-modes):
- dead code: Scope types, Separator::from_char, Node::key_with_op, get_meta_data, type_definition, commented-out code
- `Node::symbol_name` / `Node::is_symbol` replace analyzer's is_word and ~40 hand-written Symbol matches
- node/arithmetic.rs: `keeping_left_meta` + `scalar_operands!` macro (317 → 106 lines)
- checks.rs check_type_errors_inner: flat match arms
- parser: `Mark`/`mark()`/`rewind()` replace 26 position-tuple snapshots; `take_chars`, `code_words`,
  `at_group_end`, `for_iterable_and_body_word`, one `skip_blanks`
- json_xml.rs: `xml_attribute`, `insert_json_entry`, `json_object`
- imports.rs: `functions_of_operator` / `functions_of_call` tables instead of if chains
- inference.rs infer_list_type: one `head` symbol, unused closure arity gone
- expressions.rs continue_expr: `parse_right_operand`, `combined`
- xml.rs parse_xml_tag: `parse_attribute_value`, `parse_closing_tag`, `xml_element`
- atoms.rs parse_symbol_with_suffix: other languages' forms in `foreign_form`; `in_code()`
- variables.rs collect_variables_inner: one `visit` closure, `bind_destructured`, `bind_assigned`
- lists.rs parse_list_with_separators: `with_definition_block`, `parse_separator`
- `Node::single_or_list` for the one-item-or-list choice

Left (longest functions, candidates for splitting):
- atoms.rs parse_symbol_with_suffix (~140 lines): still a keyword dispatch
- atoms.rs parse_atom (158), parse_type_declaration_body (134)
- declaration_lowering.rs lower_declarations_among (141): ~20 arms `x if f(x).is_some() => f(x).expect("guarded")`
  compute their rewrite twice; a chain of `Option` rewrites would compute it once, but the arms interleave
  re-lowered and final rewrites, so the order needs care
- lookahead.rs peek_operator (123), inference.rs infer_type (120), literals.rs parse_string (114),
  checks.rs list_type_name (106)
- node/comparison.rs eq: left alone, it carries the user's comments

Outside this area (for whoever takes src/lowering/): private `is_word` / `symbol_name` / `is_symbol` copies in
lowering/comprehensions, for_loop, generators, lazy_ranges, stored_values, welcome_forms, inlining, soft_keywords,
tuples, and law/type_model.rs, modules.rs, time.rs can use `Node::is_symbol` / `Node::symbol_name`.
