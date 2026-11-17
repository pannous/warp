# GPU vectors

Roadmap card: fast-numbers.

In general, once we enter the regime of GPUs, we are no longer interested in precision (especially for neural nets it has been shown that less precision is even better). 

List arithmetic on large number lists runs as WebGPU compute (wgpu natively, WebGPU in the playground) above a size
threshold, on the CPU below it and without an adapter; tests compare GPU and CPU results. The GPU path reuses the
gpu_compute / gpu_render plumbing (lib/host/native_host/gpu.warp, web/playground/host-gpu.js, notes/web_framework.md "web-apis: WebGPU").

## In warp now
- gpu_compute(shader, arrays, workgroups) and gpu_render(shader, width, height, values) are host words a library
  answers: natively the native host module (lib/host/native_host/gpu.warp host_gpu_compute, host_gpu_render over
  libwgpu_native by FFI, notes/native_host.md "The GPU"), in the page web/playground/host-gpu.js over WebGPU. They are
  the only GPU words; `@gpu` maps are to go through gpu_compute (card xs-map-gpu).
- GPU lists `[f32]` (decision gpu-bound-floats, extensions/gpu_lists/runtime.warp): f32 cells in linear memory from their
  creation, a square List $Node marked KindUnit.GpuList whose Int is (address << 32) + count. `xs: [f32] = …` asks for
  one; a list the program makes once (literal, comprehension, `float[n]`) at its main level and uses only by #, xs#i,
  xs#i = v, for, print and gpu_compute is one by itself (extensions/rules/gpu_packing.warp gpu_lists_packed,
  gpu_bound), as is a creating form written in a gpu_compute call. gpu.warp binds the cells as the storage buffer
  and writes results back into them (array_left): samples/acceleration/gpu_million.warp went 852 + 1686 ms → 138 +
  20 ms. Other list words refuse a `[f32]` list today; card f32-gpu gives reads a boxed copy (one function, which
  xs-map-gpu replaces by a kernel) and growth (a [count, capacity, cells] header, vector doubling). Linear memory is
  never freed yet (card packed-f32-memory).
- Not yet in warp: automatic switching above GPU_AUTO_MIN_COUNT, `@gpu`/`@cpu` maps and reductions (card xs-map-gpu),
  kept buffers. The design below stands for them.

## The language side
- `xs * 2` is an error by decision (D3: repeat or multiply?), `xs + ys` concatenates. Element-wise is the dotted form
  (P166): `[1 2 3] .* 2` → `[2 4 6]`; the inner product of two lists is `dot(xs, ys)`.
- `xs .op ys` of two lists pairs the items (different lengths raise
  "the lists differ in length"); a list is a list literal, a variable only ever assigned lists (also `float[n]` and
  element-wise results) or an element-wise expression. `dot(xs, ys)` is `sum(xs .* ys)` unless the program defines
  dot. Tests: test/lists/test_element_wise_lists.warp.
- Number lists of known type are wasm GC arrays ($IntList / $FloatList), or
  linear memory (`linear xs = float[n]`, wiki/array.md), which a host reads as one block.

## Which operations
1. Element-wise `.+ .- .* ./ .^` between a number list and a number, and between two lists of equal length.
2. Reductions `sum`, `dot` (= sum of a .* b), `min`, `max`, `count where`.
3. `xs.map(f)` with f a pure numeric function (no calls but math builtins, no allocation): the body translated to WGSL.
4. Fusion: an expression of 1–3 over the same lists, `sum((xs .- mean) .^ 2)`, is ONE kernel. Without fusion each
   operator is a round trip and a single cheap op never beats the CPU (it is memory bound: the copy costs more than
   the arithmetic).

## Precision (P240: GPU results are imprecise by design)
WGSL has no f64 and no i64 (the shader-f64 feature is native-only and missing on Metal); warp floats are f64, Ints i64.
- GPU calculations are f32 and approximate by design, and the user is told: a one-time run-time notice when a map
  moves to the GPU by itself (browser host-gpu.js warnOnce), and the guides' "Whole lists at once". The error grows with the item count: a software
  adapter's sum of 40000 cosines may be off by 40000 × 2^-11.
