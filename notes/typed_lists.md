# Typed lists and the list dispatch seam

Code: `src/wasm_emitter/list_dispatch.rs`. Tests: `tests/lists/test_typed_lists.rs`.

## What is typed

A list *variable* (a local of kind List, in main or in a function body) whose every assignment is

- an int list literal `[1 2 3]`, `[a, a+1, f(x)]` (each item provably an exact Int, see `is_int_item`),
- a zero fill `x = int[100]`, `x : 100 int` (`zero_fill(count, 0)`),
- the empty list `[]` / `ø`,
- an append to itself `xs = xs + [v]`, what `xs.add(v)` / `push` lowers to, or
- another such variable (`ys = xs`, and the `x·items = xs` the `for` lowering writes)

is held as an `$IntList`: `(struct (field $length (mut i32)) (field $items (mut (ref $IntArray))))` with
`$IntArray = (array (mut i64))`. The i64 is the same Int value a local holds (fixnum or big-int/ratio handle, big_int.rs).

Excluded, so they stay cons cells: parameters, globals a function assigns, variables a nested function captures from an
enclosing function, names assigned inside a `try`,
names updated by an operator (`xs += …`, `xs++`), names indexed by a key (`xs["a"] = …`), and every list with a
non-int element (`[1, "a"]`, `[ø]`, `[1.5f]`). The analysis is a greatest fixpoint over the assignment sources.

## Why results stay identical

Only a handful of operations use the array: count (`#xs`, `count xs`, `xs.size`), element (`xs#i`, `xs[i]`, as a number or
a Node), element assignment (`xs#i = v`, `xs#i += v`), store, append. Every other use of the variable reads it as a Node
through `int_list_as_node`, which builds exactly the square cons list the literal would have built (ø when empty). So
printing, returning, passing to a function, `sort`, comparisons… are unchanged, they just pay one O(n) conversion.
Errors are the same runtime functions (`index_out_of_range`, `index_must_be_an_integer`).

Lists are shared (P200b, card shared-lists): `ys = xs` shares the typed array, so an item set or added through either
shows in both; `xs = xs + [v]` copies first while the list is aliased (TypedList.aliased). A typed list some holder keeps
as Nodes (an argument to a function that changes it, an item or field, an untyped alias) stays a Node list
(wasm_emitter/list_sharing.rs held_elsewhere, find_typed_lists), as one conversion would part the two.

`sum`, `map`, `each`, element-wise `xs * 2` are lowered to loops before emission (library_words.rs, lambdas.rs,
analyzer::element_wise); those loops reach the dispatch layer as count / element / append, so they get the array for free.
Measured (debug build, `xs=[]; for i in 1..n {xs.add(i)}; for x in xs {s+=x}`): n=1000 0.45 s vs 4.5 s cons cells,
n=4000 0.43 s vs 72 s (cons-cell append and `#items` per iteration are O(n), so the old loop was O(n²)).

## The dispatch seam

```
ListOp  { Count, Element, SetElement, Store, AsNode }        what the emitter asks for
Backend { NodeCells, TypedArray(ElementType), Host }          who implements it
list_backend(op, target) -> Backend                           the one place that decides
HOST_BACKEND_MIN_LENGTH: Option<u32> = None                   run-time switch length for a host backend
```

Each `emit_list_*` method matches on `list_backend` and emits the generic Node code or the typed code. The generic
Node implementation is never removed: it is the fallback for any list the analysis cannot prove.

## How a host-native or GPU backend plugs in (no wasm SIMD: user decision 2026-10-03)

1. Keep the whole-list operation visible until emission. Today `sum`, `map`, element-wise arithmetic and `sort` are
   lowered to loops early. For a backend to take over a whole operation, lower `sum xs` to a pseudo-call such as
   `list_sum(xs)` (like `zero_fill` and `ran_without_error`), and `map` with a known body to `list_map(xs, body)`, then add
   `ListOp::{Sum, Map(body), ElementWise(op), Sort}`. The typed backend emits the loop over `$IntArray` (what the lowered
   loop does today); the Node backend keeps today's lowering.
