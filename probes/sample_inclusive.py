#!/usr/bin/env python3
"""Inclusive sample counts of the functions matching the given name fragments in a macOS `sample` call graph:
    probes/sample_inclusive.py <sample.txt> <fragment>…   (no fragments: the 25 heaviest functions)"""
import re, sys

FRAME = re.compile(r'^(\s*[+!:| ]*)(\d+) (\S+)')

def inclusive_counts(path, fragments):
	graph = open(path, errors='replace').read().split('Call graph:')[1].split('Total number in stack')[0]
	stack, totals = [], {}
	for line in graph.splitlines():
		frame = FRAME.match(line)
		if not frame:
			continue
		depth, count, name = len(frame.group(1)), int(frame.group(2)), frame.group(3)
		while stack and stack[-1][0] >= depth:
			stack.pop()
		key = next((fragment for fragment in fragments if fragment in name), None) if fragments else name
		if key and not any(key in caller if fragments else key == caller for _, caller in stack):
			totals[key] = totals.get(key, 0) + count
		stack.append((depth, name))
	return totals

if __name__ == '__main__':
	totals = inclusive_counts(sys.argv[1], sys.argv[2:])
	for name, count in sorted(totals.items(), key=lambda item: -item[1])[:25]:
		print(count, name[:150])
