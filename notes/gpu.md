# GPU vectors (card gpu-vectors, user 2026-10-08)

User: list arithmetic on large number lists runs as WebGPU compute (wgpu natively, WebGPU in the playground) above a
size threshold, on the CPU below it and without an adapter; tests compare GPU and CPU results. The GPU path reuses the
gpu_compute / gpu_render plumbing (src/gpu.rs, web/playground/host-gpu.js, notes/web_framework.md "web-apis: WebGPU").

## Where the language stands (2026-10-08, measured on main)
- `xs * 2` is an error by decision (D3: repeat or multiply?), `xs + ys` concatenates. Element-wise is the dotted form
  (P166): `xs .* 2` works, `[1 2 3] .* 2` → `[2 4 6]`.
- `xs * ys` of two lists is their inner product, `dot(xs, ys)` (card g_n8GI, broadcasting.rs inner_product).
- `xs .+ ys` and `xs .* ys` (list with list) are errors today ("element-wise arithmetic needs an explicit map"): the CPU
  form comes first, the GPU only accelerates what the CPU already answers.
- `sum xs`, `xs.map(x => …)` work; `dot` does not exist (`sum(xs .* ys)` once list.*list works; `dot` an alias later).
- Number lists of known type are wasm GC arrays ($IntList / $FloatList, notes/typed_lists.md, list_dispatch.rs with
  the `Backend::Host` seam and `HOST_BACKEND_MIN_LENGTH`), or linear memory (`linear xs = float[n]`,
  notes/linear_arrays.md), which a host can read as one block without per-item calls.
- Release build, one CPU thread: `xs = float[10^7]; sum(xs .* 2)` ≈ 0.2–0.3 s, i.e. 20–30 ns per item for map + sum
  (noisy, process start included). The element-wise loop is lowered early (library_words.rs), not one fused kernel.

## Which operations
1. Element-wise `.+ .- .* ./ .^` between a number list and a number, and between two lists of equal length.
2. Reductions `sum`, `dot` (= sum of a .* b), `min`, `max`, `count where`.
3. `xs.map(f)` with f a pure numeric function (no calls but math builtins, no allocation): the body translated to WGSL.
4. Fusion: an expression of 1–3 over the same lists, `sum((xs .- mean) .^ 2)`, is ONE kernel. Without fusion each
   operator is a round trip and a single cheap op never beats the CPU (it is memory bound: the copy costs more than
   the arithmetic).

## Precision (P214, user 2026-10-08: option a)
WGSL has no f64 and no i64 (the shader-f64 feature is native-only and missing on Metal); warp floats are f64, Ints i64.
- Automatic offloading only where the result is identical: Int lists whose items fit i32, computed in i32 with an
  overflow flag the kernel sets (then the CPU redoes it).
- Floats only when the program allows f32: `@gpu` on the expression, or a `float32[n]` list. Where `@gpu` cannot apply
  (no adapter, an unsupported operation, big Ints) it runs on the CPU with a warning or hint saying why.
- Err on the CPU side (P214 addition): start conservative, ≥ 10^7 items or fused chains / data resident on the GPU
  only; a single cheap op like `xs .* 3` rarely pays off. Measurements set the real bounds later (card gpu-threshold).
Rejected: (b) automatic f32 for floats (results differ in the 7th digit), (c) double-float emulation.

## Threshold and transfer cost
- Cost per GPU call: dispatch + readback ≈ 50–200 µs (map_async round trip), plus the copy: wasm GC array → host
  buffer (wasmtime ArrayRef reads, item by item; browser: JS reads of the GC array) → GPU buffer (Apple: unified
  memory, Vulkan/DX12: PCIe), and back unless the result is a reduction (one number back).
- So: reductions and fused chains pay off first; element-wise results that must come back as a list need the
  biggest n. Start with one measured constant per operation class (expect 10^5–10^6 items), in one place
  (list_dispatch.rs, run-time length check as typed_lists.md describes), so a short list never pays the transfer and
  the program cannot tell which ran.