2. `list_backend` returns `Backend::Host` for those ops when the element type is supported. The emitted code tests the
   length at run time: `if length >= HOST_BACKEND_MIN_LENGTH { call host.list_sum(list) } else { typed loop }`, so a short
   list never pays the transfer and the program cannot tell which ran.
3. The host import (src/host.rs, wasmtime linker) receives the `$IntList` as a GC reference. wasmtime 49 exposes GC
   arrays to host functions (`Rooted<ArrayRef>`, element reads/writes on the store), so the host copies the items into a
   native buffer: Rust/rayon for host-native, wgpu/Metal for GPU (`map` bodies compiled to a compute shader for
   arithmetic bodies), and writes the result into a new `$IntArray`.
4. Threshold: the copy is O(n) both ways, so only compute-heavy ops pay off; start with a measured constant per op
   (expect 10⁵–10⁶ elements) and keep the generic path for anything the host cannot represent (big-int handles: an i64
   outside the fixnum range must route back to wasm, or the backend refuses lists that hold one).

## Open

- Float arrays (`list of float`, `$FloatArray = (array (mut f64))`): `ElementType` has room; few literal lists are float
  (decimals are exact rationals), so ints came first.
- Typed parameters and return values (a function taking `xs: ints`) would avoid the Node conversion at calls.
- `sort` of a typed list could sort the i64 array in place for fixnums (ratios/big ints need the exact comparison).

