#!/usr/bin/env python3
"""Times `ys = xs.map(x => x * 0.5 + 1)` over 1M floats (notes/simd.md, card simd-map): ns per item of a map, from the
difference of 41 and 1 maps (by default) (compiling and filling xs cancel out), best of 3.
Usage: probes/simd/linear_map_bench.py <warp binary> [<other warp binary> …]   (scripts/own-warp.sh builds one)"""
import os, subprocess, sys, time

# WARP_BENCH_ITEMS=10000 WARP_BENCH_ROUNDS=4001: in cache instead of memory-bound
ITEMS = int(os.environ.get("WARP_BENCH_ITEMS", 1_000_000))
ROUNDS = (1, int(os.environ.get("WARP_BENCH_ROUNDS", 41)))
TRIES = 3
# the declaration of xs: an array in linear memory (the f64x2 kernel) and the automatic GC float array
DECLARATIONS = {"linear": "linear xs", "automatic": "xs"}
PROGRAM = """{declaration} = float[{items}]
for i in 1 to {items} {{ xs#i = i * 1.0 }}
s = 0.0
for r in 1 to {rounds} {{ ys = xs.map(x => x * 0.5 + 1); s += ys#7 }}
s"""


def seconds(warp, code):
	best = None
	for _ in range(TRIES):
		started = time.perf_counter()
		run = subprocess.run([warp, "--no-ask", "eval", code], capture_output=True, text=True)
		elapsed = time.perf_counter() - started
		if "Error" in run.stdout or run.returncode:
			sys.exit(f"{warp} failed: {run.stdout[-300:]}{run.stderr[-300:]}")
		best = elapsed if best is None else min(best, elapsed)
	return best


for warp in sys.argv[1:]:
	for name, declaration in DECLARATIONS.items():
		few, many = (seconds(warp, PROGRAM.format(declaration=declaration, items=ITEMS, rounds=rounds)) for rounds in ROUNDS)
		print(f"{warp} {name}: {(many - few) / (ROUNDS[1] - ROUNDS[0]) / ITEMS * 1e9:.2f} ns per item")
