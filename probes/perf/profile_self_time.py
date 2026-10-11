#!/usr/bin/env python3
"""Self and inclusive time per function of a wasmtime --profile=guest JSON (Firefox profiler format).
Usage: profile_self_time.py data/perf/paint.json [rows]"""
import collections, json, sys

profile = json.load(open(sys.argv[1]))
rows = int(sys.argv[2]) if len(sys.argv) > 2 else 20
self_time, inclusive_time, total = collections.Counter(), collections.Counter(), 0
for thread in profile['threads']:
	stacks, frames, functions = thread['stackTable'], thread['frameTable'], thread['funcTable']
	strings = thread.get('stringArray') or profile['shared']['stringArray']
	name = lambda stack: strings[functions['name'][frames['func'][stacks['frame'][stack]]]]
	for stack in thread['samples']['stack']:
		if stack is None:
			continue
		total += 1
		self_time[name(stack)] += 1
		seen = set()
		while stack is not None:
			seen.add(name(stack))
			stack = stacks['prefix'][stack]
		inclusive_time.update(seen)
print(f"{total} samples\n  self  incl  function")
for function, count in self_time.most_common(rows):
	print(f"{count * 100 / total:5.1f}% {inclusive_time[function] * 100 / total:5.1f}%  {function}")
