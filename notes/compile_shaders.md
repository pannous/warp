# Shaders written in warp

`paint(f, width, height)` compiles a warp function `f(x, y)` to a fragment shader
(src/lowering/foreign/gpu_maps/kernels.rs), with the program's numbers it reads as the shader's values; the same
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
