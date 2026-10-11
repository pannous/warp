# Delete on lists

`xs.delete(value)` is the familiar JavaScript/collection spelling of Warp's `xs.remove(value)`. It removes the first equal item from a list variable, mutates the same runtime list as `remove`, and emits the standard alias note naming `remove` as canonical.

The implementation recognizes both words in the existing removal lowering. This keeps list mutation tracking, alias visibility, map removal behavior, and the `removed_value`/`map_without` runtime path shared rather than adding a second delete implementation.

Regression probe: `probes/delete_list.warp`, covered by `delete_removes_the_first_matching_list_item`.
