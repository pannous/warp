#!/bin/bash
# scripts/cloud-test.sh --full in shards (card cloud-lane-full), offline: every listed test file lands in exactly one
# shard, and the shards' results merge into one verdict (a killed shard's rerun adds its panic text, counts no test).
# Usage: probes/cloud_full_shards.sh <file of test names, one per line>; works in scratch/full-shards/
set -u
names=${1:?a file of test names, e.g. sed -n 's/^test \([^ ]*\) \.\.\. .*/\1/p' of a full run log}
root=$(cd "$(dirname "$0")/.." && pwd)
eval "$(sed '$d' "$root/scripts/cloud-test.sh")"
cargo() { sed 's/$/: test/' "$names"; }   # `cargo test … -- --list`
source_flags=(--offline) test_args=("${FULL_FLAGS[@]}")
work=$root/scratch/full-shards
mkdir -p "$work"
HERE_WORK=$work
slow_files > "$HERE_WORK/slow"   # from the newest merged --full run in data/cloud/
echo "slow files: $(tr '\n' ' ' < "$HERE_WORK/slow")"

for shard in $(seq 1 $FULL_SHARDS); do shard_filters "$shard/$FULL_SHARDS" > "$work/filters-$shard"; done
# each test matched by the filters of exactly one shard (libtest: a filter matches when the name contains it)
awk -v shards=$FULL_SHARDS 'FILENAME ~ /filters-/ { split(FILENAME, part, "-"); owner[$0] = owner[$0] " " part[2]; filters[$0]; next }
	{ hits = 0; for (filter in filters) if (index($0, filter)) hits++; if (hits != 1 && bad++ < 5) print "matched by " hits ": " $0 }
	END { print bad ? "PARTITION BROKEN" : "partition ok: " FNR " tests" }' "$work"/filters-* "$names"
for shard in $(seq 1 $FULL_SHARDS); do printf "shard %s: %s files " $shard $(wc -l < "$work/filters-$shard"); done; echo

# two shards: one green, one killed with a failure whose panic text only its rerun printed
shard_result() {   # folder verdict log
	mkdir -p "$work/$1"
	printf 'run: %s\nbranch: batch-x\ntip: abc\nverdict: %s\npassed: …\nwall: build 300 s, tests %s s, total %s s\nsession: https://claude.ai/code/session_%s\n' \
		"$1" "$2" "$3" "$3" "$1" > "$work/$1/result.md"
	printf '%s' "$4" | gzip > "$work/$1/log.txt.gz"
}
shard_result one green 600 "     Running tests/main.rs (x)
test a::b::c ... ok
test a::b::d ... ignored

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 2 filtered out; finished in 1s
"
shard_result two red 900 "     Running tests/main.rs (x)
test e::f::g ... ok
test e::f::h ... FAILED
cloud-test: KILLED after the time limit of 3600 s
$RERUN_MARK of the killed run's failed tests, for their panic texts
     Running tests/main.rs (x)
test e::f::h ... FAILED

failures:

---- e::f::h stdout ----

thread 'e::f::h' panicked at tests/e/f.rs:1:1:
the rerun's panic text

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 1s
"
merge_shards "$work/merged" abc "$work/one" "$work/two" 2>/dev/null
cat "$work/merged/result.md"   # verdict red; e::f::h with the rerun's panic text; a killed binary; 2 counted of 4 listed: INCOMPLETE
