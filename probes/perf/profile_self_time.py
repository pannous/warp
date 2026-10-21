#!/usr/bin/env python3
"""Self and inclusive time per function of a wasmtime --profile=guest JSON (Firefox profiler format).
Usage: profile_self_time.py data/perf/paint.json [rows] [function…]
Each function named after rows also gets its callers: the inclusive time of that function under each caller."""
import collections, json, sys

profile = json.load(open(sys.argv[1]))
rows = int(sys.argv[2]) if len(sys.argv) > 2 else 20
traced = sys.argv[3:]
self_time, inclusive_time, total = collections.Counter(), collections.Counter(), 0
callers = {function: collections.Counter() for function in traced}
for thread in profile['threads']:
	stacks, frames, functions = thread['stackTable'], thread['frameTable'], thread['funcTable']
	strings = thread.get('stringArray') or profile['shared']['stringArray']
	name = lambda stack: strings[functions['name'][frames['func'][stacks['frame'][stack]]]]
	for stack in thread['samples']['stack']:
		if stack is None:
			continue
		total += 1
		self_time[name(stack)] += 1
		seen, path = set(), []
		while stack is not None:
			path.append(name(stack))
			stack = stacks['prefix'][stack]
		inclusive_time.update(set(path))
		for function in traced:
			# the outermost call of the function, so a recursive one counts once
			if function in path:
				outermost = len(path) - 1 - path[::-1].index(function)
				callers[function][path[outermost + 1] if outermost + 1 < len(path) else '(root)'] += 1
print(f"{total} samples\n  self  incl  function")
for function, count in self_time.most_common(rows):
	print(f"{count * 100 / total:5.1f}% {inclusive_time[function] * 100 / total:5.1f}%  {function}")
for function in traced:
	print(f"\ncallers of {function}:")
	for caller, count in callers[function].most_common(rows):
		print(f"{count * 100 / total:5.1f}%  {caller}")
