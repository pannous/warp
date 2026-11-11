#!/usr/bin/env bash
# `warp test` named many files at once runs every case of each: 300 files, more than the driver's first room for a
# program's arguments, overflowed it and stopped the run (card test-files). Run it with
# bash test/native/test_many_test_files.sh; the warp under test is WARP (default the warp on PATH), a tree's
# WARP=<tree>/data/gate/warp
warp=${WARP:-warp}
export WARP_WINDOW=0
FILES=300
cases=$(cd "$(dirname "$0")/../.." && pwd)/data/many-test-files
rm -rf "$cases" && mkdir -p "$cases"
for i in $(seq 1 $FILES); do printf "'%s + 1'\t%s\n" "$i" "$((i + 1))" >"$cases/test_n$i.pairs.warp"; done
said=$(cd "$cases" && "$warp" test test_n*.pairs.warp 2>&1); status=$?
case $said in
	*"✓ $FILES tests passed in $FILES files"*) echo "✓ $FILES files, every case run" ;;
	*) echo "✗ expected ✓ $FILES tests passed in $FILES files, got: ${said: -300}"; exit 1 ;;
esac
[ $status -eq 0 ] && echo "✓ exit 0" || { echo "✗ exit $status"; exit 1; }
