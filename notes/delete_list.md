# Delete on lists

`xs.delete(value)` is the familiar JavaScript/collection spelling of Warp's `xs.remove(value)`. It removes the first equal item from a list variable, mutates the same runtime list as `remove`, and emits the standard alias note naming `remove` as canonical.

The implementation recognizes both words in the existing removal lowering. This keeps list mutation tracking, alias visibility, map removal behavior, and the `removed_value`/`map_without` runtime path shared rather than adding a second delete implementation.

Regression probe: `probes/delete_list.warp`, covered by `delete_removes_the_first_matching_list_item`.

`delete xs[i]`, `delete xs#i`, `delete m["k"]` and `delete m.k` (Python's `del`) become `xs.pop(k)` in
mutation::lower_deletes, right after stored_values (which takes `delete storage[k]` first): `x[k]` and `x.k` keep k
(a 0-based position or a key), `x#i` counts from 1, so it pops i - 1. `delete` loads the prelude's popped_item and
popped_rest as `pop` does (library_words LOWERED_TO_PRELUDE). Test: `delete_statement_removes_an_entry`.
