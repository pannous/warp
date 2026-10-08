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

## Precision: the open question (conflicts with P118)
WGSL has no f64 and no i64 (the shader-f64 feature is native-only and missing on Metal). warp floats are f64, Ints i64.
P118 (user, earlier): GPU only on an explicit `@gpu` map, never silent offloading, because f32 results differ. The new
card asks for an automatic threshold. Options, asked at the Interviewer:
- (a, recommended) automatic only where the result is identical: Int lists whose items fit i32, computed in i32 with an
  overflow flag the kernel sets (then the CPU redoes it); float lists go to the GPU only when the program says f32 is
  fine (`@gpu` on the expression, or a `float32[n]` list).
- (b) automatic for floats too above the threshold: results rounded to f32 (~7 digits), tests compare with a tolerance.
- (c) double-float emulation (two f32 per value, ~48-bit mantissa): closer to f64 but not identical, 4–10x slower.
Default until answered: (a).

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
1. CPU first: `xs .op ys` for two lists (equal length, else a clear error); `dot`. Tests pin the values.
2. Measure: a probe timing CPU vs GPU for n = 10^4…10^7 on sum, dot, .* then sum, map(sin), with the copy separated
   from the compute; the thresholds come from it.
3. Ints (option a): `sum`, `dot`, element-wise over $IntList through a host word with an overflow flag, behind the
   length check; tests compare GPU and CPU on lists above and below the threshold.
4. Fusion of an element-wise expression ending in a reduction into one WGSL kernel.
5. Floats per the Interviewer's answer; `map(f)` of a pure numeric f.
