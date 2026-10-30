# Shaders written in warp

`paint(f, width, height)` compiles a warp function `f(x, y)` to a fragment shader (lib/prelude.warp's paint rules
call lib/rule_words.warp's wgsl_shader in the compiler; the Rust era's was src/lowering/foreign/gpu_maps/kernels.rs), with the program's numbers it reads as the shader's values; the same
functions run on the CPU where there is no GPU. `shader { … }` blocks of WGSL with `$name` holes stay for hand-written
shaders. `shader_of(f)` is the WGSL that paint compiles f to. Samples: samples/sound/gpu_visualizer_warp.warp (the
tunnel as warp functions, playing mozart.mp3; native only), samples/gpu/warp_compiled_wgsl_shader.warp (three
functions painted as one shader, its WGSL printed), beside the WGSL samples/gpu/gpu_visualizer.warp.

## What kernels compile
- A list of 2–4 numbers is the vector (P241: no vector type): list literals become vecNf, `p#k` components, swizzles
  `.x .xy .zyx` (decision swizzles, S1; kernels.rs swizzle; on the CPU list_ops/swizzles.rs answers right before the
  `no field` error, so a map's or instance's own field `x` wins).
- Vector math uses the element-wise operators `.+ .- .* ./` (D3), a number on either side broadcasts; on the CPU they
  decide at run time where a side may be a list (prelude element_wise_any).
- `use math` words on lists and numbers: fract step smoothstep mix (= lerp) distance normalize cross reflect, `norm` /
  `magnitude` the Euclidean length; libm's one-number words map over lists. `round` takes halves away from zero
  (WGSL's goes to even).
- if/else values, assignments, `for step in 0..110 { … break }`, functions specialized by argument widths, functions
  of no parameters, a 3-list of 0…1 returned as the color.
- A program list read item by item (`levels#k`) reaches the shader as one vec4f per item; program globals are the
  shader's values (no `$` needed).
- A class's operator dispatch (`if x is Note then x.plus(1) else x + 1`) keeps only the built-in operator in kernels.
- `spectrum(song, time, bands)` of a file name is measured by the host (src/sound/spectrum.rs, Goertzel);
  `sound_seconds(song)`. Browser host: card spectrum-song.

## Porting a WebGPU page: web-ocean (samples/gpu/ocean.warp)
github.com/Noniv/web-ocean (MIT; clone in data/web-ocean) is a multi-pass WebGPU renderer (FFT wave cascades in
compute passes, sky LUTs, environment cube map, CDLOD mesh, bloom). paint(f) is one fragment shader, so the port
raymarches a sum of dragged exp(sin − 1) waves under an analytic sky with two noise cloud layers, and ports the
post chain (white balance, highlight bleach, AgX + look) as is; card gpu-multipass for the real pipeline.
- Settings and camera are program numbers (globals the shader reads); program *lists* don't reach kernels as vectors
  yet (card kernel-global-lists): constant colors and the sun direction are zero-parameter functions.
- shader_of names the form it refuses and the function it is in; a paint function must give a 3-list (a 4-list as color is refused too).
- The painted functions only work on the GPU: their CPU fallback (WARP_GPU=off) fails on vector parameters
  (card vector-params-cpu); vector math must use the dotted operators, `p - 289 * floor(p / 289)` types p as a number.
- Original's visual check without a local browser: probes/ocean/screenshot_on_server.sh (agent-browser + SwiftShader on
  pannous.com) — it runs (906 CDLOD nodes, 60 fps) but SwiftShader gives a white frame; compare with its docs/*.jpg.

## Writing rule words (lib/rule_words.warp)
The translator runs inside the compiler as rule words, which the seed compiles; there these refuse or misbuild:
- no `global`: keep state in a main-level class instance with mutable fields (`shading`)
- no `: nothing` result type on a procedure (`name(...) := {` instead), else "an assignment to `… : …`"
- calls only inside `${…}` are not reached ("the call … (no owner)", card hole-calls-unreached); `${#xs}` refuses
  (card count-in-interpolation): compute into a local first
- a line starting with `[` or `(` glues to the statement before it as indexing or a call: `return [...]`
- locals must not shadow a core function or global name; map keys that are operator words (ln, sqrt, π) are quoted
- a lib module used by a std module is followed only if rewrites.warp's std_module_names lists it (card
  nested-module-use), so the translator lives in rule_words.warp itself
- `use math` brings lib's own `cosine` etc. into the program: WGSL's words are looked up before program functions