## Node lists (2026-10-04)
`ElementType::Node`: a list variable of any elements is a `$NodeList` (length, `(array (mut (ref null $Node)))`, the
kind of the list node it stands for, so tuples keep their brackets) when the program indexes or counts it (`xs#i`,
`#xs`, which every for loop does). It starts from a literal, ø, appends, or `node_list_of` (one pass over a Node
list: a call's result, a parameter, another list variable); every other use reads it as a Node (`node_list_as_node`).
An `if` statement whose branch appends to a typed list runs its branches as statements (list_emitter
emit_discarded_branches), else the branch value rebuilt the Node list on every append (filter was quadratic).
5000 items (probes/night/bench_lists.sh): for loop 41 s → 0.6 s, filter 130 s → 0.5 s, index writes overflowed the
stack (recursive list_with_at) → 0.5 s. Parameters stay Nodes: a function indexing its list parameter still walks it.
- Small helpers are inlined (src/lowering/inlining.rs: not recursive, no free variables, no locals shared with the rest of the
  program, ≤ 8 statements, one final return), so `arr = swap(arr, i, j)` updates the caller's array; `x = (t = x; …; t)`
  shares one array without copies (moved_lists). A list parameter indexed in a loop and not reassigned from a call there
  is copied into an array once (`arr·list = arr`). quicksort_partitioned of 1000: 32 s → 12.5 s; what remains is the
  Node conversion per recursive call (a $NodeList ABI for parameters and results would remove it).
- Array calling convention (src/wasm_emitter/list_abi.rs, 2026-10-04): a function whose list parameter p is used only
  through its copy `p·list = p` takes p as a `$NodeList` (the callee copies it with one array.copy, values stay values);
  a function whose every result is such an array or a call of another such function returns the `$NodeList`. Callers
  convert only where a Node is needed. quicksort_partitioned of 1000: 12.5 s → 0.9 s. Not for closure targets, tuple
  functions or text parameters. Any single copy `v = p` counts when v is a Node list (a for loop's `x·items = p`), and a
  caller's Int/Float list passes through `int_list_as_node_list` (one node per element, no cons cells): 20 sums of a
  20000-element list 5.3 s → 2.5 s (debug build).

## Globals and captured lists (card compiler-picks, 2026-10-06)
A list main builds and a function reads (`xs = []; for i in 1..n { xs.add(i) }; at(i) := xs#i`) was a capture global of
cons cells: every read walked i links and n reads ran out of fuel at n = 10^5. Now:
- A typed list of main that a main-level function captures is passed in a capture global of its array type
  (`(ref null $IntList)`): main's typed lists are worked out once before the capture globals are declared
  (allocate_closure_captures, reused by emit_node_main); the capture at the definition stores the array, a copy when main
  changes the list later (`updated`), so the function keeps the value it captured. In the function the name reads the
  global through `Slot::Global` (list_dispatch.rs `typed_list`), so index, count, sum, `for` and stores of copies are the
  typed operations. Not with tasks (a task's instance copies capture globals as Nodes).
- A `global xs` that no function assigns is typed the same way (find_typed_globals; its sources are all main's).
- A typed list captured by a nested function that takes a Node capture global is converted to its Node list there.
- Measured (probes/numeric_lists_bench.sh, `global` row): n = 10^5 out of fuel before, 0.07 s after; n = 10^6 0.08 s.
- The automatic choice of linear memory (the card's first idea) gains nothing yet: linear memory and GC arrays run at
  the same speed (notes/linear_arrays.md); it pays only once a host or GPU backend takes the block.

## Element types are checked (card list-element-types, 2026-10-08)
A declared list (`names: texts`, `names: list of text`, a field `items: texts`) holds only items of its element type,
checked like a declared scalar:
- Literal items are compile-time errors (analyzer `check_declared_types`, `list_items_mismatch`): the declaration
  `names: texts = [420]`, a reassignment `names = [420]`, an append `names.add(420)` / `names = names + [420]`, an element
  `names#1 = 420`; a field's constructor (`type_constructor::field_error`) and its append `b.items.add(420)`
  (`struct_backend::misfit_list_item`).
- Items known only at run time are checked at the store (lowering/list_element_checks.rs, first MEANING pass):
  `if not (v is text) { raise "…" }` as a statement before the store, a call's item held in a temporary `checked·N`
  first, a whole list from a call checked item by item. The store keeps its form, so a typed int list stays an array.
- `names = other + [v]` checks the items of `other` in a for-loop too, unless `other` is `names` itself or declared
  with the same element type (card list-element-runtime).
- A field's run-time append `b.items.add(f())` and a whole field store `b.items = f()` are checked the same way when
  b's class is known in the pass (`b = bag(…)`, `b: bag`; class_methods `instance_classes`, `class_fields`): "items of
  bag is declared texts". Of an instance whose class is unknown (a parameter `x.items.add(v)`) each class with a list
  field `items` guards its check, `if (x is bag) and not (v is text)`, so a map with an `items` key stays unchecked and
  a literal item is checked at run time too (card list-element-field).
- A float or number list takes ints (`is number`), as a declared float does.
- Parameters (`f(xs: texts)` called with `[420]`): warp-7c's param-types work (functions2, checks.rs
  `declared_element_type`, `elements_fit`).
- A receiver inside an element (`bags#1.n = 5`, `bags#1.items.add(v)`, `bags#1.items#1 = v`) is taken out into
  `nested·N`, changed, checked and written back (lowering/nested_index.rs); a receiver that is a call's result
  (`make().n = 5`) is the error "make() gives a copy" (card list-field-receivers).
- A character stored into a list of unknown static type (one read from a field) goes as its Node; node_with_at took
  it as an Int, so `x#1 = "z"` gave `[122]` (list_ops `is_exact_int_element`).
- Variance (TypeScript's covariant arrays), P215: lists are shared (P200b), so a view of a declared list under a wider
  element type that changes it (`widen(xs: list) := xs.add(420)` of `names: texts`, `ys: [Shape] = circles;
  ys.add(…)`) is a compile error where the alias is visible (src/analyzer/list_views.rs); a view that only reads, or of
  a fitting element type (a subclass), is fine. Not yet: the run-time half (a lax alias, an unannotated parameter,
  writes checked against the list's own declared element type), probes/variance/.

## Appends copy only while aliased (card map-typed, 2026-10-08)
`xs = xs + [v]` copies the array first when another variable may hold it (P200b). That was decided for the whole alias
group, so every lowered map and filter, `d = (out = ø; for … { out = out + [x] }; out)`, copied on each append: the
alias `d = out` exists, though only after the loop (int[20000].map 0.5 s, 80000 out of fuel). Now
(list_dispatch.rs appends_while_aliased) an append copies only if it can run after an assignment between its own
variable and another (`ys = xs`, `xs = ys`) in program order, or in a loop around one. map / filter of 10^6 ints: 7–9
ms (release); a variable given a second map result (`d = d.map(…)`) stays linear. Tests: tests/lists/test_map_typed.rs.