- Automatic switching: a heavy map of a linear float array (assigned, or inside
  sum/min/max, without @gpu or @cpu, a heavy kernel) goes to the GPU from GPU_AUTO_MIN_COUNT = 10 × GPU_MAP_MIN_COUNT
  (327680 items: one order of magnitude of room for other architectures), checked at run time. Without an adapter it
  stays on the CPU silently (no warning, unlike @gpu).
  `go xs.map(x => sin(x))` is that plain map too: tasks copying Node slices are ~50× slower
  than the CPU loop.
- Exactness on demand: `@cpu` on a map (`xs.map(f) @cpu`, `@cpu xs.map(f)`), a block, a function or the program keeps
  it on the CPU in f64 and overrides the switch; `@gpu` with `@cpu` on one map is a compile error. `paint_on_gpu = no`
  pins a whole run (tests, benchmarks) to the CPU, @gpu included.
- Tests comparing GPU and CPU results use @cpu for the reference side and a tolerance.
- Int lists: never automatic (the fused CPU loop always wins).

## Threshold and transfer cost
- Cost per GPU call: dispatch + readback ≈ 50–200 µs (map_async round trip), plus the copy: wasm GC array → host
  buffer (wasmtime ArrayRef reads, item by item; browser: JS reads of the GC array) → GPU buffer (Apple: unified
  memory, Vulkan/DX12: PCIe), and back unless the result is a reduction (one number back).
- So: reductions and fused chains pay off first; element-wise results that must come back as a list need the
  biggest n. Start with one measured constant per operation class (expect 10^5–10^6 items), in one place
  (a run-time length check), so a short list never pays the transfer and
  the program cannot tell which ran.
