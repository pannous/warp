# List + item, and the empty start (cards xs-item, start)

Decisions list-append-plus and empty-plus-text (notes/decisions.md).

- `xs + item` and `xs += item` append an item that is no list: `[1 2 3]+4` → `[1 2 3 4]`, `xs += "z"`. A list on
  the right concatenates. `4 + [1 2 3]` (number first) still asks prepend or add-to-each (filled_lists::list_plus).
- `[]` is ø at run time, but the parser keeps it marked (`Node::empty_list`, the source literal `[]`) and a local
  given it remembers that (`Local::empty_list`): `t = []; t += "a"` appends, `t = ø; t += "a"` fails with
  `ø plus a text; fix: start a text with t = "", a list with t = []` (runtime error ø_plus_a_text).
- One rule, three places:
  - statically, `analyzer::appends_item` (left a list or a `[]` local, right of a concrete non-list kind): the item
    is added as `[item]` (`one_item_list`), by the analyzer (typing) and the emitter (key_emitter
    emit_list_extend_assignment, node_arithmetic's list concat) alike;
  - at run time, `list_extend` (`xs += part` of an `any` part) wraps a part that is no list as one item
    (emit_item_as_list), and `node_add` appends an item to a list and refuses ø + text;
  - `node_add_assign` (`+=` of two run-time kinds) grows a list in place, ø by a list, and leaves the rest to node_add.
- Open: `t = []; u = t + "a"` (plus, not `+=`) of a `[]` local still meets ø at run time and fails; the hint that
  `xs += [3]` needs no brackets is not given yet.
