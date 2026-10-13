"""Notes that name repository paths which no longer exist (cleanup-stale-notes): `python3 probes/cleanup/dead_note_paths.py`
prints each note with its dead paths, most first. A path counts when it starts with a top-level folder of the repo;
wiki/ is its own repository, checked out in the main checkout only."""
import os
import re
import subprocess

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


def main():
	report = [(note, dead_paths(note)) for note in tracked_notes()]
	for note, paths in sorted(report, key=lambda entry: -len(entry[1])):
		if paths:
			print(f"{len(paths):3} {note}: {' '.join(paths[:8])}")


if __name__ == "__main__":
	main()