- Linear memory blocks (linear_arrays.md) avoid the per-item read: the compiler may pick them for a list the GPU takes
  (the linear-memory card's "compiler picks it" step).

## Fallback
- No adapter (CI, a browser without WebGPU, a built site without task Workers): the CPU path, silently: the result
  is the same by (a). A GPU failure after the adapter exists (out of memory, device lost) falls back and prints one
  loud warning per run.
- Big Ints (handles beyond the fixnum range) and non-number items: CPU.

## Steps
1. Done (branch gpu-vectors): `xs .op ys` of two lists pairs the items (broadcasting.rs paired_lists: both lists held
   once under paired_left_N / paired_right_N, `(1 to n).map(i => l#i op r#i)`, different lengths raise "the lists
   differ in length"); a list is a list literal, a variable only ever assigned lists (now also `float[n]` and
   element-wise results) or an element-wise expression. `dot(xs, ys)` is `sum(xs .* ys)` unless the program defines
   dot. Tests: tests/lists/test_element_wise_lists.rs. CPU cost (release, process start included): dot of two
   float[10^6] 0.47 s, of 10^7 2.05 s (~200 ns an item: indexing through the closure map), slower than
   `sum(xs .* 2)` (20–30 ns): a fused loop for the paired form is worth doing with the GPU kernel's lowering.
2. Measured (probes/webgpu/threshold.sh, release, M-series, machine busy so ±50 %): through today's gpu_compute the
   GPU never pays off. Milliseconds:

   | n      | transfer only (idle shader) | transfer + doubling | CPU `sum(xs .* 2)` |
   |--------|-----------------------------|---------------------|--------------------|
   | 10^4   | 12–17                       | 11–43               | 0–1                |
   | 10^5   | 104                         | 102                 | 3                  |
   | 10^6   | 1073–1665                   | 1115–1478           | 34–87              |
   | 3·10^6 | 5929                        | 3150                | 93                 |

   ~1 µs per item, all of it marshalling: host.rs gpu_compute reads the list as a Node (cons cells → Vec<Node>),
   converts each number, and builds the result through built_in_program; the shader's work is lost in the noise.
   The CPU is 20–30 ns per item. Consequence for step 3: the automatic path must not go through Nodes. The list is
   copied by a wasm loop into linear memory (~1 ns/item, i64 → i32/f32), the host word takes (pointer, length) and
   hands the slice to wgpu as is, and the result comes back the same way (a reduction: one number).
   Done: `gpu_compute(shader, xs, w)` of a `linear xs = float[n]` lowers to the host word gpu_compute_linear
   (shared_arrays.rs; host.rs natively, host-gpu.js in the browser), which reads the block's f64 cells, runs the
   shader over them as f32 and writes them back in place; `ys = gpu_compute(…, xs, …)` names the same block.
   Test: test_webgpu a_compute_shader_runs_over_a_linear_array_in_place. Measured the same way (ms):

   | n      | transfer only | transfer + doubling | CPU `sum(xs.map(x => x * 2))` of the linear array |
   |--------|---------------|---------------------|---------------------------------------------------|
   | 10^4   | 1             | 0                   | 1                                                  |
   | 10^5   | 3             | 2                   | 5                                                  |
   | 10^6   | 11            | 13                  | 53                                                 |
   | 10^7   | 114           | 118                 | 1776                                               |

   ~11 ns an item for the whole round trip (f64 → f32, upload, readback, back to f64), against 20–30 ns of the CPU's
   fused `sum(xs .* 2)` over a GC float list: break-even near 10^5 for a list coming back, lower for a reduction.
   The browser path (host-gpu.js) copies the cells into the task Worker's message, untested with a real adapter.
   CPU fusion first (done, broadcasting.rs fused_sum): `sum(xs .op k)`, `sum(xs .op ys)` and `dot` are one loop adding
   the items, no list built. Int lists, release, per item: `sum(xs .* 3)` 24 → 3.2 ns, `dot(xs, xs)` 190 → 15 ns
   (`sum xs` alone 3–4 ns). Test: test_element_wise_lists a_sum_of_an_element_wise_expression_is_fused.
   Finding: a fused CPU reduction (3–15 ns) beats the GPU round trip (~11 ns an item as f64, maybe ~5 as i32,
   + ~1 ms fixed) at every size, so automatic offloading of single Int reductions never pays off; the GPU wins only
   on heavy per-item work (map of sin/exp/pow, long fused chains) or data that stays on the GPU. Step 3 is therefore
   reduced to what still has a payoff, and the threshold card (gpu-threshold) should measure those, not sums.
   Fixed on the way (cards float-typed, list-literal): `xs#1 = 0.5` widened a float list to a Node list (variables.rs
   widen_element_type), and `[2 2] .* [2 4]` asked the list * number question for the lambda's `each_element * [2 4]`.
3. Dropped (gpu-threshold): Ints automatically never pay against the fused CPU loop (step 2 finding).
4. Fusion of an element-wise expression ending in a reduction into one WGSL kernel.
5. Floats per the Interviewer's answer; `map(f)` of a pure numeric f.
   Done for linear float arrays (src/lowering/gpu_maps.rs): `ys = xs.map(x => …) @gpu` (or `@gpu xs.map(…)`) of a
   lambda of + - * / ^ √ ‖‖ and the math words sin … atan2, min, max of the item and number literals becomes a WGSL
   kernel (whole powers up to 8 multiplied out: WGSL's pow of a negative base is NaN); shared_arrays.rs lowers it to
   `ys = linear_new(count(xs)); if gpu_map_linear(kernel, xs, ys, workgroups) == 0 { CPU loop }`. The host word gives
   0 without an adapter (a runtime warning, once) and the CPU maps the same lambda in f64. Where @gpu cannot apply (a
   list not in linear memory, a lambda WGSL cannot compute, a map not assigned, not a map) a compile-time warning says
   why and the map runs on the CPU as written. Tests: test_webgpu a_gpu_map_runs_a_numeric_lambda_as_a_kernel,
   a_gpu_map_the_gpu_cannot_run_says_why. Measured (probes/webgpu/threshold.sh, release, ms):

   | n      | @gpu `x * 2 + 1` | CPU (f64x2 kernel) | @gpu `sin(x) * cos(x) + √x` | CPU (generic map) |
   |--------|------------------|--------------------|-----------------------------|-------------------|
   | 10^3   | 2                | 0                  | 3                           | 0                 |
   | 10^4   | 9–13             | 0                  | 4–24                        | 2                 |
   | 3·10^4 | 4                | 0                  | 2                           | 8                 |
   | 10^5   | 2                | 0                  | 3                           | 16                |
   | 10^6   | 13               | 1                  | 12                          | 167               |
   | 10^7   | 129              | 11                 | 122                         | 7542              |

   A light lambda never pays (the CPU's SIMD kernel is ~1 ns an item); a heavy one breaks even near 2–3·10^4 items and
   wins 14× at 10^6, 60× at 10^7. The CPU's generic map of a linear array grew faster than linear (167 → 754 ns an
   item from 10^6 to 10^7): it mapped the array collected into a list grown item by item. Fixed (card linear-map,
   shared_arrays.rs numeric_map, collected): a pure numeric lambda maps in one loop into a new block (`sin(x)`: 68 ns
   an item at 10^7, most of it warp's sin), and an array read as a whole fills a `float[n]` / `int[n]` list
   (`sum(xs)` 71 → 25 ns an item). The heavy CPU column then reads 17, 281, 1871 ms at 10^5, 10^6, 10^7 (noisy):
   the GPU still wins 15–20× from 10^6.
   Thresholds (card gpu-threshold, P214 err on the CPU side), from these measurements:
   - automatic offload (no @gpu): none. Int reductions and element-wise ops are memory bound; the fused CPU loop
     (3–15 ns an item) beats the GPU round trip (~12 ns + ~2 ms) at every size, so step 3 is dropped.
   - `@gpu` map of a light lambda (only arithmetic: + - * / √ ‖‖ min max floor, whole powers): the CPU, in f64, with
     a warning saying so (gpu_maps.rs Kernel.heavy).
   - `@gpu` map of a heavy lambda (math words, powers): the GPU from GPU_MAP_MIN_COUNT = 32768 items (2^15, above the
     measured ~3·10^4 break-even), the CPU below, checked at run time in the lowered map.
   Test: test_webgpu a_gpu_map_runs_on_the_cpu_where_that_is_faster (below the count the value equals the f64 one).
   f32 note: `sin` of large arguments loses digits on the GPU (sin(13333.3) differs in the 3rd digit, the argument
   itself rounds to f32): @gpu is the program's consent to that.
   Outer numbers (done): a lambda may read the program's numbers (`x => sin(x) * k + shift`): the kernel appends
   their values to `data` after the items (`data[count + i]`, count = arrayLength - their number), so no second
   binding and no shader recompiled per value; gpu_map_linear takes them as a list (a value not a number: an error).
   "Light" is now a cost class, not the f64x2 kernel: a lambda without heavy math words (sin … log, atan2) or
   non-whole powers stays on the CPU, outer numbers included (`x => x * k`). Test: a_gpu_map_reads_outer_numbers.
   Chains (done, gpu_maps.rs fused): `xs.map(f).map(g)`, with or without @gpu, is one map of g after f (g's
   parameter replaced by f's body), so no list of f's results and one GPU round trip. Map results found until none
   is added: `ys = xs.map(f); zs = ys.map(g)` writes ys a block, and zs maps it as one. Measured (release, warm
   shader cache, `sin(x)` then `y * y + 1`, ms; noisy, other sessions building):

   | n      | CPU chain | @gpu chain |
   |--------|-----------|------------|
   | 10^5   | 11–17     | 47–69      |
   | 10^6   | 140       | 80–108     |
   | 10^7   | 1290–1480 | 280–320    |

   Before the fusion the CPU chain mapped f's results as a GC list grown by `out = out + [x]`, quadratic (card
   map-filter: any `map` of a list, `int[20000].map(x => x * 2)` 0.5 s, 80000 out of fuel).
   GC lists (done, shared_arrays.rs copied_into_block): `ys = xs.map(f) @gpu` of a list not in linear memory
   (`xs = float[n]`, a literal, a call's result) copies its items as floats into a block once, then maps that; ys is
   a linear float array. A non-number item is a run-time error of `float`. Measured (release, `sin(x)`, ms, copy
   included): 10^5 CPU 8 / @gpu 31, 10^6 87 / 45, 10^7 614 / 252. (Filling float[10^7] item by item exhausts the
   default fuel; WARP_FUEL=10^11 for that row.) Test: test_webgpu a_gpu_map_takes_a_list_of_floats.
   Across statements (done, gpu_maps.rs kept_on_gpu): `ys = xs.map(f) @gpu; zs = ys.map(g) @gpu` where only later
   @gpu maps read ys becomes `zs = xs.map(f).map(g) @gpu`: ys never comes back from the GPU. Not when ys is read
   elsewhere, or xs or a number f reads is written after ys's statement (ys keeps the values before). The round trip
   (upload, dispatch, readback, f64↔f32) is the cost, not the kernel: 10^7 sin then cos, ms: two maps 272–321, folded
   143–188, one map alone ~150 (load average 20–30, noisy). Test: a_gpu_map_result_mapped_again_stays_on_the_gpu.
   Reductions (done, gpu_maps.rs Reduction + reduce_shader, shared_arrays.rs gpu_reduced, host word
   gpu_reduce_linear): `s = sum(xs.map(f) @gpu)`, min, max. Each workgroup of 256 maps its items and folds them in
   shared memory (tree of halvings), one partial per group is written after the data, only those come back (n/256
   floats), the CPU combines them. Below GPU_MAP_MIN_COUNT or without an adapter, the reduction as written runs.
   f32: sum of 10^7 sin·exp is 9093306.84 against 9093307.88 in f64 (1e-7 relative). Measured (release,
   probes/webgpu/threshold.sh, `sin(x) * exp(x)`, GC list copied in, ms): 10^5 GPU 4–27 / CPU 15, 10^6 20 / 140,
   10^7 170–200 / 2800–3500 (the CPU side builds the mapped GC list, then sums). Tests: test_webgpu
   a_reduction_of_a_gpu_map_reads_back_partial_results, a_reduction_of_a_gpu_map_takes_a_list_of_floats.
   dot (done, CPU on purpose, P214): `dot(xs, ys)` is `sum(xs .* ys)` fused into one loop (broadcasting.rs
   PAIRED_SUM_TEMPLATE); of two linear float arrays it is linear_dotf (wasm_emitter/linear_arrays.rs), f64x2 lanes over
   both blocks after a length check (shared_arrays.rs paired_with_linear, which also pairs `xs .* ys` of linear
   arrays: before, the map read ys as one number, its block's address). Measured (release, 10^6, ms,
   probes/webgpu/dot_linear.warp): item by item over a products list 200, fused 51, linear_dotf 2–3; GC lists fused 13.
   No GPU dot: a multiply-add is ~1 ns an item in lanes, so uploading the two arrays (16 MB at 10^6) already costs
   more than the CPU's whole dot. It pays only for data already on the GPU (a dot of two @gpu maps: kept buffers).
   `float[n]` of a variable count was no list to broadcasting (`xs .* ys` was a float * list error): fixed.
   Sample: samples/dot.warp.
   Browser (done, host-gpu.js gpu_kernel): the block's cells go to the task Worker as a transferred Float32Array and
   come back as raw f32 bytes in the shared buffer (writeSharedFloats, state FLOATS_STATE), only from the first cell
   needed (the partials of a reduction). Before, 10^6 floats went as a JS array and came back as JSON: 160–350 ms, slower
   than the CPU. Measured with samples/gpu_map.warp (headless Chrome, Apple Metal, `sin(x) * exp(x) + cos(x * 3)`,
   10^6, ms): sum GPU 8–17 / CPU 80–160, the mapped list back 12–16; the first run 86 (device + shader). Raw WebGPU for
   that kernel is ~3 ms. SwiftShader (CI): correct, sum 464 ms, sin less precise (2e-5 relative). Chrome's Tint
   refuses the f32 literal 3.4028235e38 (it rounds above the largest f32): LARGEST_F32 is a bitcast. Careful when
   measuring: the page keeps an old host-gpu.js in the browser cache; open it with `?nocache=<n>` in a new session.
   Native runs of the sample show 41 ms for the sum: its first GPU call creates the device (gpu.rs keeps it per
   process); warmed up (threshold.sh) it is 20 ms.
   Kept buffers (native): a result read on the CPU (ys#i, #ys, text holes) and then mapped or reduced @gpu again keeps
   its storage buffer on the GPU (gpu.rs KEPT, the last 4 blocks); the later kernel copies from it instead of uploading.
   gpu_maps.rs marked_kept sets the flags (6th argument of gpu_map_linear / gpu_reduce_linear: KEEP_RESULT 1,
   SOURCE_KEPT 2); a write to ys or a call that hands it on between them drops both. 10^7 items, second map: 125–172 →
   68–84 ms. Missing kept buffer → plain upload. Only a producer that maps on the GPU (a heavy lambda) is kept; a newly
   kept block replaces an earlier run's buffer for the same address (addresses recur across runs in one process).
   Browser (card gpu-kept): GPU jobs go to the task Worker of the last GPU job (host-gpu.js gpuTaskWorker), which keeps
   the buffers (keptBuffers, by block and count); busy or missing → another worker / an upload (KEPT_MISSING, retried).
   Headless Chrome, samples/gpu_map.warp: the kept-buffer map 18 ms at 10^6, same result as native.
