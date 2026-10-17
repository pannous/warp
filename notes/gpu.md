# GPU vectors (card gpu-vectors, user 2026-10-08)

Roadmap card: fast-numbers (board topic; its sub-cards are the steps and leftovers below, notes/roadmaps.md).

In general, once we enter the regime of GPUs, we are no longer interested in precision (especially for neural nets it has been shown that less precision is even better). 

User: list arithmetic on large number lists runs as WebGPU compute (wgpu natively, WebGPU in the playground) above a
size threshold, on the CPU below it and without an adapter; tests compare GPU and CPU results. The GPU path reuses the
gpu_compute / gpu_render plumbing (src/graphics/gpu.rs, web/playground/host-gpu.js, notes/web_framework.md "web-apis: WebGPU").

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

## Precision (P240, user 2026-10-10, revises P214: GPU results are imprecise by design)
WGSL has no f64 and no i64 (the shader-f64 feature is native-only and missing on Metal); warp floats are f64, Ints i64.
- GPU calculations are f32 and approximate by design, and the user is told: a one-time run-time notice when a map
  moves to the GPU by itself (gpu_maps.rs AUTOMATIC_NOTICE, native diagnostic::report_runtime_warning_once, browser
  host-gpu.js warnOnce), and the guides' "Whole lists at once". The error grows with the item count: a software
  adapter's sum of 40000 cosines may be off by 40000 × 2^-11.
- Automatic switching (card gpu-auto): a heavy map of a linear float array (gpu_maps.rs automatic: assigned, or inside
  sum/min/max, without @gpu or @cpu, Kernel.heavy) goes to the GPU from GPU_AUTO_MIN_COUNT = 10 × GPU_MAP_MIN_COUNT
  (327680 items: one order of magnitude of room for other architectures), checked at run time (flag AUTOMATIC = 4 in
  the keeping argument). Without an adapter it stays on the CPU silently (no warning, unlike @gpu).
  `go xs.map(x => sin(x))` is that plain map too (parallel.rs kernel_map, card go-heavy-map): tasks copying Node slices
  took 50 s for 10 maps of 300k items, the CPU loop 1 s (probes/perf/go_heavy_map.warp).
- Exactness on demand: `@cpu` on a map (`xs.map(f) @cpu`, `@cpu xs.map(f)`), a block, a function or the program keeps
  it on the CPU in f64 and overrides the switch; `@gpu` with `@cpu` on one map is a compile error. `WARP_GPU=off`
  pins a whole run (tests, benchmarks) to the CPU, @gpu included.
