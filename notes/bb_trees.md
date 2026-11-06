# BB trees for warp (card bb-trees, evaluation)

BB[α] trees (bounded balance, weight-balanced): binary search trees whose every node keeps its subtree's size and
rebalances by rotation when one side outweighs the other by a fixed factor. Haskell's Data.Map and Data.Set, MIT
Scheme's wt-tree and many functional libraries use them.

## What they give warp that it lacks
- **Sorted maps and sets.** warp's maps are hashed and keep insertion order; sorted order means `m.keys.sort()` each
  time (O(n log n)). A BB tree iterates in key order, answers `min`, `max`, the first key ≥ k and ranges
  (`m from "b" to "f"`) in O(log n).
- **Rank and select in O(log n)** from the stored sizes: the 5th smallest key, the position of a key, a median. A
  hashed map has none of these; a sorted list has them, but each insertion costs O(n).
- **Persistent values.** An insertion copies only the O(log n) path from the root and shares the rest, so a "changed
  copy" (`m2 = m with k: v`) costs O(log n) instead of a full map copy. This fits warp's value semantics for data
  (a `same as xs` result without the changed mark is a copy, notes/rust_hacks.md row 3).
- **Set algebra by split and join**: union, intersection and difference in O(m log(n/m + 1)), better than one lookup
  per item for sets of very different sizes (∪ ∩ in lib/list.warp walk lists today).

## What they cost
- Every lookup is O(log n) against a hash map's O(1): no default map type, only for the scenarios above.
- A WASM GC struct per node (key, value, size, left, right): about 5 fields of overhead per entry, against the
  hashed $NodeMap's arrays.

## Shape in warp, no core
A library module (lib/sorted.warp), plain warp: `class SortedMap { root: Tree? }` over `class Tree { key; value;
size: int; left: Tree?; right: Tree? }`, with `get`, `set` (a changed copy), `remove`, `min`, `max`, `at(rank)`,
`rank(key)`, `from … to …`, iteration in order, and the operators ∪ ∩ ⊆ declared for SortedSet as lib/list.warp
declares them. Keys compare by `<`, so any ordered value works: numbers, texts, a class with its own `<`.

## Recommendation
Worth building as a library when a program asks for sorted keys or ranges; a sample (a leaderboard: rank of a score,
the top 10, scores between 100 and 200) shows where it beats `keys.sort()`. Not as the default map type. A
follow-up card: sorted-map-library.
