#!/bin/bash
# scripts/profile.sh [-s <seconds>] [-n <top>] <warp words…>: where one warp run spends its time, by wasm function,
# each function's own time (its callees' not counted), most first (notes/compile_speed.md "Profiling").
# The run gets WARP_PROFILE=perfmap, so the native driver writes each compiled function's address range to
# /tmp/perf-<pid>.map (core/native/driver.warp warp_config); macOS's sample takes the main thread's stacks for <seconds>
# (else until it ends; asked every millisecond, it took one each ~1.4 ms on a compile), each frame named through
# that map: a table of own time, one with the callees. The raw sample lands in data/profile/<pid>.sample; `-r <pid>`
# reports that one again; `-f <function>` counts only the stacks through that function and names its callers (e.g. typed_wasm_module, the
# emit). Example: scripts/profile.sh -s 60 rebuild pure
# -a <pid> -o <dir>: attach to a running warp of WARP_PROFILE=perfmap until it ends (scripts/build-slot.sh does, for
# every build): <dir>/profile.txt the report, sample.gz the raw sample, perf.map the function map; no output.
# Its cost (notes/compile_speed.md): no CPU, ~10-15% wall at 1 ms; ATTACHED_SAMPLE_MS keeps a long build's cheaper.
set -euo pipefail
ATTACHED_SAMPLE_MS=10
seconds=3600
top=40
reported= focus= attached= out=
while getopts "s:n:r:f:a:o:" option; do
	case $option in
		s) seconds=$OPTARG ;;
		n) top=$OPTARG ;;
		r) reported=$OPTARG ;;
		f) focus=$OPTARG ;;
		a) attached=$OPTARG ;;
		o) out=$OPTARG ;;
		*) sed -n '2,12p' "$0" >&2; exit 2 ;;
	esac
done
shift $((OPTIND - 1))
checkout=$(cd "$(dirname "$0")/.." && pwd)
samples_kept=$checkout/data/profile
if [ -n "$attached" ]; then
	[ -n "$out" ] || { sed -n '2,12p' "$0" >&2; exit 2; }
	mkdir -p "$out"
	run=$attached samples=$out/sample perf_map=$out/perf.map
	/usr/bin/sample "$run" "$seconds" $ATTACHED_SAMPLE_MS -mayDie -file "$samples" >/dev/null 2>&1 || true
	cp "/tmp/perf-$run.map" "$perf_map" 2>/dev/null || { echo "profile.sh: no /tmp/perf-$run.map (not WARP_PROFILE=perfmap?)" >"$out/profile.txt"; exit 0; }
	[ -s "$samples" ] || { echo "profile.sh: sample took no stacks of $run" >"$out/profile.txt"; exit 0; }
	exec >"$out/profile.txt"
	trap 'gzip -f "$samples"' EXIT
else
	mkdir -p "$samples_kept"
	run=$reported
	if [ -z "$run" ]; then
		[ $# -gt 0 ] || { sed -n '2,12p' "$0" >&2; exit 2; }
		WARP_PROFILE=perfmap WARP_NO_WINDOW=1 "${WARP_BINARY:-warp}" "$@" >/dev/null &
		run=$!
		/usr/bin/sample "$run" "$seconds" 1 -mayDie -file "$samples_kept/$run.sample" >/dev/null 2>&1 || true
		kill "$run" 2>/dev/null || true
		wait "$run" 2>/dev/null || true
	fi
	samples=$samples_kept/$run.sample perf_map=/tmp/perf-$run.map
fi
python3 - "$samples" "$perf_map" "$top" "$focus" <<'PYTHON'
import bisect, re, sys
samples, perf_map, top, focus = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
starts, ends, names = [], [], []
for line in open(perf_map):
	start, size, name = line.rstrip("\n").split(" ", 2)
	starts.append(int(start, 16)); ends.append(int(start, 16) + int(size, 16)); names.append(name)
order = sorted(range(len(starts)), key=starts.__getitem__)
starts, ends, names = [starts[i] for i in order], [ends[i] for i in order], [names[i] for i in order]
def named(frame):
	address = re.search(r"\[0x([0-9a-f]+)", frame)
	if address and "???" in frame:
		at = int(address.group(1), 16)
		i = bisect.bisect_right(starts, at) - 1
		if i >= 0 and at < ends[i]: return names[i]
	symbol = re.match(r"\s*(.+?)\s+\(in ", frame)
	return symbol.group(1) if symbol else frame.strip()
# the call graph of the main thread (the run's; the others wait for work): a frame's depth is where its count starts,
# its own samples are its count less its children's
frames = []
in_graph = in_main = False
for line in open(samples):
	if line.startswith("Call graph:"): in_graph = True; continue
	if in_graph and not line.strip(): break
	found = re.match(r"([\s+!:|]*)(\d+) (.*)", line) if in_graph else None
	if not found: continue
	if len(found.group(1)) <= 4: in_main = "main-thread" in found.group(3); continue
	if in_main: frames.append((len(found.group(1)), int(found.group(2)), found.group(3)))
own, within, called_from = {}, {}, {}
callers = []
for k, (depth, count, frame) in enumerate(frames):
	children = 0
	child_depth = frames[k + 1][0] if k + 1 < len(frames) else 0
	# (by index: a slice of the frames after each one copied them all, quadratic on a long run's sample)
	for later in range(k + 1, len(frames)):
		later_depth, later_count, _ = frames[later]
		if later_depth <= depth: break
		if later_depth == child_depth: children += later_count
	name = named(frame)
	while callers and callers[-1][0] >= depth: callers.pop()
	focused = not focus or name == focus or any(caller == focus for _, caller in callers)
	if focused: own[name] = own.get(name, 0) + count - children
	# with its callees: each stack counted once for a function, however often it recurses in it
	if focused and all(caller != name for _, caller in callers): within[name] = within.get(name, 0) + count
	# the warp functions calling the focused one, two deep, at its outermost calls
	if name == focus and all(caller != name for _, caller in callers):
		named_callers = [caller for _, caller in callers if not caller.startswith(("_R", "wasm[", "closure_"))][-2:]
		chain = " < ".join(reversed(named_callers))
		called_from[chain] = called_from.get(chain, 0) + count
	callers.append((depth, name))
total = sum(own.values()) or 1
def table(title, counts):
	print(title)
	for name, count in sorted(counts.items(), key=lambda item: -item[1])[:top]:
		print(f"{count:8d} {100 * count / total:5.1f}%  {name}")
table(f"{total} samples of the main thread{' through ' + focus if focus else ''}; each function's own:", own)
table("\nwith its callees:", within)
if focus: table(f"\ncalled from (the two warp functions above {focus}):", called_from)
PYTHON
