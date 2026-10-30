#!/usr/bin/env bash
# The GPU paints what the CPU paints (card shader-port, notes/wgsl_robustness.md): each program paints a warp function
# headless twice, as its WGSL shader and, with WARP_GPU=off, as warp code on the CPU, and the two pictures agree pixel
# by pixel (within f32 rounding). A form the shader can't say is refused by name, so the GPU run paints nothing and the
# case fails. Run it with bash test/gpu/test_gpu_matches_cpu.sh [name…]; the warp under test is WARP (default the warp
# on PATH), its compiler from WARP_LIB as usual. Needs a GPU (wgpu) and python3.
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
# the most a channel (of 255) may differ, and the share of pixels allowed to: f32 against f64 at an edge
TOLERANCE=3
OFF_SHARE=0.02
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
failures=0
only=" $* "

# the pixels of two PNGs (8-bit RGB or RGBA, as the native warp writes them) compared: "same" or what differs
compared() {
	python3 -I - "$1" "$2" "$TOLERANCE" "$OFF_SHARE" <<'PYTHON'
import sys, zlib, struct
def pixels(path):
    data = open(path, "rb").read()
    width, height, depth, kind = struct.unpack(">IIBB", data[16:26])
    channels = {2: 3, 6: 4}[kind]
    at, packed = 8, b""
    while at < len(data):
        length, name = struct.unpack(">I4s", data[at:at + 8])
        if name == b"IDAT": packed += data[at + 8:at + 8 + length]
        at += 12 + length
    raw, stride, rows, previous = zlib.decompress(packed), width * channels, [], bytearray(width * channels)
    for y in range(height):
        kind_of_filter, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up, corner = previous[i], previous[i - channels] if i >= channels else 0
            if kind_of_filter == 1: line[i] = (line[i] + left) & 255
            elif kind_of_filter == 2: line[i] = (line[i] + up) & 255
            elif kind_of_filter == 3: line[i] = (line[i] + (left + up) // 2) & 255
            elif kind_of_filter == 4:
                p = left + up - corner
                nearest = min((abs(p - left), 0, left), (abs(p - up), 1, up), (abs(p - corner), 2, corner))[2]
                line[i] = (line[i] + nearest) & 255
        rows.append([tuple(line[x * channels:x * channels + 3]) for x in range(width)])
        previous = line
    return width, height, rows
gpu, cpu, tolerance, share = pixels(sys.argv[1]), pixels(sys.argv[2]), int(sys.argv[3]), float(sys.argv[4])
if gpu[:2] != cpu[:2]: print(f"sizes {gpu[:2]} and {cpu[:2]}"); sys.exit()
off = [(x, y, a, b) for y, (ga, ca) in enumerate(zip(gpu[2], cpu[2])) for x, (a, b) in enumerate(zip(ga, ca))
       if max(abs(i - j) for i, j in zip(a, b)) > tolerance]
print("same" if len(off) <= share * gpu[0] * gpu[1] else f"{len(off)} pixels differ, first at {off[0][:2]}: gpu {off[0][2]} cpu {off[0][3]}")
PYTHON
}
painted() { # painted <folder> <file> [env…]: the PNG the program paints, "" when it paints none
	mkdir -p "$1"
	env TMPDIR="$1/" "${@:3}" "$warp" "$2" >"$1/said" 2>&1
	ls "$1"/warp-paint/paint.png 2>/dev/null
}
check() { # check <name> <code painting f>
	case $only in "  ") ;; *" $1 "*) ;; *) return ;; esac
	printf '%s\n' "$2" >"$stage/$1.warp"
	gpu=$(painted "$stage/$1-gpu" "$stage/$1.warp")
	cpu=$(painted "$stage/$1-cpu" "$stage/$1.warp" WARP_GPU=off)
	if [ -z "$gpu" ] || [ -z "$cpu" ]; then
		echo "✗ $1: painted nothing: $(grep -v '^hint' "$stage/$1-$([ -z "$gpu" ] && echo gpu || echo cpu)/said" | head -3)"
		failures=$((failures + 1))
		return
	fi
	got=$(compared "$gpu" "$cpu")
	if [ "$got" = same ]; then echo "✓ $1"; else echo "✗ $1: $got"; failures=$((failures + 1)); fi
}
refused() { # refused <name> <code painting f> <words the refusal says>
	case $only in "  ") ;; *" $1 "*) ;; *) return ;; esac
	printf '%s\n' "$2" >"$stage/$1.warp"
	gpu=$(painted "$stage/$1-gpu" "$stage/$1.warp")
	if [ -z "$gpu" ] && grep -qF "$3" "$stage/$1-gpu/said"; then echo "✓ $1 refused"; return; fi
	echo "✗ $1: not refused with \"$3\": $(grep -v '^hint' "$stage/$1-gpu/said" | head -2)"
	failures=$((failures + 1))
}

# warp's operators as warp means them: % never negative, /! the floored quotient, rem truncated
check euclidean_remainder 'f(x, y) := [((x - 8) % 3) / 3, ((y - 8) % 2.5) / 2.5, 0.5]
paint(f, 16, 16)'
check floored_quotient 'f(x, y) := [((x - 8) /! 3 + 3) / 6, ((y - 8) rem 3 + 3) / 6, 0.5]
paint(f, 16, 16)'
# x op= v is x = x op v for every operator and function word (decision op-assign-general)
check compound_assignments 'f(x, y) := {
	a = x
	a %= 5
	b = y / 16
	b ^= 2
	c = x / 16
	c max= 0.5
	[a / 5, b, c]
}
paint(f, 16, 16)'
# c ? a : b, yes and no, continue
check conditions 'f(x, y) := {
	s = 0
	i = 0
	while yes {
		i += 1
		if i > 10 { break }
		if i % 2 == 0 { continue }
		s += i
	}
	x > y ? [s / 25, 0, 0] : [0, 0, 1]
}
paint(f, 16, 16)'
# named arguments and a default
check arguments 'blend(a, b, weight = 0.25) := a * (1 - weight) + b * weight
f(x, y) := [blend(x / 16, y / 16), blend(b: 1, a: x / 16), blend(0, 1, weight: y / 16)]
paint(f, 16, 16)'
# a name WGSL reserves
check reserved_names 'target(x) := x / 16
f(x, y) := {
	filter = target(y)
	[target(x), filter, 0.5]
}
paint(f, 16, 16)'
# what warp means otherwise on lists is refused, never painted as WGSL means it: + joins lists, length counts
refused vector_plus 'f(x, y) := [x, y, 0] + [0, 0, 1]
paint(f, 16, 16)' 'element-wise math is .+'
refused vector_length 'f(x, y) := [length([x, y]) / 16, 0, 0]
paint(f, 16, 16)' 'magnitude is norm(v)'
exit $((failures > 0))
