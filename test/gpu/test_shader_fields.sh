#!/usr/bin/env bash
# paint of a list of shaders: each draws into the field of its name, which a shader reads as $name(at), last frame's
# where nothing wrote it earlier in the frame; the last shows, headless as the pixels paint gives (wiki/graphics.md
# "Fields and passes"; decision shader-value (c)). Paints run outside `warp test`, as test_paint_sizes.sh does.
# Run it with bash test/gpu/test_shader_fields.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/../native/checks.sh"
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT

last_line() { # last_line <name> <code>: the code as a file, run, the last line it printed
	printf '%s\n' "$2" >"$stage/$1.warp"
	"$warp" --mute "$stage/$1.warp" 2>&1 | tail -1
}

check "a field keeps what its shader drew the frame before" "$(last_line kept 'width = 2
height = 1
count = shader {
@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return $count(at.xy) + vec4f(0.25, 0.0, 0.0, 0.0); }
}
show = shader {
@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return vec4f($count(at.xy).r, 0.0, 0.0, 1.0); }
}
paint([count, show])
pixels = paint([count, show])
red = int(pixels#1 / 65536) % 256
print red > 120 and red < 135')" "yes"
check "a pass with values of its own" "$(last_line valued 'width = 2
height = 1
glow = shader {
@fragment fn main() -> @location(0) vec4f { return vec4f($level, 0.0, 0.0, 1.0); }
}
show = shader {
@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return $glow(at.xy); }
}
pixels = paint([glow with {level: 1.0}, show])
print int(pixels#1 / 65536) % 256')" "255"
check "a shader block without an entry line is a fragment body, at its pixel" "$(last_line body 'width = 2
height = 1
level = shader { return vec4f(at.x / 2.0, 0.0, 0.0, 1.0); }
show = shader { return $level(at.xy); }
pixels = paint([level, show])
print int(pixels#2 / 65536) % 256')" "191"
check "a read of a field no shader draws is refused" "$(last_line unwritten 'width = 2
height = 1
show = shader { return $dye(at.xy); }
other = shader { return vec4f(1.0); }
paint([other, show])' | grep -o 'fix: put dye in the list')" "fix: put dye in the list"
# fields declared as values, drawn by program functions: `velocity: advect` draws the field velocity, which a function
# reads at a cell, velocity(x, y) (card fluid-25d-dsl)
check "a declared field keeps what its function drew the frame before" "$(last_line declared 'width = 2
height = 1
count = field(1, 2, 1)
grow(x, y) := count(x, y) + 0.25
show(x, y) := [count(x, y), 0, 0]
paint([count: grow, show])
pixels = paint([count: grow, show])
red = int(pixels#1 / 65536) % 256
print red > 120 and red < 135')" "yes"
check "a function giving another width than its field holds is refused" "$(last_line wide 'width = 2
height = 1
count = field(1, 2, 1)
grow(x, y) := [count(x, y), 1]
show(x, y) := [count(x, y), 0, 0]
paint([count: grow, show])' | grep -o 'gives vec2f, but the field it draws holds f32')" "gives vec2f, but the field it draws holds f32"
check "a function before the last names the field it draws" "$(last_line unnamed 'width = 2
height = 1
count = field(1, 2, 1)
grow(x, y) := count(x, y) + 0.25
show(x, y) := [count(x, y), 0, 0]
paint([grow, show])' | grep -o 'fix: name it')" "fix: name it"
check "a field smaller than the painting is read at its own cells" "$(last_line cells 'width = 8
height = 4
ink = field(1, 4, 2)
drop(x, y) := if x == 1 and y == 0 then 1 else 0.5
show(x, y) := [ink(floor(x / 2), floor(y / 2)), 0, 0]
pixels = paint([ink: drop, show])
print [0, 1, 2, 3, 4, 5, 6, 7].map(i => int(pixels#(i + 1) / 65536) % 256)')" "[128, 128, 255, 255, 128, 128, 128, 128]"
check "lists of functions share the declared fields; one whose last function draws paints nothing" "$(last_line shared 'width = 2
height = 1
count = field(1, 2, 1)
grow(x, y) := count([x, y]) + 0.25
show(x, y) := [count([x, y]), 0, 0]
paint([count: grow])
paint([count: grow])
pixels = paint([show])
red = int(pixels#1 / 65536) % 256
print red > 120 and red < 135')" "yes"
check "a field of four numbers, drawn by a function reading a long list" "$(last_line listed 'width = 2
height = 1
levels = [0.1, 0.2, 0.3, 0.4, 0.5, 0.75]
glow = field(4, 2, 1)
fill(x, y) := [levels#6, 0, 0, 1]
show(x, y) := [glow(x, y).x, 0, 0]
pixels = paint([glow: fill, show])
print int(pixels#1 / 65536) % 256')" "191"
exit $((failures > 0))
