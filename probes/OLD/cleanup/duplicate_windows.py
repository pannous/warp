"""Repeated windows of N consecutive code lines (whitespace-normalised, comments and trivial lines skipped) across files.
Usage: python3 probes/cleanup/duplicate_windows.py <window> <files...>"""
import sys, collections, re
window = int(sys.argv[1])
seen = collections.defaultdict(list)
for path in sys.argv[2:]:
	lines = []
	for number, line in enumerate(open(path), 1):
		code = re.sub(r'\s+', ' ', line.split('//')[0]).strip()
		if len(code) > 3 and code not in ('}', '});', '},', '];', ')', '{'):
			lines.append((number, code))
	for start in range(len(lines) - window + 1):
		chunk = tuple(code for _, code in lines[start:start + window])
		seen[chunk].append(f"{path}:{lines[start][0]}")
reported = set()
for chunk, places in sorted(seen.items(), key=lambda item: -len(item[1])):
	if len(places) > 1 and not any(place in reported for place in places):
		reported.update(places)
		print(f"{len(places)}x {', '.join(places[:6])}\n    " + "\n    ".join(chunk[:window]) + "\n")
