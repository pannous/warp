#!/usr/bin/env python3
"""probes/wasm_function_sizes.py <old.wasm> <new.wasm>: per-function size change, biggest first (card samples-quicksort)"""
import re, subprocess, sys

def sizes(path):
	out = subprocess.run(["wasm-objdump", "-x", "-j", "Code", path], capture_output=True, text=True).stdout
	found = {}
	for line in out.splitlines():
		match = re.search(r"func\[(\d+)\] size=(\d+)(?: <(.*)>)?", line)
		if match:
			found[match.group(3) or "#" + match.group(1)] = int(match.group(2))
	return found

old, new = sizes(sys.argv[1]), sizes(sys.argv[2])
rows = sorted(((new.get(name, 0) - old.get(name, 0), name) for name in set(old) | set(new)), reverse=True)
for delta, name in rows:
	if delta:
		print(f"{delta:+6d} {name} ({old.get(name, 0)} -> {new.get(name, 0)})")
print("total", sum(old.values()), "->", sum(new.values()))