- Tests comparing GPU and CPU results use @cpu for the reference side and common::gpu_tolerance.
- Int lists: never automatic (step 3: the fused CPU loop always wins).
Superseded (P214): floats on the GPU only with @gpu or float32[n]; (b) automatic f32 was rejected then.

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
   Done for linear float arrays (src/lowering/foreign/gpu_maps.rs): `ys = xs.map(x => …) @gpu` (or `@gpu xs.map(…)`) of a
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
   Sample: samples/lists/dot.warp.
   Browser (done, host-gpu.js gpu_kernel): the block's cells go to the task Worker as a transferred Float32Array and
   come back as raw f32 bytes in the shared buffer (writeSharedFloats, state FLOATS_STATE), only from the first cell
   needed (the partials of a reduction). Before, 10^6 floats went as a JS array and came back as JSON: 160–350 ms, slower
   than the CPU. Measured with samples/gpu/gpu_map.warp (headless Chrome, Apple Metal, `sin(x) * exp(x) + cos(x * 3)`,
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
   Headless Chrome, samples/gpu/gpu_map.warp: the kept-buffer map 18 ms at 10^6, same result as native.

## Painting a shader every frame (samples/gpu/gpu_visualizer.warp, 2026-10-10)
- src/graphics/gpu.rs `painter` keeps the compiled module and render pipeline per shader text (16, as host-gpu.js
  MOST_COMPILED): before, every paint compiled the pipeline anew, 10–20 ms for the raymarching sample on Metal plus
  driver work, now only the uniform's bind group is made per frame. Test: test_webgpu a_kept_shader_reads_each_frames_values.
- Measured (debug build, M-series, WARP_NO_WINDOW): the raymarcher's draw + readback 9 ms at 640×360; a paint is
  ~66 ms even for a one-line shader (probes/gpu/paint_cost.warp): pixels → Node list → PNG in a debug build.
  That host side, not the GPU, bounds the frame rate.
- WGSL raymarching tip: a distance field over angular sectors (bars whose height differs per band) must take the
  neighbor sectors into account (min over beside = -1, 0, 1), or rays step into a taller neighbor: stair artifacts.
- RGBA fast path (gpu-polish): a shader's paint goes gpu::render_rgba → paint::paint_rgba → the window as tight RGBA
  bytes, no Node list: 1280×720 went from ~84 ms to ~5 ms per frame (debug). Headless, PNG deflate is fast mode.
- The viewer shows a frame of 1024+ pixels across pixel for pixel (PhysicalSize, crisp on Retina), smaller ones scaled
  up by a whole factor (paint_window.rs shown_size).
- Beat reaction in the sample: each band against its own fading peak (auto-gain), kick = the summed rise of all bands,
  fading 0.85 per frame; tone = the loudest band, eased. Spectrum of a note is clean (one or two bands), so per-band
  gain doesn't amplify leakage.

## Shader built-ins (card shader-builtins, user 2026-10-10)
$width $height $size $time $frame $mouse $mouse_down $key need no variable: shader_holes.rs BUILTIN_HOLES leaves them
out of the values map unless the program assigns that name; the host fills those the shader reads (`values.<name>`):
natively gpu::with_builtin_values (input from the paint window's stdout lines `input x y down key`,
paint_window::input), in the browser host-gpu.js builtinValues (pointer and key from canvas.js's shared buffer).
time = seconds since the first render, frame = renders before, key = code point, arrows 0xF700–0xF703, 0 = none.
Costs nothing for a shader that doesn't mention them.

## $previous: a shader's own last frame (card shader-read, user wish 2026-10-12)
`$previous(at)` is the color the shader left at pixel `at` (vec2f, clamped to the image) in its previous render of
the same size, black before the first (another paper: `select(paper, $previous(at), $frame > 0)`): painting with
memory (samples/graphics/shader_trails.warp). $last, $feedback, $before are not taken from programs; a value missing under
them hints at $previous (shader_holes.rs PREVIOUS_WORDS, user's Interviewer: canonical word plus hints). Not a value but a
function: BUILTIN_HOLES has "previous" so no values entry is made; the host rewrites `values.previous(` to
`warp_previous(` and appends PREVIOUS_FRAME (texture_2d at @group(0) @binding(1), textureLoad). Each render draws
into a new texture and keeps it per shader text (native gpu.rs PREVIOUS_FRAMES, ≤ PAINTERS_KEPT; browser host-gpu.js
previousFrames); a canvas paint then copies it into the canvas texture (configured COPY_DST). Kept as 8 bits per
channel, so small fades round: subtract a step (≥ 1/255) rather than multiply by 0.99, which stalls at ~33/255.

## WGSL errors at their place in the program (card wgsl-error, user 2026-10-13)
WGSL in `shader { … }` stays as written (no warp syntax converted into it; only `$name` holes). Both hosts report a
shader that does not compile as `<word>: line:column: why` in the shader's text (native gpu.rs compile_errors, browser
host-gpu.js checkedModule, `gpu_paint: …` there). pipeline::eval_source passes the failed program through
shader_errors::relocated: it finds the program's shader blocks again in the source (warp_parser shader_blocks, the
same dedent as the block's text: lines dropped around it, characters dropped before each line) and, when exactly one
block has that place, says `<word>: why at line:column` in the program, which the CLI and the playground underline.
Columns count `$name` as written, not as `values.name`. Two blocks that both have the place: the message stays as the
GPU said it (no guess). Not relocated: shaders from imported modules, shaders given as quoted text.

## GLSL from twigl.app: removed (user 2026-10-16)
twigl and shadertoy shaders (`twigl { … }`, `paint(twigl(code))`, shadertoy(code), lib/draw.warp glsl_shader) are gone
from the compiler, to keep Rust lean: naga's GLSL front end (crates/warp-glsl, in the browser a 705 KB glsl.wasm loaded
on first use), the parser's dialect block, the draw.glsl_wgsl adapter, samples/twigl.warp, tests/programs/test_twigl*.rs
and probes/twigl. The last state with all of it is commit b04da3e2c (`git show b04da3e2c:notes/gpu.md` has this
section in full): a `use twigl` module can come back from there (card twigl-module). Translating GLSL to WGSL helps no
WebGL-only browser: its WGSL still needs WebGPU.

## Paint window, default canvas, probes (from the warp-sound handover, 2026-10-10)
- src/graphics/paint_window.rs is a separate `warp paint-window` process fed RGBA frames on stdin; it writes
  `input x y down key` lines on stdout, which warp-runtime system_values.rs window_input keeps (system values and
  shader built-ins both read it). $key holds only the last key pressed.
- Default canvas: lib/draw.warp sizes it from system values view_width / view_height (the playground's output pane
  via playground.js viewSize; natively 640×480). notes/web_playground.md "The default canvas is the output pane".
- Probes: probes/gpu/paint_cost.warp, render_speed.warp, web_speed.py (paint and render timing);
  probes/paint_window.sh (the viewer process); probes/webgpu/ (gpu_compute thresholds). tests/web/test_playground_paint.rs
  runs host.js under node, a harness for playground host behaviour without a browser (compile with `pipeline::for_a_page`).
- Quirks: run paint probes under WARP_NO_WINDOW=1, or windows pop up on the user's Mac; headless, paint writes PNGs
  to $TMPDIR/warp-paint, shared by every session (filter frames by size and mtime). A headless 1080p animation is
  bound by PNG writing; the window path isn't. A shader built-in is skipped when the program assigns a variable of
  that name (gpu_visualizer keeps its own `time`, the song's clock). `sleep 16ms` takes a unit on its number.

## Warp functions on the GPU (card gpu-functions, plan 2026-10-13)
Step (a), card paint-keeps: a shader painted in a window renders in the paint-window process; per frame only the
shader text and its values cross (~6 KB, not an 8 MB readback), and the window draws the newest frame. Step (b), this
plan: warp functions compiled to WGSL, so the work around a shader (spectrum(), a map's helper functions) runs there too.

- **The subset that compiles.** It's the kernel translation of gpu_maps.rs (`Translation::expression`), widened from
  one lambda expression to a function body:
  - numbers in f32 (P240: imprecise by design), i32 for ints; vectors of 2–4 numbers as vec2f…vec4f
  - local variables, `if`/`else`, `for i in a..b`, `while`
  - the math words (sin cos tan sqrt exp log pow atan2 min max abs floor fract round clamp)
  - calls of other functions in the subset, each emitted as its own WGSL `fn`
  - Refused, with the reason named: texts, lists of variable length, maps, closures, allocation, host words,
    exceptions, recursion (WGSL has none).
- **Types.** A parameter's type comes from the call site that sends it to the GPU: an f32 item, a program number, a
  pixel coordinate. One WGSL function is made per signature (shares the specialization with card untyped-parameters).
- **Which functions go to the GPU.**
  1. A map kernel may call user functions. Today its lambda may only use operators and math words.
     `xs.map(x => shade(x))` translates shade() and its callees; the threshold rule stays (heavy kernels auto
     from GPU_AUTO_MIN_COUNT, `@gpu` forces).
  2. `paint(f, width, height)` with a warp function `f(x, y) -> color` instead of a WGSL text: f becomes the fragment
     shader; the program's numbers it reads become `values` (uniforms, as for text shaders).
  3. `@gpu` on a function: every call from a heavy map or a paint function compiles it, and a function outside the
     subset is a warning that names the construct (an error under strict).
- **Precision** (user 2026-10-13: "Math is exact by default, except when doing lots of computation. GPU should be
  F32 too by default. With a little warning. It's only applied for mass calculations, anyways."): P240 stands. Small
  computations stay exact in f64; mass work (a heavy map from GPU_AUTO_MIN_COUNT items) goes to the GPU in f32 by
  itself, with the one-time notice. The functions such a map calls go with it, in f32 under the same notice.
  `@gpu` and paint ask for the GPU outright; `@cpu` and WARP_GPU=off keep f64.
  On the GPU itself precision doesn't matter (user 2026-10-13): every number is an f32, ints and `x as float`
  included, with no effort spent on exactness. Colors are the one exception and stay exact: a packed 0xAARRGGBB int
  in an f32 would garble its channels (alpha pushes it past 2^24). So a function giving rgb(…) (hsv and other callers
  of rgb too) becomes a WGSL fn returning a vec4f, a packed color literal is unpacked at compile time, and any other
  number a painted function gives is ink unless 0, as in paint(pixels, …).
- **Fallback.** As for maps:
  - not translatable: runs on the CPU as written, warned once
  - no adapter: the CPU, silently
  - `@cpu` / WARP_GPU=off: the CPU in f64
  - the browser runs the same WGSL through host-gpu.js
- **Steps**, each with its test under tests/wasm/ or tests/programs/:
  1. Translate statements: let, assignment, if, for over a range, return.
  2. Calls of user functions, with a callee closure and a recursion check.
  3. Map kernels calling user functions.
  4. paint(f, …) of a warp function.
  5. Acceptance: samples/gpu/gpu_visualizer.warp's tunnel shader written in warp gives the same frame as the WGSL one.
- **Progress.** Done 2026-10-13: steps 1–3 (gpu_maps.rs `Translation::function`, tests/web/test_gpu_functions.rs; 1M items of
  a function with a loop of cos: 324 ms @cpu → 29 ms on the GPU). Step 4: `paint(f, width, height)` lowers to a
  fragment shader of f (gpu_maps.rs `painted_functions`, tests/programs/test_paint_function.rs), the program's
  numbers f reads as its values; f outside the subset, `@cpu` or WARP_GPU=off: f of each pixel on the CPU
  (probes/gpu/paint_mandelbrot.warp, full HD, debug build: 18.4 s on the CPU → 1.6 s, mostly the PNG). Open: no
  adapter falls back to the CPU only for maps, a painted function fails like a shader text (card paint-function).
  Next, after the Rust cleanup (user decision cleanup-only, 2026-10-13): step 5, the tunnel shader of
  samples/gpu/gpu_visualizer.warp written in warp. It needs vectors (vec2f…vec4f) in the translation, and arrays as
  values (`levels`).

## Shaders in the browsers' playground (cards shaders-firefox, https-pannous, very-fast, 2026-10-14)
- CI's browsers had no real canvas path: Linux Firefox keeps WebGPU off (every GPU example skipped as "no adapter"),
  Chrome's SwiftShader drops the page canvas, so shaders came back as pixels. The Firefox tour now turns WebGPU on
  (FIREFOX_PREFS `dom.webgpu.enabled`, `gfx.webgpu.ignore-blocklist`, lavapipe from mesa-vulkan-drivers): the only
  CI browser drawing a shader into the page's canvas, as users' browsers with a GPU do.
- Firefox lost an OffscreenCanvas posted into a MessagePort whose other end was itself still on its way to the task
  Worker (the program's Worker made the channel); only a Firefox-internal "uncaught exception: undefined" said so, and
  the program hung in Atomics.wait. The page now posts the canvas straight to the task Worker it made
  (task-workers.js postToTaskWorker, by `taskWorkerId`).
- An `on frame` painting arrives after the run's report (playground.js showEventOutput), where the GPU canvas's id
  was never mapped to its canvas: the page showed "1" instead of the brush, so on any real GPU the `on frame` shader
  samples vanished after their first frame. paintingOf maps it in both places; a pixel frame is redrawn in place.
- Probes: probes/firefox_webgpu/check.sh (WebGPU steps on the page and in a Worker), variants.sh (programs run one by
  one with the console after each); firefox_driver.mjs takes FIREFOX_PREFS and names an error's line and realm.
- Firefox's "uncaught exception: undefined" during 400 GPU jobs (card firefox-gpu-jobs-uncaught) is no WebGPU fault: on
  lavapipe 400 jobs take over 10 s, the playground stops the run, and terminating a Worker inside WebAssembly.promising
  logs it (Firefox's bug, card firefox-logs). gpu_jobs.sh (run 38683768383): 1 and 50 jobs clean, every 400 stopped and
  logged. The tour's check accepts a stop ("or says why not"), so it stays green there.
