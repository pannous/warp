#!/bin/bash
# The first commit at which a probe's output went wrong, by git bisect in a worktree of its own (removed at the end).
# usage: scripts/bisect-lane.sh [--lane pure|self|both] [--all] <probe.warp> <expected text, or a file of it> [good] [bad]
#   good  default the newest good/* tag (scripts/gate-probes.sh --tag-good sets them); bad default origin/main
#   --lane  which lane's output is judged (default both: wrong in either is bad)
#   --all   bisect into merged branches too (default --first-parent: main's own commits and merges, the units it bounces)
# Each step: a fresh module cache, `warp rebuild pure` and (lane self or both) `warp rebuild self`, the fixpoint promotion
# of warp.wasm to warp-self.wasm, with the shared binary
# (/workspace/cow/warp-rust-latest, or WARP_GATE_BINARY) and WARP_LIB=<step tree>/lib, then the probe run inside the tree
# (data/bisect-probe.warp, so `use` finds the tree's lib/). Wrong output marks the step bad; a failed rebuild, a probe
# that times out or a step that cannot run is skipped (exit 125), never bad; with --lane both a step whose warp.wasm is
# at no fixpoint (no warp-self.wasm) is judged by its pure lane alone. Trace: <repo>/data/bisect-lane/trace.log.
# Heavy (two rebuilds per step, ~10 min each on the box): run it through /workspace/box-slot.sh.
lane=both first_parent=--first-parent args=()
while (($#)); do
	case $1 in
		--lane) lane=$2; shift 2 ;;
		--all) first_parent=; shift ;;
		-h|--help) sed -n '2,12p' "$0"; exit 0 ;;
		*) args+=("$1"); shift ;;
	esac
done
probe=${args[0]} expected=${args[1]}
[[ -f $probe && -n $expected ]] || { sed -n '3,3p' "$0" >&2; exit 2; }
[[ -f $expected ]] && expected=$(cat "$expected")
repo=$(git -C "$(dirname "$0")" rev-parse --show-toplevel) || exit 2
git -C "$repo" fetch -q origin main --tags || exit 1
good=${args[2]:-$(git -C "$repo" tag -l 'good/*' --sort=-creatordate | head -1)}
bad=${args[3]:-origin/main}
[[ -n $good ]] || { echo "bisect-lane: no good/* tag yet: name a good commit" >&2; exit 2; }
binary=${WARP_GATE_BINARY:-/workspace/cow/warp-rust-latest}
out=$repo/data/bisect-lane; mkdir -p "$out"
wt=$(mktemp -d "${TMPDIR:-/tmp}/bisect-lane.XXXXXX") && rmdir "$wt"
git -C "$repo" worktree add -q --detach "$wt" "$bad" || exit 1
cleanup() { git -C "$wt" bisect reset -q > /dev/null 2>&1; git -C "$repo" worktree remove --force "$wt"; }
trap cleanup EXIT
cp -L "$binary" "$out/warp" || exit 1
cp "$probe" "$out/probe.warp" && printf '%s' "$expected" > "$out/expected"
# the step, run by git bisect inside the worktree
cat > "$out/step.sh" <<STEP
#!/bin/bash
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=\$PWD/lib WARP_MODULE_CACHE=\$(mktemp -d) WARP_GATE_PROBE=gate-ok
trap 'rm -rf "\$WARP_MODULE_CACHE"' EXIT
commit=\$(git rev-parse --short HEAD)
"$out/warp" rebuild pure > "$out/rebuild.log" 2>&1 || { echo "\$commit skip: rebuild pure failed" >> "$out/trace.log"; exit 125; }
lanes=
[[ $lane == self ]] || lanes=pure
if [[ $lane != pure ]]; then
	if "$out/warp" rebuild self >> "$out/rebuild.log" 2>&1; then lanes="\$lanes self"
	elif [[ $lane == self ]]; then echo "\$commit skip: rebuild self (fixpoint promotion) failed" >> "$out/trace.log"; exit 125
	else echo "\$commit: no warp-self.wasm (no fixpoint): pure lane only" >> "$out/trace.log"; fi
fi
mkdir -p data && cp "$out/probe.warp" data/bisect-probe.warp
verdict=0
for l in \$lanes; do
	got=\$(timeout 120 "$out/warp" \$l data/bisect-probe.warp 2> "$out/probe.err"); status=\$?
	((status == 124)) && { echo "\$commit skip: \$l timed out" >> "$out/trace.log"; exit 125; }
	[[ -n \$got ]] || got=\$(grep -v '^ *[0-9]* |\|^ *|' "$out/probe.err" | head -1)
	[[ \$got == "\$(cat "$out/expected")" ]] || verdict=1
	echo "\$commit \$l: \${got:0:120}" >> "$out/trace.log"
done
exit \$verdict
STEP
chmod +x "$out/step.sh"
echo "== $(date '+%F %T') bisect $good..$bad, probe $probe, lane $lane" >> "$out/trace.log"
cd "$wt" && git bisect start $first_parent "$bad" "$good" > /dev/null && git bisect run "$out/step.sh" > "$out/bisect.log" 2>&1
first=$(grep -m1 'is the first bad commit' "$out/bisect.log" | cut -d' ' -f1)
if [[ -n $first ]]; then echo "first bad: $(git log -1 --format='%h %s' "$first")"
else echo "bisect-lane: no first bad commit (see $out/bisect.log, $out/trace.log)"; tail -3 "$out/bisect.log"; exit 1; fi
