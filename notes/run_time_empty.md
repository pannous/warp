# `[]` at run time (card run-time-empty)

`[]` used to become plain ø the moment it was emitted: `print []` showed `ø`, `type([])` was empty, and a `[]`
returned from a function no longer took `+= "a"` ("ø plus a text").

Now `[]` is ø marked with the square bracket above the kind bits, `type_kinds::EMPTY_LIST_KIND`
(`(SQUARE_BRACKET_INFO << KIND_BITS) | Kind::Empty`), built by `new_empty_list`, as a bool is an Int marked BOOL_INFO.

- Every read that masks the kind (KIND_MASK, `emit_kind_test`) takes it as empty: count 0, concatenation,
  emptiness, falsiness. Tests of the whole kind against `Kind::Empty` must mask; equality keeps the kinds apart,
  count and `emit_empty_as_null` use `emit_kind_test`.
- Where it differs from ø: printing (`[]`, also nested: `[[] []]`), `type` (list), reading back (an empty Square
  List), and node_add (`[] + "a"` appends, plain `ø + "a"` still fails, decision empty-plus-text).
- `[] == ø` is no (decision empty-is-not-missing, notes/emptiness.md).

Not yet: the empty results of list operations (`xs[0..<0]`, a filter that keeps nothing, `pop` of the last item)
are still plain ø; the self-hosted runtime (core/runtime) has its own empty list and still shows `ø`.
