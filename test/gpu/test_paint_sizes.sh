#!/usr/bin/env bash
# paint(s) and paint(s, values) paint at the program's own width and height where it assigns both, else at the view's
# size; paint(shader, width, height) of a shader without holes paints it (wiki/graphics.md, cards paint-shader-dimensions,
# paint-shader-values). Run it with bash test/gpu/test_paint_sizes.sh; the warp under test is WARP (default the warp on
# PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0

check() { # check <what> <got> <expected>
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
sizes() { # sizes <name> <code>: the code as a file, run, the sizes it painted, one a line
	printf '%s\n' "$2" >"$stage/$1.warp"
	"$warp" "$stage/$1.warp" 2>&1 | sed -n 's/^painted \([0-9]*×[0-9]*\):.*/\1/p' | tr '\n' ' '
}
glow='glow = shader {
@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return vec4f(at.x / $width, $level, 0.5, 1.0); }
}'

check "paint(shader) and paint(shader, values) at the program's size" "$(sizes sized "use graphics
width = 160
height = 90
level = 0.25
$glow
paint(glow)
paint(glow, {level: 0.75})")" "160×90 160×90 "
check "paint(shader, values) at the view's size" "$(sizes unsized "use graphics
$glow
paint(glow, {level: 0.75})")" "$("$warp" 'print "${view_width()}×${view_height()}"' 2>&1 | head -1) "
check "paint(shader, width, height) of a shader without holes" "$(sizes plain 'use graphics
plain = shader {
@fragment fn main() -> @location(0) vec4f { return vec4f(0.3, 0.2, 0.5, 1.0); }
}
paint(plain, 16, 9)')" "16×9 "
check "paint(f) of a function at the program's size" "$(sizes function 'width = 12
height = 6
shade(x, y) := [x / width, y / height, 0.5]
paint(shade)')" "12×6 "
exit $((failures > 0))
