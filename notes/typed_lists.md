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

Excluded, so they stay cons cells: parameters, globals, variables a function captures, names assigned inside a `try`,
names updated by an operator (`xs += …`, `xs++`), names indexed by a key (`xs["a"] = …`), and every list with a
non-int element (`[1, "a"]`, `[ø]`, `[1.5f]`). The analysis is a greatest fixpoint over the assignment sources.

## Why results stay identical

Only a handful of operations use the array: count (`#xs`, `count xs`, `xs.size`), element (`xs#i`, `xs[i]`, as a number or
a Node), element assignment (`xs#i = v`, `xs#i += v`), store, append. Every other use of the variable reads it as a Node
through `int_list_as_node`, which builds exactly the square cons list the literal would have built (ø when empty). So
printing, returning, passing to a function, `sort`, comparisons… are unchanged, they just pay one O(n) conversion.
Errors are the same runtime functions (`index_out_of_range`, `index_must_be_an_integer`).

Value semantics: wasp lists are values (`node_with_at` copies). A typed list is updated in place, so `ys = xs` copies the
list (`int_list_copy`) when either side is ever updated by index or append; otherwise the two share it.

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
- Small helpers are inlined (src/inlining.rs: not recursive, no free variables, no locals shared with the rest of the
  program, ≤ 8 statements, one final return), so `arr = swap(arr, i, j)` updates the caller's array; `x = (t = x; …; t)`
  shares one array without copies (moved_lists). A list parameter indexed in a loop and not reassigned from a call there
  is copied into an array once (`arr·list = arr`). quicksort_partitioned of 1000: 32 s → 12.5 s; what remains is the
  Node conversion per recursive call (a $NodeList ABI for parameters and results would remove it).
