# WGSL translator robustness (card shader-port, 2026-10-30)

Well-known fragment shaders ported to plain warp paint functions, as a sample author writes them
(samples/gpu/ocean.warp is the model), painted headless on the GPU and on the CPU (`WARP_GPU=off`), the two PNGs
compared. Ports of permissive sources (own takes on public algorithms, Inigo Quilez's MIT article code) are kept in
samples/gpu; Shadertoy ports (CC BY-NC-SA) live outside the repo in ~/dev/angles/shaders/warp/ports
(samples/shaders links there). Outcome test: test/gpu/test_gpu_matches_cpu.sh (GPU pixels == CPU pixels, or refused
by name).

## Results (translator of origin/main before the card)
| port | GPU | CPU | verdict |
|---|---|---|---|
| mandelbrot, julia | painted | julia flat, mandelbrot never ends | julia: `length(z)` is WGSL's magnitude but warp's count (wrong); mandelbrot: exact fractions on the CPU (card mandelbrot-cpu-hang) |
| plasma, palette (iq cosine palette), hexgrid, sphere (Phong) | painted | same pixels | works |
| fbm + domain warp (iq), voronoi (Worley), truchet, stars | painted | trap | `fract` of a list traps in lib/math on the CPU (card fract-cpu-trap) |
| SDF raymarch (iq distfunctions, soft shadow) | painted | "no field y" | `[…] - direction.y` was plain `-` on a vector: warp refuses it, the GPU subtracted element-wise (wrong) |
| shapes2d (iq 2D SDFs) | painted | flat | `length(p)` again (wrong) |
| tunnel (checkerboard) | painted, half black | correct | `%` was WGSL's truncating %, warp's is Euclidean (wrong) |
| creation (Silexars), auroras, protean clouds (nimitz) | painted | – | work; Shadertoy licenses, kept outside the repo |
| protean clouds, first take | shader invalid | – | a local named `target`: WGSL reserves ~150 words the translator didn't rename (crash) |

Gaps found with small probes (data/wgsl/gaps in the worktree), each refused before:
`x %= 3`, `x ^= 2`, `x max= 5` (op= the parser leaves as written); `p.z = 1`, `p.x += 1`, `p.xy = q`; `c ? a : b`;
`while yes`; `continue`; `x /! 8`, `x rem 3`; named arguments `g(b: y, a: x)`; default parameters `g(a, b = 2)`;
a local lambda (now refused by name: WGSL has no closures).

## Fixed in lib/wgsl.warp (general rules)
- OPERATOR_MEANINGS: warp's operators WGSL has with another meaning, as WGSL expressions: `%` Euclidean, `rem`
  truncated, `/!` floored. `%` is no longer passed through.
- Compound assignment is `x = x op v` for every operator and function word (decision op-assign-general), the parser's
  `+= -= *= /=` and the ones it leaves as written alike; `.op=` element-wise.
- Assignment targets: a variable or components of one; a swizzle is assigned one component at a time in a block of
  its own (WGSL assigns no swizzle).
- `c ? a : b` is `if c then a else b` (select); yes/no are true/false; `continue`.
- A call of a program function places named arguments by parameter and fills defaults.
- WGSL's reserved words (spec list) join WGSL_TAKEN, so `target`, `filter`, `mod` … are renamed.
- Refuse, never guess: on a vector, `+ - * /` (warp's + joins lists, - and * are refused by warp), negation, and
  `length`/`count`/`size` (warp counts; the magnitude is `norm`) are refused with the element-wise fix-it.

## Not translator (cards)
- rule-fn-in-sequence (is-function-miss): a function defined in a `;`-joined line of a file isn't `is function` to the
  rules, so shader_of fails ("text of node") and paint skips the shader.
- fract-cpu-trap, mandelbrot-cpu-hang (above). `exp2` is a WGSL word the warp side lacks (`2 ^ x` works on both).
- A local named `check` reads as an assertion (`check = …`), not a variable.
