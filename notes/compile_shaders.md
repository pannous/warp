# Shaders written in warp (card compile-shaders)

User: "compile shaders from warp e.g. gpu_visualizer.warp". Today the tunnel is a `shader { … }` block of WGSL with
`$name` holes. `paint(f, width, height)` already compiles a warp function `f(x, y)` to a fragment shader
(src/lowering/foreign/gpu_maps/kernels.rs), with the program's numbers it reads as the shader's values. Goal: the tunnel
as warp functions, `paint(tunnel, width, height)` compiling them; the same functions run on the CPU where there is no GPU.

## What the tunnel needs, and what warp has

| WGSL in the tunnel | warp today | proposal |
|---|---|---|
| `vec2f/vec3f/vec4f(…)` | lists (P241: no vector type) | a list of 2–4 numbers is the vector: `[c * p.x - s * p.y, s * p.x + c * p.y]` |
| `p.x`, `p.xy`, `e.xyy` swizzles | the same (S1): components of a list of 2–4 numbers | WGSL's swizzle (step 4) |
| `a + b`, `2.0 * v`, `v * w` on vectors | `+` concatenates, `.+ .*` are element-wise (D3); `xs .* 2` works, `2 .* xs` errors | `.+ .- .* ./` with a number on either side broadcasts (build); plain `+` on vectors: **question S2** |
| `length dot` | exist on lists | WGSL mapping in the kernel |
| `normalize cross reflect distance fract mix smoothstep step` | missing (`lerp clamp sign` in math) | add to `use math` for lists and numbers, `mix` an alias of `lerp`; WGSL mapping (build) |
| `sin cos atan2 exp pow abs floor round min max` | exist for numbers | element-wise on lists, as WGSL does |
| `select(f, t, c)` | `if c then t else f` | kernels already compile if/else values |
| `var x = …; x += …` | assignments | kernels support them for f32; vectors with S1/S2 |
| `for (var i = 0; i < 110; i++) { … break; }` | `for step in 0..110 { … break }` | `break` in kernels (build) |
| `const BANDS = 16.0` | a program global | read as a value already; a constant global could be inlined |
| `$levels[i / 4][i % 4]` (16 floats packed in vec4s) | `levels[band]`, a global list | the kernel passes a global list of numbers as an array uniform (build) |
| `$time $bass $kick …` | program globals | already the shader's values: no `$` needed |
| `@builtin(position) at`, `$size` | `f(x, y)` gets the pixel's column and row; `width`, `height` globals | as now |
| `return vec4f(color, 1.0)` | `rgb(r, g, b)` or a packed color | returning a 3-list of 0…1 is the color (build) |

## Order
1. DONE (branch compile-shaders): math words on lists and numbers (CPU): `.+ .- .* ./` decide at run time where a side
   may be a list (prelude element_wise_any), `norm`/`magnitude` the Euclidean length (S3), libm's one-number words map
   over lists, `use math` has fract step smoothstep mix(=lerp, lists too) distance normalize cross reflect.
2. DONE: kernel vector values (list literals vecNf, element-wise ops, `p#k` components), the math words, `break`,
   functions specialized by argument widths, a 3-list as the color; a program list read item by item (`levels#k`)
   reaches the shader as one vec4f per item (`[[item] for item in levels]` in the values map, any length), `center()`
   calls a function of no parameters, `round` takes halves away from zero (WGSL's goes to even), a class's operator
   dispatch (`if x is Note then x.plus(1) else x + 1`) keeps only the built-in operator.
3. DONE: samples/gpu_visualizer_warp.warp, the tunnel as warp functions painted with paint(tunnel, width, height),
   beside the WGSL samples/gpu_visualizer.warp. It needed card note-dispatch-element-wise: Note's plus/minus dispatch
   had hidden `.+ .-` from broadcasting (p .+ [1, 2] gave [3 4]).
4. DONE (branch swizzles): `.x .y .z .w`, `.xy .zyx .xyy` (S1). Kernels: `Key(p, Dot, xy)` → `{p}.xy` of its width
   (kernels.rs swizzle). CPU: library_words reads `p.xy` of a list as a field (it was "undefined function: x"), the
   analyzer types a multi-letter swizzle of a list as a list, and the field chain (emit_generic_indexed_node) asks the
   runtime function swizzled (list_ops/swizzles.rs) right before the `no field` error, so a map's entry `x` (`{x:1}`,
   `[x:1, y:2]`), an instance's field and a tagged object's field all answer first. Not a vector (5 items, bools, a
   component beyond it): `no field x`. Names: words::swizzle_components; probe probes/swizzles.warp.

## Questions (warp-interviewer)
- S1 DECIDED (user, 2026-10-15): `.x .y .z .w` and swizzles (`.xy`, `.xyy`, `.zyx`) on lists of 2–4 numbers,
  everywhere, the CPU too. Built in step 4.
- S2 vector math with plain `+ - * /` inside functions compiled to shaders, or always the element-wise `.+ .*` (D3)?
