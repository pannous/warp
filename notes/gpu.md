# GPU vectors (card gpu-vectors, user 2026-10-08)

User: list arithmetic on large number lists runs as WebGPU compute (wgpu natively, WebGPU in the playground) above a
size threshold, on the CPU below it and without an adapter; tests compare GPU and CPU results. The GPU path reuses the
gpu_compute / gpu_render plumbing (src/gpu.rs, web/playground/host-gpu.js, notes/web_framework.md "web-apis: WebGPU").

## Where the language stands (2026-10-08, measured on main)
- `xs * 2` is an error by decision (D3: repeat or multiply?), `xs + ys` concatenates. Element-wise is the dotted form
  (P166): `xs .* 2` works, `[1 2 3] .* 2` → `[2 4 6]`.
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
3. Ints (option a): `sum`, `dot`, element-wise over $IntList through a host word with an overflow flag, behind the
   length check; tests compare GPU and CPU on lists above and below the threshold.
4. Fusion of an element-wise expression ending in a reduction into one WGSL kernel.
5. Floats per the Interviewer's answer; `map(f)` of a pure numeric f.