- Linear memory blocks (wiki/array.md) avoid the per-item read: the compiler may pick them for a list the GPU takes
  (the linear-memory card's "compiler picks it" step).

## Fallback
- No adapter (CI, a browser without WebGPU, a built site without task Workers): the CPU path, silently: the result
  is the same. A GPU failure after the adapter exists (out of memory, device lost) falls back and prints one
  loud warning per run.
- Big Ints (handles beyond the fixnum range) and non-number items: CPU.

## What pays on the GPU (measured, release, M-series)
- The round trip through a linear block (f64 → f32, upload, readback, back) costs ~11 ns an item + ~1–2 ms fixed;
  going through Nodes (gpu_compute of a GC list) costs ~1 µs an item and never pays. `gpu_compute(shader, xs, w)` of
  a GPU list `[f32]` runs the shader over its cells as they are, in place.
- A fused CPU reduction (`sum(xs .op k)`, `sum(xs .op ys)` and `dot` are one loop, no list
  built; 3–15 ns an item) beats the GPU at every size, so Int reductions and element-wise ops are never offloaded.
  The GPU wins only on heavy per-item work (sin/exp/pow, long fused chains) or data that stays on the GPU.
- Open: fusion of an element-wise expression ending in a reduction into one WGSL kernel.

## @gpu maps (design for card xs-map-gpu; measured on the rust-bootstrap implementation)
   `ys = xs.map(x => …) @gpu` (or `@gpu xs.map(…)`) of a
   lambda of + - * / ^ √ ‖‖ and the math words sin … atan2, min, max of the item and number literals becomes a WGSL
   kernel (whole powers up to 8 multiplied out: WGSL's pow of a negative base is NaN). Without an adapter (a runtime
   warning, once) the CPU maps the same lambda in f64. Where @gpu cannot apply (a lambda WGSL cannot compute, a map not
   assigned, not a map) a compile-time warning says why and the map runs on the CPU as written.
   A light lambda never pays (the CPU's SIMD kernel is ~1 ns an item); a heavy one breaks even near 2–3·10^4 items and
   wins 14× at 10^6, 60× at 10^7.
   Thresholds (P214: err on the CPU side), from these measurements:
   - automatic offload (no @gpu): none. Int reductions and element-wise ops are memory bound; the fused CPU loop
     (3–15 ns an item) beats the GPU round trip (~12 ns + ~2 ms) at every size.
   - `@gpu` map of a light lambda (only arithmetic: + - * / √ ‖‖ min max floor, whole powers): the CPU, in f64, with
     a warning saying so.
   - `@gpu` map of a heavy lambda (math words, powers): the GPU from GPU_MAP_MIN_COUNT = 32768 items (2^15, above the
     measured ~3·10^4 break-even), the CPU below, checked at run time in the lowered map.
   f32 note: `sin` of large arguments loses digits on the GPU (sin(13333.3) differs in the 3rd digit, the argument
   itself rounds to f32): @gpu is the program's consent to that.
   Outer numbers: a lambda may read the program's numbers (`x => sin(x) * k + shift`): the kernel appends
   their values to `data` after the items (`data[count + i]`, count = arrayLength - their number), so no second
   binding and no shader recompiled per value (a value not a number: an error).
   "Light" is a cost class: a lambda without heavy math words (sin … log, atan2) or
   non-whole powers stays on the CPU, outer numbers included (`x => x * k`).
   Chains: `xs.map(f).map(g)`, with or without @gpu, is one map of g after f (g's
   parameter replaced by f's body), so no list of f's results and one GPU round trip. Map results found until none
   is added: `ys = xs.map(f); zs = ys.map(g)` writes ys a block, and zs maps it as one.
   GC lists: `ys = xs.map(f) @gpu` of a list not in linear memory
   (`xs = float[n]`, a literal, a call's result) copies its items as floats into a block once, then maps that; ys is
   a linear float array. A non-number item is a run-time error of `float`.
   Across statements: `ys = xs.map(f) @gpu; zs = ys.map(g) @gpu` where only later
   @gpu maps read ys becomes `zs = xs.map(f).map(g) @gpu`: ys never comes back from the GPU. Not when ys is read
   elsewhere, or xs or a number f reads is written after ys's statement (ys keeps the values before). The round trip
   (upload, dispatch, readback, f64↔f32) is the cost, not the kernel.
   Reductions: `s = sum(xs.map(f) @gpu)`, min, max. Each workgroup of 256 maps its items and folds them in
   shared memory (tree of halvings), one partial per group is written after the data, only those come back (n/256
   floats), the CPU combines them. Below GPU_MAP_MIN_COUNT or without an adapter, the reduction as written runs.
   f32: sum of 10^7 sin·exp is 9093306.84 against 9093307.88 in f64 (1e-7 relative).
   dot (CPU on purpose, P214): `dot(xs, ys)` is `sum(xs .* ys)` fused into one loop.
   No GPU dot: a multiply-add is ~1 ns an item in lanes, so uploading the two arrays (16 MB at 10^6) already costs
   more than the CPU's whole dot. It pays only for data already on the GPU (a dot of two @gpu maps: kept buffers).
   Sample: samples/lists/dot.warp.
   Browser (host-gpu.js gpu_kernel): the block's cells go to the task Worker as a transferred Float32Array and
   come back as raw f32 bytes in the shared buffer, only from the first cell
   needed (the partials of a reduction). Measured with samples/acceleration/gpu_map.warp (headless Chrome, Apple Metal, `sin(x) * exp(x) + cos(x * 3)`,
   10^6, ms): sum GPU 8–17 / CPU 80–160, the mapped list back 12–16; the first run 86 (device + shader). Raw WebGPU for
   that kernel is ~3 ms. SwiftShader (CI): correct, sum 464 ms, sin less precise (2e-5 relative). Chrome's Tint
   refuses the f32 literal 3.4028235e38 (it rounds above the largest f32). Careful when
   measuring: the page keeps an old host-gpu.js in the browser cache; open it with `?nocache=<n>` in a new session.
   Kept buffers: a result read on the CPU (ys#i, #ys, text holes) and then mapped or reduced @gpu again keeps
   its storage buffer on the GPU (the last 4 blocks); the later kernel copies from it instead of uploading.
   A write to ys or a call that hands it on between them drops it. Missing kept buffer → plain upload. Only a
   producer that maps on the GPU (a heavy lambda) is kept.
   Browser: GPU jobs go to the task Worker of the last GPU job (host-gpu.js gpuTaskWorker), which keeps
   the buffers.

## Painting a shader every frame (samples/gpu/gpu_visualizer.warp)
- host-gpu.js keeps the compiled module and render pipeline per shader text (16, MOST_COMPILED): only the uniform's
  bind group is made per frame.
- WGSL raymarching tip: a distance field over angular sectors (bars whose height differs per band) must take the
  neighbor sectors into account (min over beside = -1, 0, 1), or rays step into a taller neighbor: stair artifacts.
- Natively gpu.warp shows a painting in the window's Metal layer, drawn at its own size and scaled into the window
  whole, its aspect kept, the rest black.
- Beat reaction in the sample: each band against its own fading peak (auto-gain), kick = the summed rise of all bands,
  fading 0.85 per frame; tone = the loudest band, eased. Spectrum of a note is clean (one or two bands), so per-band
  gain doesn't amplify leakage.

## Shader built-ins
$width $height $size $time $frame $mouse $mouse_down $mouse_wheel $key need no variable: extensions/rules/shader_holes.warp BUILTIN_HOLES leaves them
out of the values map unless the program assigns that name; the host fills those the shader reads (`values.<name>`):
natively lib/host/native_host/gpu.warp (from the window's events), in the browser host-gpu.js builtinValues (pointer
and key from canvas.js's shared buffer).
time = seconds since the first render, frame = renders before, key = code point, arrows 0xF700–0xF703, 0 = none.
mouse_wheel = whole wheel steps turned since the start, up adding (the page's canvas.js
counts WheelEvent deltas, 100 px or 3 lines a step, and keeps the wheel off the page while over a canvas); also the
system value mouse_wheel; `$wheel` is its alias; compute_toy's mouse.zoom = 1.05^mouse_wheel, as compute.toys zooms.
Costs nothing for a shader that doesn't mention them.

## $previous: a shader's own last frame
`$previous(at)` is the color the shader left at pixel `at` (vec2f, clamped to the image) in its previous render of
the same size, black before the first (another paper: `select(paper, $previous(at), $frame > 0)`): painting with
memory (samples/graphics/shader_trails.warp). $last, $feedback, $before are not taken from programs; a value missing under
them hints at $previous (host-gpu.js PREVIOUS_WORDS). Not a value but a
function: BUILTIN_HOLES has "previous" so no values entry is made; the host rewrites `values.previous(` to
`warp_previous(` and appends PREVIOUS_FRAME (texture_2d at @group(0) @binding(1), textureLoad). Each render draws
into a new texture and keeps it per shader text (native gpu.warp previous_kept; browser host-gpu.js
previousFrames); a canvas paint then copies it into the canvas texture (configured COPY_DST). Kept as 8 bits per
channel, so small fades round: subtract a step (≥ 1/255) rather than multiply by 0.99, which stalls at ~33/255.

## Several passes (card gpu-multipass)
web-ocean renders its sea in passes (FFT spectra, a height-field texture, then the sea that samples it); one fragment
shader per paint covers only the last pass. The host half is built, as library words in lib/gpu_passes.warp (no new
syntax): `gpu_fields(width, height, names)` keeps a pair of rgba16float targets per name (rgba32float, then
rgba8unorm, where that format cannot be a target), `gpu_pass(shader, fields, into, values)` draws the shader into
that field and swaps the pair, and `gpu_show` / `gpu_picture` draw the shading pass at the fields' size.
`gpu_show_sized` / `gpu_picture_sized` draw that same pass at another size (a window, a picture), the fields still
sampled in 0..1, so a coarse grid can shade at the drawable. `gpu_show_sized` opens the window at the given point
size and shades the drawable itself when that is a larger image of the same aspect. The passes of one frame share
one command encoder and nothing is read back until `gpu_picture`. A shader reads a field as `field_<name>(uv)` with
uv in 0..1, linearly when the format filters. `gpu_fields_format()` says which format. The `render()` / `waves(p.xz)`
spelling below stays a design: it needs the compiler to see an image value, and that is not built.
The plan, in warp's words, with $previous as the model:
- **A pass is a painted function whose image stays on the GPU.** `waves = render(height_field, 256, 256)` (decision
  gpu-multipass: render, alias texture) gives an image value: a texture handle, no readback, never shown. Calling it again each frame renders into the same texture (kept by name like the previous frames).
- **Another painted function reads it** as a function of a position (decision gpu-multipass): `waves(p.xz)` in
  `ocean(x, y)`. The kernel translation sees a program value that is an image (as it sees program vectors today)
  and makes it a texture binding: `@group(0) @binding(2 + k) var warp_image_k: texture_2d<f32>` plus a
  sampler, the call `textureSampleLevel(warp_image_k, warp_sampler, uv / size, 0.0)`; ints sample with textureLoad.
- **The host** generalizes $previous's one binding to a list of textures per painter (native gpu.warp,
  browser host-gpu.js): the painter's layout gets one texture entry per image it reads, the render binds the kept
  textures by name. Rendering order is the program's order of statements, so a pass written before the paint is
  drawn first in the same frame (one command encoder, no CPU wait between passes).
- **Precision**: images as rgba16float (the heights need more than 8 bits; $previous's 8-bit rounding is the warning).
- **On the CPU** an image is the list of its pixels (as paint_each computes them) and `waves(at)` reads the nearest
  pixel, so the fallback paints the same.
- **Compute passes** (an FFT over a spectrum) are maps over lists kept on the GPU (linear arrays), a later
  step; an image built from such a list (`image(list, width, height)`) joins them to the paint.
- First step: render + reading an image in a painted function, native only, one probe rendering a gradient into an
  image and painting it shifted. Then the browser host, then samples/gpu/ocean.warp's waves as a texture.

## WGSL errors at their place in the program
WGSL in `shader { … }` stays as written (no warp syntax converted into it; only `$name` holes). The block's reader
(extensions/shader_blocks.warp shader_place) marks its text with where it stands: the line of its first written line,
the columns before that line and before each later one, which the dedent took off. A paint, gpu_render or gpu_compute
of a name the program assigns a shader block passes shader_words(s, place) (rules in extensions/rules/modules/
prelude.warp, rule word shader_words_of): the GPU compiles the text placed as written, blank lines and spaces before
it, so its own line:column is the program's. Both hosts then say `why at line:column` (host-gpu.js checkedModule,
gpu.warp shader_failure out of naga's report), the runtime adds no call site (error_sentences names_place) and the
playground marks the place (driver.js placeNamed, report.error_at). Columns after a `$name` hole on the same line count
it as `values.name`. Not placed: a shader block given inline (`paint(shader {…})`), shaders from imported modules,
shaders given as quoted text: their place stays the shader's own.

## GLSL (twigl, shadertoy)
Not in the compiler, to keep it lean (naga's GLSL front end alone was a 705 KB glsl.wasm). Commit b04da3e2c has the
last full implementation; a `use twigl` module can come back from there (card twigl-module). Translating GLSL to WGSL
helps no WebGL-only browser: its WGSL still needs WebGPU.

## Paint window, default canvas
- The native window is lib/host/native_host/window.warp over SDL3, in the run's own process; its events feed the
  system values and the shader built-ins. $key holds only the last key pressed.
- Default canvas: lib/draw.warp sizes it from system values view_width / view_height (the playground's output pane
  via playground.js viewSize; natively 640×480). notes/web_playground.md "The default canvas is the output pane".
- Quirks: run paint probes under WARP_WINDOW=0, or windows pop up on the user's Mac; headless, paint writes PNGs
  to $TMPDIR/warp-paint, shared by every session (filter frames by size and mtime). A headless 1080p animation is
  bound by PNG writing; the window path isn't. A shader built-in is skipped when the program assigns a variable of
  that name (gpu_visualizer keeps its own `time`, the song's clock). `sleep 16ms` takes a unit on its number.

## Warp functions on the GPU
Warp functions compile to WGSL (lib/shaders.warp), so the work around a shader (spectrum(), a map's helper
functions) runs there too.

- **The subset that compiles.** It's the kernel translation, widened from one lambda expression to a function body:
  - numbers in f32 (P240: imprecise by design), i32 for ints; vectors of 2–4 numbers as vec2f…vec4f
  - local variables, `if`/`else`, `for i in a..b`, `while`
  - the math words (sin cos tan sqrt exp log pow atan2 min max abs floor fract round clamp)
  - calls of other functions in the subset, each emitted as its own WGSL `fn`
  - Refused, with the reason named: texts, lists of variable length, maps, closures, allocation, host words,
    exceptions, recursion (WGSL has none).
- **Types.** A parameter's type comes from the call site that sends it to the GPU: an f32 item, a program number, a
  pixel coordinate. One WGSL function is made per signature (shares the specialization with card untyped-parameters).
- **Which functions go to the GPU.**
  1. A map kernel may call user functions: `xs.map(x => shade(x))` translates shade() and its callees; the threshold rule stays (heavy kernels auto
     from GPU_AUTO_MIN_COUNT, `@gpu` forces).
  2. `paint(f, width, height)` with a warp function `f(x, y) -> color` instead of a WGSL text: f becomes the fragment
     shader; the program's numbers it reads become `values` (uniforms, as for text shaders).
  3. `@gpu` on a function: every call from a heavy map or a paint function compiles it, and a function outside the
     subset is a warning that names the construct (an error under strict).
- **Precision**: P240 stands, math is exact by default except for mass computation. Small
  computations stay exact in f64; mass work (a heavy map from GPU_AUTO_MIN_COUNT items) goes to the GPU in f32 by
  itself, with the one-time notice. The functions such a map calls go with it, in f32 under the same notice.
  `@gpu` and paint ask for the GPU outright; `@cpu` and paint_on_gpu = no keep f64.
  On the GPU itself precision doesn't matter: every number is an f32, ints and `x as float`
  included, with no effort spent on exactness. Colors are the one exception and stay exact: a packed 0xAARRGGBB int
  in an f32 would garble its channels (alpha pushes it past 2^24). So a function giving rgb(…) (hsv and other callers
  of rgb too) becomes a WGSL fn returning a vec4f, a packed color literal is unpacked at compile time, and any other
  number a painted function gives is ink unless 0, as in paint(pixels, …).
- **Fallback.** As for maps:
  - not translatable: runs on the CPU as written, warned once
  - no adapter: the CPU, silently
  - `@cpu` / paint_on_gpu = no: the CPU in f64
  - the browser runs the same WGSL through host-gpu.js
- **State**: `paint(f, width, height)` compiles f to a fragment shader at run time (lib/shaders.warp, its rules
  extensions/rules/modules/shaders.warp), the program's values f reads as its values; vectors are vec2f…vec4f; no
  adapter or paint_on_gpu = no: f of each pixel on the CPU (lib/shaders.warp paint_each). Map kernels calling user
  functions are card xs-map-gpu.
- **Open**: the acceptance step, samples/gpu/gpu_visualizer.warp's tunnel shader written in warp giving the
  same frame as the WGSL one, needs arrays as values (`levels`).

## Shaders in the browsers' playground
- Linux Firefox keeps WebGPU off and Chrome's SwiftShader drops the page canvas (shaders come back as pixels). The
  Firefox tour turns WebGPU on
  (FIREFOX_PREFS `dom.webgpu.enabled`, `gfx.webgpu.ignore-blocklist`, lavapipe from mesa-vulkan-drivers): the only
  CI browser drawing a shader into the page's canvas, as users' browsers with a GPU do.
- Firefox loses an OffscreenCanvas posted into a MessagePort whose other end is itself still on its way to a Worker
  (only "uncaught exception: undefined", then a hang in Atomics.wait). The page posts the canvas straight to the task
  Worker it made
  (task-workers.js postToTaskWorker, by `taskWorkerId`).
- An `on frame` painting arrives after the run's report (playground.js showEventOutput): paintingOf maps the GPU
  canvas's id to its canvas there too; a pixel frame is redrawn in place.
- web/playground/firefox_driver.mjs takes FIREFOX_PREFS and names an error's line and realm.
- Firefox's "uncaught exception: undefined" during 400 GPU jobs is no WebGPU fault: on lavapipe 400 jobs take over
  10 s, the playground stops the run, and terminating a Worker inside WebAssembly.promising logs it (Firefox's bug,
  card firefox-logs). The tour's check accepts a stop ("or says why not").

## Third-party shaders stay out (license P150)

warp is AGPL-3.0 with commercial dual licensing, so it ships only shaders written for it. compute.toys shaders carry
no default license (all rights with their authors) and Shadertoy's default is CC BY-NC-SA 3.0, neither compatible. The
compute.toys ports (balatro, cyber_fuji, dandelion, fiddling_rings, kuko175_pumpkins, singularity, solar, wormhole)
and the bundled WebGL obj demo moved to github.com/pannous/shaders (warp/compute_toys, webgl/), each headed with title,
author, source and license; they run there with `warp <file>`. Our own compute.toys kernels: samples/gpu/lava_lamp,
northern_lights. The old files remain in git history.

## A window out of sight shows nothing (card show-black-native)
wgpu's Metal surface answers Occluded while NSWindow.occlusionState lacks "visible" (wgpu-hal metal/surface.rs,
workaround for gfx-rs/wgpu#8309), and presented() then skips the frame. A native window opens behind the app in front
(a run from a terminal or an agent is not activated), so a still picture's only present was dropped: black until
closed. gpu.warp keeps the painting it could not show (shown_kept) and shown_again shows it; window.warp's event
loop calls that on SDL_EVENT_WINDOW_EXPOSED. An
animation needs nothing: its next frame shows. SDL_RaiseWindow/SyncWindow don't help (SDL's OCCLUDED flag lags).
Checking a window by eye: capture it by window id (Quartz CGWindowList by the run's pid, screencapture -l), bring it
to the front with System Events `set frontmost of (first process whose unix id is <pid>)`; SIGTERM does not end a
held still picture, kill -9 by pid.

## One shader (card shader-value, plan 2026-11-12)
Today six ways write GPU code: shader blocks with `$holes`, warp functions translated by lib/shaders.warp,
compute_toy kernels, raw gpu_compute, GPU lists, gpu_fields/gpu_pass multipass (and `@gpu` maps, broken since the
Rust emitter left). They all end in one place: WGSL text plus a values map handed to a host. The plan makes that the
one shape: **one Shader value, one hole table, one resource model, two words.**
1. Shader value: every form yields one. Its sources are a warp function (canonical: `paint(julia)`) or a
   `shader { … }` block (the WGSL escape hatch). The block reader detects the dialect by its entry point: `@fragment
   fn main` WGSL, `main_image` compute.toys (compute_toy becomes an identity alias), later `mainImage` Shadertoy.
2. One hole table in lib/shaders.warp: time, frame, width, height, size, mouse, mouse_down, mouse_wheel, key, with
   their aliases (u_time, Time, iTime …) as aliases in the same table. One compile-time scanner; the hosts receive
   the hole list and stop keeping their own (gpu.warp hole_value, host-gpu.js builtinValues, compute_toys Time/Mouse).
   One rule fills the holes of any call whose argument is a Shader, instead of one rule per entry word.
3. Fields are the one resource (Taichi's field, Halide's Func): shape, element type, lifetime. The compiler picks
   buffer or texture from the use (sampled or drawn into → texture; atomics, structs, scatter → buffer) and refuses
   loudly where uses conflict. A list of shaders runs in list order, as a list of notes is a melody:
   `paint([advect, pressure, show])`; the reads/writes graph is only checked (a read nobody wrote this frame or last
   is refused with a fix-it), never used to reorder. Shadertoy's rule: a read sees the latest write earlier in the
   frame, else last frame's value; so `$previous` is just a read-before-write, persistence follows from the dataflow,
   and a pass reading and writing one field gets two copies swapped (never read_write textures, which core WebGPU
   allows only for r32 formats). gpu_fields/gpu_pass/gpu_show go into it. Culling, aliasing, async can come later
   behind the same list. Sources: Frostbite FrameGraph GDC 2017, shadertoy.com/howto, docs.isf.video multipass,
   docs.taichi-lang.org, usegpu.live.
4. Two words: `paint(s)` draws; computing is `xs.map/reduce/scan(f)` (Futhark's forms, neighbours may fuse) on a GPU list or under `@gpu`, through the same translator
   grown to compute kernels (this brings back the broken `@gpu` maps, wiki/graphics.md:95); a raw WGSL kernel runs
   with `run(kernel, lists)`. gpu_compute, gpu_render, gpu_pass, gpu_show become aliases, then leave.
5. The values layout is written down once (here) and both hosts implement it.
Phases, each landing alone: (a) hole table + one scanner + one rule; (b) dialect detection, compute_toy alias;
(c) fields subsume $previous and gpu_passes, paint(list); (d) compute kernels from warp functions, @gpu maps;
(e) remove the duplicates, samples written the canonical way, wiki graphics.md (wiki text to the Interviewer).
Core cost: none in core/; extensions/shader_blocks and the hole rules shrink; lib/shaders.warp grows.
- (a) done: the table is SHADER_HOLES in extensions/rules/shader_holes.warp, not lib/shaders.warp: the block reader
  and the rule both run inside the compiler, which uses no lib module. The block reader keeps no hole list of its own;
  one rule `$f($s, $w, $h, $v...)` for paint and gpu_render fills the holes (gpu_render(s, w, h, values) did not).
  The hosts still list the built-in holes they fill; that list goes when (c) changes what they receive.
- (b) done: a rule reads the dialect off the entry point where a shader the program assigns is painted or rendered:
  `fn main_image` without `@fragment` becomes compute_toy(s); compute_toy of a fragment shader is that shader. The
  block itself stays the kernel as written. compute_toy lives in lib/shaders.warp (card compute-toys-own): no `use`;
  the pictures channel0/channel1 are paint values, `paint(toy, w, h, {channel0: picture})`: compute_toy reads them as
  `values.channel0`, and pictured_values, which the dialect rule wraps around the values, gives each its pixels as a
  GPU list [u32] made once per image (refusing a channel the shader samples that they lack).
- Pictures among paint's values (card gpu-texture-binds): anything with width, height and pixels binds as a
  texture_2d<f32>, bgra8unorm (a 0xAARRGGBB word is B, G, R, A in memory), `values.<name>` renamed warp_picture_<name>,
  from binding 16 on (gpu.warp PICTURE_BINDING, host-gpu.js); the samplers the shader names (nearest, bilinear,
  nearest_repeat, bilinear_repeat, as compute.toys names them) from binding 24 on, trilinear refused (no mipmaps).
  A GPU list [u32] of pixels is written into the texture where it is; any other list is walked, with a slow-path note.
