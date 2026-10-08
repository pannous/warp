#!/bin/bash
# card gpu-vectors step 2 (notes/gpu.md): milliseconds of gpu_compute with a shader that does nothing (the transfer),
# with one doubling every number (transfer + compute), and of the CPU's sum(xs .* 2), for growing n.
# usage: probes/webgpu/threshold.sh [warp binary] [n…]   (a release build: cargo build --release --features native)
WARP=${1:-warp}
shift
COUNTS=${*:-10000 100000 1000000}
SHADER_HEAD='@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size(256) fn main(@builtin(global_invocation_id) id: vec3<u32>) {'

for n in $COUNTS; do
	workgroups=$(((n + 255) / 256))
	"$WARP" eval "double = \"$SHADER_HEAD if (id.x < arrayLength(&data)) { data[id.x] = data[id.x] * 2.0; } }\"
idle = \"$SHADER_HEAD if (id.x == 0xffffffffu) { data[0] = 0.0; } }\"
xs = (1 to $n).map(i => float(i) / 4)
gpu_compute(idle, [1.0], 1)
t0 = clock(); a = gpu_compute(idle, xs, $workgroups); t1 = clock(); b = gpu_compute(double, xs, $workgroups); t2 = clock(); c = sum(xs .* 2); t3 = clock()
{n: $n, transfer: t1 - t0, transfer_and_double: t2 - t1, cpu_double_and_sum: t3 - t2}"
done

# `ys = xs.map(f) @gpu` over a linear float array (src/lowering/gpu_maps.rs) against the CPU's map of the same lambda:
# a light one (the CPU's f64x2 kernel) and a heavy one (sin, cos: the CPU's generic map)
for n in $COUNTS; do
	"$WARP" eval "linear xs = float[$n]
for i in 1 to $n { xs#i = float(i) / 4 }
warm = xs.map(x => x + 1) @gpu
t0 = clock(); a = xs.map(x => x * 2 + 1) @gpu; t1 = clock(); b = xs.map(x => x * 2 + 1); t2 = clock()
c = xs.map(x => sin(x) * cos(x) + √x) @gpu; t3 = clock(); d = xs.map(x => sin(x) * cos(x) + √x); t4 = clock()
{n: $n, gpu_light: t1 - t0, cpu_light: t2 - t1, gpu_heavy: t3 - t2, cpu_heavy: t4 - t3}" 2>&1 | grep -v -e hint -e 'the compiler picks'
done
