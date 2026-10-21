# `xs.pop()` in place (card shrink-in-place)

`xs.pop()` lowered to `pop_tmp = xs#count(xs); xs = list_become(xs, xs[0:count(xs) - 1]); pop_tmp`: a slice (an
array of all items and a new cell each) and a copy into xs per pop, so draining a list by pops was quadratic
(20000 pops 4.5 s).

Now the middle statement is `xs = list_drop_last(xs)` (src/lowering/collections/list_mutations.rs), changing xs in
place as list_become did, so every holder sees it (P200b):
- a typed array (list_dispatch.rs emit_typed_list_store): its count goes down by one, O(1); find_typed_lists takes
  the statement as Source::Empty (any element type), so a pop keeps an int list an int array;
- a Node list (list_changes.rs emit_list_drop_last): the next-to-last cell's rest is cut, no cell copied; a one-item
  list becomes ø in place. Still a walk from the head (cons cells have no back links), but no allocation; the kept
  count is lowered by one, the read and extend cursors are forgotten.

The compiler's own node stacks (scope.warp nodes_kept, walk.warp lists_in) take their top with `pop()` instead of
`xs#(#xs)` and `xs = xs[0 ..< #xs - 1]`.

Not yet: `xs = xs[0 ..< #xs - 1]` itself still copies (it makes a new list; rewriting it in place needs the
self_sums.rs escape check, card out-list); a Node list grown by `+= some_list()` stays cons cells, so its pops walk.
