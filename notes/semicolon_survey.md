# Semicolon rule survey (2026-09-29)

Rule surveyed (wiki/list.md): `;` and newline bind loosest. Blocks are only the top level and `{…}`; a block yields its
LAST `;`/newline item (`1;2;3` → 3, `'hello';(1 2 3 4);10` → 10). `(…)`, `[…]` and data mode keep all items
(`(1;2;3)`, `[1;2;3]` → `(1 2 3)`; `(1 2, 3 4; 5 6, 7 8)` is a tensor). Space and comma always form value lists.
Block values are checked with `is!("1;2;3", 3)`.

Method: the rule was applied experimentally (`is_semicolon_sequence` in src/analyzer.rs, `;` and newline,
`Bracket::None | Bracket::Curly`) in a separate git worktree, then the full `cargo test --no-fail-fast` suite was run.
Nothing was changed in the main checkout; the current heuristic from de18c3bb is still in place.

## Existing tests that conflict (not to be modified without consent)

| Location | Input | Expected now | Value under the rule |
|---|---|---|---|
| tests/lists/test_lists.rs:194 (`test_root_lists`) | `{1;2;3}` | list `[1,2,3]` | 3 |
| tests/lists/test_lists.rs:205 (`test_root_list_strings`) | `a;b;c` at top level | strings `a b c` | `c` |
| tests/lists/test_lists.rs:213 (`test_root_list_strings`) | `{a;b;c}` | strings `a b c` | `c` |
| tests/lists/test_lists.rs:189 (commented out) | `1;2;3` expecting `ints(1,2,3,0)` | list | 3; stays commented |

Both tests stop at their first failing assert, so the three lines above were identified by reading; only the first is
seen failing per test.

## Checked and not conflicting

- `(1;2;3)`, `(a;b;c)` (tests/lists/test_lists.rs:188, 204), `[1;2;3]`, `[a;b;c]` (:195, :211): stay lists.
- `(1, 2; 3, 4)[1][0]` and `[1,0]` (tests/control/test_blocks.rs:86, :91): stay a matrix. These fail if `(…)` is treated as a block.
- tests/wasm/test_wasm.rs:1347 `'hello';(1 2 3 4);10` → 10: already passes, agrees.
- tests/parser/test_semicolon_square.rs: pins `[1;2;3]` and `[1\n2\n3]` equal to `[1 2 3]`; passes before and after.
- Wiki tensor `t=(1 2, 3 4; 5 6, 7 8)` (wiki/list.md:89): in no test.

## Reference suite ~/wasp/test (by reading, not re-run after the last rule correction)

- Agree: tests.cpp:3184 `(1;2;3)`, :3199 `(a;b;c)`, :3191 `[1;2;3]`, :3206 `[a;b;c]`, :632/:633 `(1, 2; 3, 4)[1][0]` / `[1,0]`,
  test_wasm.cpp:256, :1008, :1404 `'hello';(1 2 3 4);10` → 10.
- Note: tests.cpp:3184 and :3199 use `assert_is` (data mode); under the rule `(…)` keeps all items either way.

## Open question: newline-separated data objects in braces

With newline as a block separator and no item-content check, `{a:1\nb:2}` evaluates to `b:2` (the first key is lost);
`1\n2\n3` → 3, `a\nb\nc` → `c`, `(1\n2)` → `(1 2)`. No existing test evaluates a newline-separated braced data object, so
nothing fails, but the wasp data format (`Person {\n name: "Alice"\n age: 30 }`) is exactly that shape.
Decision needed: is `{…}` whose items are `key:value` pairs data (all items kept) or a block (last item)? Without an
item-content check the rule says block; that would silently drop keys when such an object goes through eval.
Parsing (`parse`, `parse_data`, `warp data`) is unaffected; only eval/emit is.

## Implementation plan once approved

1. Replace `is_semicolon_sequence` (src/analyzer.rs, used in src/wasm_emitter/list_emitter.rs and the analyzer's
   `is_data_node`/`infer_type`) by the bracket rule: `Semicolon | Newline` and `Bracket::None | Bracket::Curly`.
2. Add tests: `is!("1;2;3", 3)`, `is!("'hello';(1 2 3 4);10", 10)`.
3. The three assertions in the table conflict with the new rule; they need the owner's decision.
