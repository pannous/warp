#!/bin/bash
# The tiered gate of a tree (a merged batch, a branch, main), pure pipeline first, stopping at the first red tier:
#   tier 0  rebuild pure + pure hello, rebuild self + self hello   (the self check; minutes, the rebuilds)
#   tier 1  every probes/gate/<name>.warp in both lanes, its stdout against <name>.expected   (seconds)
#   tier 2  `warp test` (pure) of the test files the tree changed since --base   (seconds to minutes)
#   tier 3  Rust: only when the change touches src/, crates/, *.rs, Cargo.*, build.rs; not run here, only named
#           (scripts/cloud-test.sh, or the Integrator's cargo gate)
# usage: scripts/gate-probes.sh [--tiers 0-2|0,1|1] [--no-rebuild] [--base <ref>] [--folders] [--tag-good]
#                               [<tree>, default this checkout] [<binary>, default /workspace/cow/sol-warp-latest]
#   --no-rebuild  tier 0 only runs hello with the tree's built pair (data/compiler.wasm, data/compiler-self.wasm)
#   --base        what tier 2 diffs against (default origin/main; a batch merged on main: the main it started from)
#   --folders     tier 2 runs the whole folders of the changed test files (many fail today: compare with the base's run)
#   --tag-good    when every tier run is green and the tree's HEAD is on origin/main: tag good/<YYYY-MM-DD-HHMM>
#                 (Europe/Berlin) and push the tag; scripts/bisect-lane.sh starts from the newest good/* tag
# Output: one line per tier (time), one per probe and lane (ok, or FAIL got … want …), logs in <tree>/data/gate/.
# Exit: 0 green, 1 a red tier, 2 usage. Heavy (the rebuilds): on the Grok box run it through /workspace/box-slot.sh.
tiers=0-2 rebuild=1 base=origin/main folders=0 tag=0 args=()
while (($#)); do
	case $1 in
		--tiers) tiers=$2; shift 2 ;;
		--no-rebuild) rebuild=0; shift ;;
		--base) base=$2; shift 2 ;;
		--folders) folders=1; shift ;;
		--tag-good) tag=1; shift ;;
		-h|--help) sed -n '2,18p' "$0"; exit 0 ;;
		-*) echo "gate: unknown flag $1" >&2; exit 2 ;;
		*) args+=("$1"); shift ;;
	esac
done
tree=$(cd "${args[0]:-$(git rev-parse --show-toplevel)}" && pwd) || exit 2
binary=${args[1]:-${WARP_GATE_BINARY:-/workspace/cow/sol-warp-latest}}
wants() { # wants <tier>: the tier is in --tiers (a range a-b or a list a,b)
	local t
	for t in ${tiers//,/ }; do
		if [[ $t == *-* ]]; then (($1 >= ${t%-*} && $1 <= ${t#*-})) && return 0; elif (($1 == t)); then return 0; fi
	done
	return 1
}
logs=$tree/data/gate; mkdir -p "$logs"
# a copy of the binary taken once: a refresh of the shared link mid-run never mixes two builds
warp=$logs/warp; cp -L "$binary" "$warp" || exit 1
export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$tree/lib WARP_MODULE_CACHE=${WARP_MODULE_CACHE:-$logs/module-cache}
export WARP_GATE_PROBE=gate-ok
cd "$tree" || exit 1
started=$(date +%s)
since() { echo "$(($(date +%s) - $1))s"; }
red() { echo "gate: RED at tier $1 ($(git rev-parse --short HEAD), $(since $started) in all)"; exit 1; }
# the first line worth showing of a run: its stdout, else its first error line without the source gutter
said() { local text; text=$(grep -v '^ *[0-9]* |\|^ *|' "$1" | head -1); echo "${text:0:160}"; }

if wants 0; then
	t0=$(date +%s) ok=1
	for lane in pure self; do
		if ((rebuild)); then
			"$warp" rebuild $lane > "$logs/rebuild-$lane.log" 2>&1 || { echo "FAIL rebuild $lane: $(grep -i 'error\|refus' "$logs/rebuild-$lane.log" | head -1 | cut -c1-160) (log $logs/rebuild-$lane.log)"; red 0; }
		fi
		got=$(timeout 300 "$warp" $lane samples/hello/hello.warp 2>&1 | tail -1)
		[[ $got == *Hello* ]] || { echo "FAIL $lane hello: got ${got:0:160}"; red 0; }
	done
	echo "tier 0 (rebuild + hello, both lanes): $( ((ok)) && echo ok || echo RED) $(since $t0)"
	((ok)) || red 0
fi

if wants 1; then
	t1=$(date +%s) ok=1
	for probe in probes/gate/*.warp; do
		name=${probe%.warp}; want=$(cat "$name.expected" 2>/dev/null) || { echo "FAIL ${name##*/}: no $name.expected"; ok=0; continue; }
		for lane in pure self; do
			timeout 120 "$warp" $lane "$probe" > "$logs/probe.out" 2> "$logs/probe.err"
			got=$(cat "$logs/probe.out")
			if [[ $got == "$want" ]]; then echo "ok   ${name##*/} ($lane)"
			else
				[[ -n $got ]] || got=$(said "$logs/probe.err")
				echo "FAIL ${name##*/} ($lane): got ${got//$'\n'/⏎} want ${want//$'\n'/⏎}"; ok=0
			fi
		done
	done
	echo "tier 1 (probes/gate, both lanes): $( ((ok)) && echo ok || echo RED) $(since $t1)"
	((ok)) || red 1
fi

from=$(git merge-base "$base" HEAD 2>/dev/null)
if wants 2; then
	t2=$(date +%s)
	files=$(git diff --name-only --diff-filter=AMR "$from" HEAD -- 'tests/*.warp' 2>/dev/null)
	if [[ -z $files ]]; then echo "tier 2 (pure warp test of the changed tests): none changed since $base"
	else
		targets=$files; ((folders)) && targets=$(dirname $files | sort -u)
		# warp test takes folders: the changed files are copied into one of their own (relative lib resolution: in the tree)
		if ((! folders)); then rm -rf "$logs/tests" && mkdir -p "$logs/tests" && cp $files "$logs/tests/"; targets=$logs/tests; fi
		timeout 2400 "$warp" test $targets > "$logs/tests.log" 2>&1; status=$?
		echo "tier 2 (pure warp test of $(echo "$files" | wc -l) changed test files$( ((folders)) && echo ", whole folders")): $( ((status == 0)) && echo ok || echo RED) $(since $t2): $(tail -1 "$logs/tests.log")"
		((status == 0)) || { grep '^✗' "$logs/tests.log" | head -10 | cut -c1-200; red 2; }
	fi
fi

rust=$(git diff --name-only "$from" HEAD -- src crates Cargo.toml Cargo.lock '*.rs' '*build.rs' 2>/dev/null | head -3)
[[ -n $rust ]] && echo "tier 3 (Rust): needed, not run here: the change touches ${rust//$'\n'/ } … (scripts/cloud-test.sh / the cargo gate)"
echo "gate: green ($(git rev-parse --short HEAD), $(since $started) in all)"

if ((tag)); then
	git fetch -q origin main && git merge-base --is-ancestor HEAD origin/main ||
		{ echo "gate: not tagging, HEAD $(git rev-parse --short HEAD) is not on origin/main"; exit 0; }
	name=good/$(TZ=Europe/Berlin date +%Y-%m-%d-%H%M)
	git tag "$name" HEAD && git push -q origin "refs/tags/$name" && echo "gate: tagged $name = $(git rev-parse --short HEAD)"
fi
exit 0
