# Shader compiler trap: two value-noise calls

A fragment shader that calls this value-noise twice, then mixes the sum with a sampled field, traps the GPU compiler. The host does not return naga's report. `gpu_pass` raises an uncatchable wasm trap (`wasm unreachable` / `thrown Wasm exception`), so `try`/`catch` does not see it. One call of the same noise, mixed with the same field, compiles. Defining `fbm` (two calls inside the function) and calling it once also compiles. The trap is this expression, not every second call.

Save the program below and run it with `WARP_WINDOW=0 warp --no-window --lib <repo>/lib <file> one` and the same with `two`. `one` prints `one compiled`. `two` traps.

```warp
use gpu_passes
fields = gpu_fields(8, 8, ["density"])
kind = args[0]
head = '
fn hash(p: vec2f) -> f32 { return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453); }
fn noise(p: vec2f) -> f32 {
	let i = floor(p);
	let f = fract(p);
	let u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2f(1.0, 0.0)), u.x), mix(hash(i + vec2f(0.0, 1.0)), hash(i + vec2f(1.0, 1.0)), u.x), u.y);
}
@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
	let uv = at.xy / values.size;
	let n0 = noise(uv * 4.0);
'
one = head + '
	return vec4f(mix(field_density(uv).x, 0.2 + 0.8 * n0, 0.1), 0.0, 0.0, 1.0);
}
'
two = head + '
	let n1 = noise(uv * 9.0 + vec2f(1.7, 3.1));
	let target = 0.05 + 0.72 * n0 + 0.38 * n1;
	return vec4f(mix(field_density(uv).x, target, 0.05), 0.0, 0.0, 1.0);
}
'
gpu_pass(if kind == "two" then two else one, fields, "density", {size: [8, 8]})
kind + " compiled"
```

The same trap fires on the WGSL builtin `mod` (`mod(u, 0.2)` in a fragment that returns that value). A hand-written wrap, `p - period * floor(p / period)`, compiles. The fluid sample bakes its tiling noise with that wrap, one octave per pass, and does not call `noise` twice in one shader.
