# Parsed and built nodes in warp code

Cards parsed-kinds, built-key-print, bracketed-empty, parse-typedef and parse-const-fold. These are what a compiler pass written in warp
needs (notes/self_hosting_readiness.md). Tests: tests/node/test_parsed_node_walks.rs.

- Code is marked as code only in its top node (as_code, type_kinds::CODE_MARK). The joins (list_join, list_text)
  test atoms by their whole kind field, so a marked atom or type declaration joins as its unmarked form
  (text_builtins::emit_without_code_mark). Only MARKED_CODE_FORMS are tested: a time keeps its own info in those bits.
- node_is_map: a curly list marked as code is no map, nor is one that doesn't begin with a pair (card
  replace-quoted-block). Code-typed subjects skip the "a map is no list" exclusion, so `parse("{a: b}") is list`.
  A `{a: b}` deeper in the tree is still a map at run time (card curly-group). `n.bracket != ø` is the group test
  that holds at every depth.
- `n.bracket` and `n.separator` of a node that is no group are ø, not "no field".
- `bracketed([], b)` and `separated([], s)` are ø, the empty list, as `()` parses.
- `key(l, op, r)` registers its op as a key-operator need, or every op when only the run time knows it, so list_text
  writes `y+z` and not `y:z`.
- A parameter that gets values of several Node kinds (a symbol and a key) takes a Node (widen_parameters).
- `data f(x) := …` defines nothing: extract_user_functions_in skips quoted data.
- parse() of a constant text that declares a type is not folded. The run time builds it (TaskValue::Type,
  new_type), because a folded `class P {…}` would read as the program's own type P. A TypeDef prints as
  Node::serialize writes it: `type P {x:type int ø}`.
- `key_of(n)`, `value_of(n)`, `op_of(n)`, `name_of(n)`, `bracket_of(n)` and `separator_of(n)` read a node part even
  where a field of that name wins over `.key` and the like (card node-part-clash: the code `key = 3`). They give ø for a
  part the node lacks. A program's own function of that name still wins.
