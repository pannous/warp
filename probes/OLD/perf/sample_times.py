#!/usr/bin/env python3
"""Seconds each sample takes with the warp CLI (WARP_NO_WINDOW), slowest first: which samples dominate
tests/programs/test_samples_run_cleanly. Usage: probes/perf/sample_times.py <warp binary> [count]"""
import os, subprocess, sys, time, glob

TIMEOUT = 20
NEEDS_A_WINDOW = ("raylib", "sdl")
warp, count = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 15
environment = dict(os.environ, WARP_NO_WINDOW="1", WARP_HINTS="0")
times = []
for sample in sorted(glob.glob("samples/*/*.warp")):
	if any(word in sample for word in NEEDS_A_WINDOW) or sample.startswith(("samples/golf/", "samples/wit/")):
		continue
	start = time.time()
	try:
		subprocess.run([warp, sample], env=environment, stdin=subprocess.DEVNULL, capture_output=True, timeout=TIMEOUT)
	except subprocess.TimeoutExpired:
		pass
	times.append((time.time() - start, sample))
for seconds, sample in sorted(times, reverse=True)[:count]:
	print(f"{seconds:6.2f} {sample}")
print(f"total {sum(seconds for seconds, _ in times):.1f} s over {len(times)} samples")
