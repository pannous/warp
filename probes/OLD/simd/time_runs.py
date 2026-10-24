#!/usr/bin/env python3
"""Times map_bench.wat functions under wasmtime with warp's engine settings (fuel, NaN canonicalization) on and off."""
import subprocess, sys, time

FUNCTIONS = ["scalar_gc_in_place", "scalar_memory", "simd_memory"]
SETTINGS = {"plain": "", "fuel": ",fuel=100000000000", "nan": ",nan-canonicalization=y", "fuel+nan": ",fuel=100000000000,nan-canonicalization=y"}
module = sys.argv[1] if len(sys.argv) > 1 else "map_bench.wat"
for label, extra in SETTINGS.items():
	for function in FUNCTIONS:
		start = time.time()
		run = subprocess.run(["wasmtime", "run", "-W", "gc=y,function-references=y" + extra, "--invoke", function, module], capture_output=True, text=True)
		result = run.stdout.strip() or run.stderr.strip().splitlines()[-1]
		print(f"{time.time() - start:6.2f}s {function:20} {label:9} {result}")
