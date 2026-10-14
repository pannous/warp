"""The runs of src/ lines a coverage run never executed, mapped onto today's checkout, biggest first: where to look
for dead branches (card cleanup-sweep). The coverage is from an older commit; difflib carries each line over to the
current file, a changed line drops out.
Usage: python3 probes/cleanup/missed_blocks.py <coverage.lcov> <commit of the coverage run> <checkout> [min lines]
  [--skip path-prefix ...]"""
import difflib
import subprocess
import sys
from collections import defaultdict

lcov_path, coverage_commit, checkout = sys.argv[1:4]
minimum = int(sys.argv[4]) if len(sys.argv) > 4 and sys.argv[4].isdigit() else 5
skipped = sys.argv[sys.argv.index("--skip") + 1:] if "--skip" in sys.argv else []


def missed_lines():
	missed, path = defaultdict(set), None
	for line in open(lcov_path):
		if line.startswith("SF:"):
			path = line[3:].strip()
		elif line.startswith("DA:") and int(line[3:].split(",")[1]) == 0:
			missed[path].add(int(line[3:].split(",")[0]))
	return missed


def relative(path):
	return path[path.index("/src/") + 1:] if "/src/" in path else None


def carried_over(old_lines, new_lines):
	"""old line number → new line number for the lines both versions share"""
	mapping = {}
	for block in difflib.SequenceMatcher(None, old_lines, new_lines, autojunk=False).get_matching_blocks():
		for offset in range(block.size):
			mapping[block.a + offset + 1] = block.b + offset + 1
	return mapping


def blank_or_comment(text):
	stripped = text.strip()
	return not stripped or stripped.startswith("//") or stripped in ("}", "};", "},", ")", "),", "]")


blocks = []
for path, lines in missed_lines().items():
	name = relative(path)
	if not name or any(name.startswith(prefix) for prefix in skipped):
		continue
	try:
		new_lines = open(f"{checkout}/{name}").read().split("\n")
	except FileNotFoundError:
		continue
	old = subprocess.run(["git", "-C", checkout, "show", f"{coverage_commit}:{name}"], capture_output=True, text=True)
	mapping = carried_over(old.stdout.split("\n"), new_lines)
	current = sorted(mapping[line] for line in lines if line in mapping)
	run = []
	for line in current + [None]:
		gap_is_trivial = run and line and all(blank_or_comment(new_lines[n - 1]) for n in range(run[-1] + 1, line))
		if run and line and gap_is_trivial:
			run.append(line)
			continue
		if len(run) >= minimum:
			blocks.append((run[-1] - run[0] + 1, name, run[0], new_lines[run[0] - 1].strip()[:90]))
		run = [line] if line else []
for size, name, first, text in sorted(blocks, reverse=True):
	print(f"{size:5} {name}:{first}  {text}")
