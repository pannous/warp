#!/usr/bin/env python3
"""Rewrites the flat sample paths of before 2026-10-16 (samples/fizzbuzz.warp) to the chapter folders
(samples/conditions/fizzbuzz.warp, notes/samples.md) in every tracked text file of a git checkout: for a branch, the
wiki or anything else written against the flat samples/. Also moves a flat sample a merge brings into the folder
given. Usage: probes/samples_paths.py [checkout, default .] [--move <name.warp> <folder>]…"""
import os, re, subprocess, sys

REPOSITORY = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SAMPLES = os.path.join(REPOSITORY, "samples")
NOT_CHAPTERS = ("golf", "wit", "tests")


def folders_of_files():
	"""each file of a chapter folder (fizzbuzz.warp, mozart.mp3) and its folder"""
	chapters = [name for name in os.listdir(SAMPLES) if os.path.isdir(os.path.join(SAMPLES, name)) and name not in NOT_CHAPTERS]
	return {file: chapter for chapter in chapters for file in os.listdir(os.path.join(SAMPLES, chapter))}


def rewritten(text, folders):
	"""samples/<name>.warp, samples/<name>.<ext> and a bare samples/<name> (samples/webgpu) with their folder; a bare
	name that is a folder too (samples/modules) stays: it names the folder"""
	names = dict(folders)
	stems = {file[:-len(".warp")]: folder for file, folder in folders.items() if file.endswith(".warp")}
	names.update({stem: folder for stem, folder in stems.items() if stem not in set(folders.values())})
	pattern = re.compile(r"(?<![\w.-])samples/(" + "|".join(map(re.escape, sorted(names, key=len, reverse=True))) + r")(?![\w/-])")
	return pattern.sub(lambda match: f"samples/{names[match.group(1)]}/{match.group(1)}", text)


def rewrite_checkout(checkout, folders):
	listed = subprocess.run(["git", "-C", checkout, "ls-files"], capture_output=True, text=True, check=True).stdout
	for path in filter(None, listed.split("\n")):
		if path.startswith(("samples/golf/", "samples/wit/")) or path == "probes/samples_paths.py":
			continue
		full = os.path.join(checkout, path)
		try:
			text = open(full, encoding="utf-8").read()
		except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
			continue
		new = rewritten(text, folders)
		if new != text:
			open(full, "w", encoding="utf-8").write(new)
			print(path)


def main(arguments):
	checkout = arguments[0] if arguments and not arguments[0].startswith("--") else "."
	moves = arguments[arguments.index("--move"):] if "--move" in arguments else []
	for file, folder in zip(moves[1::3], moves[2::3]):
		os.makedirs(os.path.join(SAMPLES, folder), exist_ok=True)
		subprocess.run(["git", "-C", REPOSITORY, "mv", f"samples/{file}", f"samples/{folder}/{file}"], check=True)
	rewrite_checkout(checkout, folders_of_files())


if __name__ == "__main__":
	main(sys.argv[1:])
