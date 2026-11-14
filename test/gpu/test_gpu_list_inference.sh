#!/usr/bin/env bash
# A list made for the GPU alone is made as a GPU list of the cells its items take (card gpu-list-inference, decision
# gpu-bound-floats): four numbers each [vec4], numbers [f32], so paint's upload copies it once and `warp --strict` (no
# slow path) runs it; a GPU list declared so keeps its cells in every later assignment. The warp under test is WARP
# (default the one on PATH), its lib LIB
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
failures=0
check() { if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi; }
strict() { WARP_WINDOW=0 "$warp" --strict --mute --lib "$lib" "$1" 2>&1 | tail -1; }
shader='s = "@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return values.xs[1]; }"'
check "a list of four numbers each, made for the GPU alone, is a GPU list [vec4]" "$(strict "xs = [[0.0, 0.0, 1.0, 1.0] for i in 1 to 4]
$shader
print gpu_render(s, 2, 2, {xs: xs})#1")" 4278190335
check "a list another name holds too stays a list: the slow path says so" "$(strict "xs = [[0.0, 0.0, 1.0, 1.0] for i in 1 to 4]
$shader
gpu_render(s, 2, 2, {xs: xs})
ys = xs" | grep -c 'slow path')" 1
check "a declared GPU list keeps its cells when assigned again" "$(strict "global xs: [vec4] = [[1.0, 0.0, 0.0, 1.0]]
paint_blue() := { xs = [[0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 1.0, 1.0]] }
paint_blue()
$shader
print gpu_render(s, 2, 2, {xs: xs})#1")" 4278190335
# paint's first argument is pixels the CPU paints: a list painted so stays a list (samples/advanced/filled_circle.warp)
check "a list of pixels painted stays a list" "$(WARP_WINDOW=0 "$warp" --mute --lib "$lib" 'surface = int[6]; paint(surface, 3, 2); print gpu_list_place(surface)' 2>&1 | tail -1)" -1
# a list whose items the program reads too would show them rounded to f32: it stays a list, the slow path says so
check "a list the program reads stays a list" "$(strict "xs = []
for i in 1 to 4 { xs = xs + [[0.0, 0.0, 1.0 / 3, 1.0]] }
$shader
gpu_render(s, 2, 2, {xs: xs})
print xs#1" | grep -c 'slow path')" 1
check "a list grown from [] for the GPU alone is a GPU list" "$(strict "xs = []
for i in 1 to 4 { xs = xs + [[0.0, 0.0, 1.0, 1.0]] }
$shader
print gpu_render(s, 2, 2, {xs: xs})#1")" 4278190335
# paint's pixels the CPU reads are read from a GPU list in place too
check "a GPU list of pixels paints" "$(WARP_WINDOW=0 "$warp" --mute --lib "$lib" 's: [u32] = int[6]; paint(s, 3, 2)' 2>&1 | grep -c '^painted 3×2')" 1
check "a list joined of made lists for the GPU alone is a GPU list" "$(strict "plain = [[0.0, 0.0, 1.0, 1.0]]
xs = plain + [[0.0, 0.0, 1.0, 1.0] for i in 1 to 3]
$shader
print gpu_render(s, 2, 2, {xs: xs})#1")" 4278190335
check "a list a function hands to the GPU alone is a GPU list" "$(strict "$shader
shown(xs) := gpu_render(s, 2, 2, {xs: xs})
xs = [[0.0, 0.0, 1.0, 1.0] for i in 1 to 4]
print count(xs)
print shown(xs)#1")" 4278190335
[ "$failures" = 0 ] || exit 1
