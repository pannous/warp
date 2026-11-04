#!/usr/bin/env bash
# lib performance benchmark (card lib-perf-bench): common lib functions at sizes 1k and 100k, warp against python.
# Not part of smoke or `warp test`. Each warp case is its own small program; its time is measured inside the program
# with clock() around the work only (compile and startup excluded), the median of 3 runs; python likewise
# (time.perf_counter). A run past BENCH_TIMEOUT seconds (default 60) is reported as a timeout.
# Usage: test/bench/lib_bench.sh [case-filter] [sizes]    e.g. test/bench/lib_bench.sh sort "1000 100000"
# Heavy: run it through /workspace/box-slot.sh. Needs warp on PATH (WARP_LIB names the lib/ to measure).
set -u
filter="${1:-}"
sizes="${2:-1000 100000}"
timeout_s="${BENCH_TIMEOUT:-60}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# name | uses | setup (warp, N given) | timed work (warp) | python setup | python timed work
cases=$(cat <<'CASES'
sort||xs = [(i * 7919) % N for i in 0 ..< N]|r = sort(xs)|xs = [(i * 7919) % N for i in range(N)]|r = sorted(xs)
split_join||t = join([str(i) for i in 0 ..< N], ",")|r = join(split(t, ","), ";")|t = ",".join(str(i) for i in range(N))|r = ";".join(t.split(","))
replace||t = join([str(i) for i in 0 ..< N], ",")|r = replace(t, ",", ";")|t = ",".join(str(i) for i in range(N))|r = t.replace(",", ";")
index_of_text||t = join(["ab" for i in 0 ..< N], "") + "needle"|r = index_of(t, "needle")|t = "ab" * N + "needle"|r = t.index("needle")
index_of_list||xs = [i for i in 0 ..< N]|r = index_of(xs, N - 1)|xs = list(range(N))|r = xs.index(N - 1)
map_set_get||keys = ["k" + str(i) for i in 0 ..< N]|m = {}; for k in keys { m[k] = 1 }; r = 0; for k in keys { r = r + m[k] }|keys = ["k" + str(i) for i in range(N)]|m = {}\n    for k in keys: m[k] = 1\n    r = sum(m[k] for k in keys)
filter_map||xs = [i for i in 0 ..< N]|r = xs.filter(x => x % 2 == 0).map(x => x * 3)|xs = list(range(N))|r = [x * 3 for x in xs if x % 2 == 0]
json_write|use json|v = [{"id": i, "name": "n" + str(i)} for i in 0 ..< N]|r = to_json(v)|import json\n    v = [{"id": i, "name": "n" + str(i)} for i in range(N)]|r = json.dumps(v)
json_read|use json|t = to_json([{"id": i, "name": "n" + str(i)} for i in 0 ..< N])|r = json(t)|import json\n    t = json.dumps([{"id": i, "name": "n" + str(i)} for i in range(N)])|r = json.loads(t)
csv_write|use csv|rows = [[str(i), "x" + str(i), "a,b"] for i in 0 ..< N]|r = csv_text(rows)|import csv, io\n    rows = [[str(i), "x" + str(i), "a,b"] for i in range(N)]|b = io.StringIO(); csv.writer(b).writerows(rows); r = b.getvalue()
csv_read|use csv|t = join([str(i) + ",x" + str(i) + ",\"a,b\"\r\n" for i in 0 ..< N], "")|r = csv_rows(t)|import csv, io\n    t = "".join(str(i) + ",x" + str(i) + ',"a,b"\\r\\n' for i in range(N))|r = list(csv.reader(io.StringIO(t)))
regex_find_all|use regex|t = join(["a" + str(i) for i in 0 ..< N], " ")|r = find_all(t, "[0-9]+")|import re\n    t = " ".join("a" + str(i) for i in range(N))|r = re.findall("[0-9]+", t)
json_write_long|use json|t = join(["abcdefghi\\n" for i in 0 ..< N /! 10], "")|r = to_json(t)|import json\n    t = "abcdefghi\\n" * (N // 10)|r = json.dumps(t)
json_read_long|use json|t = to_json(join(["abcdefghi\\n" for i in 0 ..< N /! 10], ""))|r = json(t)|import json\n    t = json.dumps("abcdefghi\\n" * (N // 10))|r = json.loads(t)
string_build||n = N|s = ""; for i in 0 ..< n { s = s + "x" }; r = s|n = N|s = ""\n    for i in range(n): s = s + "x"\n    r = s
CASES
)

median() { sort -n | awk '{a[NR]=$1} END{if(NR==0){print "";exit} print a[int((NR+1)/2)]}'; }

run_warp() { # name uses setup work N → ms or "timeout"/"error"
	local f="$work/$1_$5.warp"
	{ [ -n "$2" ] && echo "$2"; echo "N = $5"; echo "$3"; echo "bench_start = clock()"; echo "$4"; echo "print(clock() - bench_start)"; } > "$f"
	local out; out=$(timeout "$timeout_s" warp "$f" 2>/dev/null); local rc=$?
	if [ $rc -eq 124 ]; then echo timeout; return; fi
	local ms; ms=$(echo "$out" | tail -1 | tr -d '\r')
	if [[ "$ms" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then echo "$ms"; else echo error; fi
}
run_python() { # setup work N → ms
	local f="$work/py.py"
	printf 'import time\ndef main(N):\n    %b\n    t0 = time.perf_counter()\n    %b\n    return (time.perf_counter() - t0) * 1000\nprint("%%.2f" %% main(%s))\n' "$1" "$2" "$3" > "$f"
	timeout "$timeout_s" python3 "$f" 2>/dev/null || echo error
}

printf '| case | size | warp ms | python ms | ratio |\n|---|---|---|---|---|\n'
while IFS='|' read -r name uses setup wwork psetup pwork; do
	[ -z "$name" ] && continue
	[ -n "$filter" ] && [[ "$name" != *$filter* ]] && continue
	for n in $sizes; do
		ws=(); bad=""
		for k in 1 2 3; do
			r=$(run_warp "$name" "$uses" "$setup" "$wwork" "$n")
			if [ "$r" = timeout ] || [ "$r" = error ]; then bad=$r; break; fi
			ws+=("$r")
		done
		ps=$(for k in 1 2 3; do run_python "$psetup" "$pwork" "$n"; done | median)
		if [ -n "$bad" ]; then w="$bad"; ratio="-"
		else w=$(printf '%s\n' "${ws[@]}" | median); ratio=$(awk -v a="$w" -v b="$ps" 'BEGIN{ if (b+0 < 0.05) b = 0.05; printf "%.0f", a/b }'); fi
		printf '| %s | %s | %s | %s | %s |\n' "$name" "$n" "$w" "$ps" "$ratio"
	done
done <<< "$cases"
