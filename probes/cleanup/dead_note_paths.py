"""Notes that name repository paths which no longer exist (cleanup-stale-notes): `python3 probes/cleanup/dead_note_paths.py`
prints each note with its dead paths, most first; `--fix` rewrites each one git recorded as moved (renames followed
to the path that exists now). A path counts when it starts with a top-level folder of the repo;
wiki/ is its own repository, checked out in the main checkout only."""
import os
import re
import subprocess
import sys

ROOTS = ("src/", "tests/", "lib/", "samples/", "probes/", "web/", "wiki/", "notes/", "integrator_tools/")
PATH = re.compile(r"(?<![\w/.-])((?:%s)[\w./-]*\w)" % "|".join(re.escape(root) for root in ROOTS))


def main_checkout():
	common = subprocess.run(["git", "rev-parse", "--git-common-dir"], capture_output=True, text=True).stdout.strip()
	return os.path.dirname(os.path.abspath(common))


def exists(path):
	return os.path.exists(path) or (path.startswith("wiki/") and os.path.exists(os.path.join(main_checkout(), path)))


def tracked_notes():
	listing = subprocess.run(["git", "ls-files", "notes/*.md", "notes/**/*.md"], capture_output=True, text=True)
	return sorted(set(listing.stdout.split()))


def dead_paths(note):
	text = open(note, encoding="utf-8").read()
	paths = {match.rstrip(".") for match in PATH.findall(text)}
	return sorted(path for path in paths if "*" not in path and "<" not in path and not exists(path))


def moved_paths():
	"""old path → the path it has now, following git's recorded renames (newest first, so a path's last move wins)"""
	log = subprocess.run(["git", "log", "-M", "--diff-filter=R", "--name-status", "--format="], capture_output=True, text=True)
	moves = {}
	for line in log.stdout.splitlines():
		parts = line.split("\t")
		if len(parts) == 3 and parts[1] not in moves:
			moves[parts[1]] = parts[2]
	def current(path, seen=()):
		new = moves.get(path)
		return path if new is None or new in seen else current(new, seen + (path,))
	return {old: current(old) for old in moves}


HISTORY = re.compile(r"\bwas\b|\bgone\b|no longer|removed|deleted|renamed", re.I)  # lines that name the old path on purpose


def fix(note, paths, moves):
	lines = open(note, encoding="utf-8").read().split("\n")
	for path in paths:
		new = moves.get(path) or (path[:-3] + "/mod.rs" if path.endswith(".rs") else None)  # a file that became a folder
		if not (new and os.path.exists(new)):
			continue
		written = re.compile(r"(?<![\w/.-])%s(?![\w/-])" % re.escape(path))
		lines = [line if HISTORY.search(line) else written.sub(new, line) for line in lines]
	open(note, "w", encoding="utf-8").write("\n".join(lines))


def main():
	report = [(note, dead_paths(note)) for note in tracked_notes()]
	if "--fix" in sys.argv:
		moves = moved_paths()
		for note, paths in report:
			fix(note, paths, moves)
		report = [(note, dead_paths(note)) for note in tracked_notes()]
	for note, paths in sorted(report, key=lambda entry: -len(entry[1])):
		if paths:
			print(f"{len(paths):3} {note}: {' '.join(paths[:8])}")


if __name__ == "__main__":
	main()
