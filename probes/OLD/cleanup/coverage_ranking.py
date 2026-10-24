"""Rank the src/ code a coverage run never executed, by lines: whole functions, then unexecuted lines inside executed
functions, then the files and folders holding the most of both.
Input from one `cargo llvm-cov --no-report` run of the test suite (which also runs every sample):
  cargo llvm-cov report --json > coverage.json      function extents and call counts
  cargo llvm-cov report --lcov > coverage.lcov      line hits
Usage: python3 probes/cleanup/coverage_ranking.py <coverage.json> <coverage.lcov> <checkout root> [top]"""
import json
import re
import sys
from collections import defaultdict

CODE_REGION = 0
FUNCTION_NAME = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")


def source_lines(path, cache={}):
	if path not in cache:
		with open(path) as file:
			cache[path] = file.read().split("\n")
	return cache[path]


def function_name(path, line):
	"""The `fn` named at or just above the line a function starts on, else it is a closure"""
	lines = source_lines(path)
	for number in range(line - 1, max(line - 6, -1), -1):
		found = FUNCTION_NAME.search(lines[number])
		if found:
			return found.group(1)
	return "closure"


def function_extents(coverage_path, source_root):
	"""(path, first line) → [last line, calls]; generic functions come once per instantiation, summed here"""
	extents = {}
	for function in json.load(open(coverage_path))["data"][0]["functions"]:
		path = function["filenames"][0]
		regions = [region for region in function["regions"] if region[5] == 0 and region[7] == CODE_REGION]
		if not path.startswith(source_root) or not regions:
			continue
		first, last = min(region[0] for region in regions), max(region[2] for region in regions)
		extent = extents.setdefault((path, first), [last, 0])
		extent[0] = max(extent[0], last)
		extent[1] += function["count"]
	return extents


def missed_lines(lcov_path):
	"""path → the lines no test executed"""
	missed, path = defaultdict(set), None
	for line in open(lcov_path):
		if line.startswith("SF:"):
			path = line[3:].strip()
		elif line.startswith("DA:"):
			number, hits = line[3:].split(",")[:2]
			if int(hits) == 0:
				missed[path].add(int(number))
	return missed


def main(coverage_path, lcov_path, root, top):
	source_root = root.rstrip("/") + "/src/"
	extents = function_extents(coverage_path, source_root)
	missed = missed_lines(lcov_path)
	# each missed line belongs to the innermost function around it (a closure inside its function)
	owner = {}
	for (path, first), (last, _) in sorted(extents.items(), key=lambda item: -(item[1][0] - item[0][1])):
		for line in missed[path] & set(range(first, last + 1)):
			owner[(path, line)] = (path, first)
	missed_by_function = defaultdict(int)
	for place in owner.values():
		missed_by_function[place] += 1
	dead, partial, by_file = [], [], defaultdict(int)
	for (path, first), lines in missed_by_function.items():
		relative = path[len(source_root) - 4:]
		row = (lines, f"{relative}:{first}", function_name(path, first))
		(dead if extents[(path, first)][1] == 0 else partial).append(row)
		by_file[relative] += lines
	by_folder = defaultdict(int)
	for relative, lines in by_file.items():
		by_folder[relative.rsplit("/", 1)[0]] += lines
	total = sum(len(lines) for path, lines in missed.items() if path.startswith(source_root))
	print(f"unexecuted src lines: {total}")
	print(f"never executed functions: {len(dead)}, {sum(row[0] for row in dead)} lines")
	print(f"unexecuted lines in executed functions: {sum(row[0] for row in partial)}\n")
	for title, rows in [("folders", by_folder.items()), ("files", by_file.items())]:
		print(f"== unexecuted lines by {title}")
		for place, lines in sorted(rows, key=lambda row: -row[1])[:top]:
			print(f"{lines:6} {place}")
		print()
	for title, rows in [("never executed functions", dead), ("executed functions, unexecuted lines", partial)]:
		print(f"== {title}")
		for lines, place, name in sorted(rows, reverse=True)[:top]:
			print(f"{lines:6} {name:40} {place}")
		print()


if __name__ == "__main__":
	main(*sys.argv[1:4], int(sys.argv[4]) if len(sys.argv) > 4 else 60)
