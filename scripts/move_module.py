#!/usr/bin/env python3
"""Pure move of a root module into a concept folder (notes/restructuring.md, "Root").

move_module.py <module> <folder>      e.g. move_module.py real numbers

src/<module>.rs (and src/<module>/) go to src/<folder>/; the `mod` line with its attributes leaves lib.rs for the
folder's module file (created when missing, declared in lib.rs in place of the old line); every `crate::<module>` and
`warp::<module>` path becomes `…::<folder>::<module>`, lib.rs re-exports included. Its include_str!/include_bytes! paths gain one `../`;
its `super::` paths are reported for a look (inline `mod tests` keep theirs).
"""
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOURCE = os.path.join(REPO, "src")
LIB = os.path.join(SOURCE, "lib.rs")
PATH_DIRECTORIES = ["src", "tests", "benches", "examples", "crates", "build.rs", "probes"]


def read(path):
	return open(path).read()


def write(path, text):
	open(path, "w").write(text)


def git_mv(source, target):
	subprocess.run(["git", "-C", REPO, "mv", source, target], check=True)


def take_declaration(lib, module):
	"""The `mod` line of module with the attribute lines right above it, removed from lib"""
	lines = lib.split("\n")
	pattern = re.compile(r"^(pub(\([^)]*\))? )?mod " + module + r";")
	index = next(i for i, line in enumerate(lines) if pattern.match(line))
	start = index
	while start > 0 and lines[start - 1].startswith("#["):
		start -= 1
	return "\n".join(lines[:start] + lines[index + 1:]), lines[start:index + 1], start


def folder_module_file(folder):
	for candidate in (os.path.join(SOURCE, folder, "mod.rs"), os.path.join(SOURCE, folder + ".rs")):
		if os.path.exists(candidate):
			return candidate, False
	os.makedirs(os.path.join(SOURCE, folder), exist_ok=True)
	return os.path.join(SOURCE, folder, "mod.rs"), True


def declare(module_file, declaration):
	lines = read(module_file).split("\n") if os.path.exists(module_file) else []
	last_mod = max((i for i, line in enumerate(lines) if re.match(r"^(pub(\([^)]*\))? )?mod \w+;", line)), default=None)
	position = last_mod + 1 if last_mod is not None else len(lines)
	write(module_file, "\n".join(lines[:position] + declaration + lines[position:]).rstrip("\n") + "\n")


def rust_files():
	for entry in PATH_DIRECTORIES:
		path = os.path.join(REPO, entry)
		if os.path.isfile(path):
			yield path
		for root, _, files in os.walk(path):
			yield from (os.path.join(root, file) for file in files if file.endswith(".rs"))


def rewrite_grouped_imports(text, module, folder):
	"""`use warp::{a, module, b::c}`: the module named at the group's first level"""
	output, position = [], 0
	for opening in re.finditer(r"\buse (crate|warp)::\{", text):
		if opening.start() < position:
			continue
		depth, index = 1, opening.end()
		while depth:
			depth += {"{": 1, "}": -1}.get(text[index], 0)
			index += 1
		group, level, pieces, start = text[opening.end():index - 1], 0, [], 0
		for offset, char in enumerate(group + ","):
			level += {"{": 1, "}": -1}.get(char, 0)
			if char == "," and level == 0:
				pieces.append(group[start:offset])
				start = offset + 1
		pieces = [re.sub(r"^(\s*)" + module + r"\b", r"\g<1>" + folder + "::" + module, piece) for piece in pieces]
		output.append(text[position:opening.end()] + ",".join(pieces))
		position = index - 1
	return "".join(output) + text[position:]


def rewrite_file_mentions(module, folder):
	"""Comments, scripts and notes naming src/<module>.rs or src/<module>/ (the decision history keeps its wording)"""
	listed = subprocess.run(["git", "-C", REPO, "grep", "-lE", r"src/" + module + r"(\.rs|/)"], capture_output=True, text=True)
	for relative in listed.stdout.split():
		if relative != "notes/decisions.md":
			path = os.path.join(REPO, relative)
			write(path, re.sub(r"\bsrc/" + module + r"(?=\.rs|/)", "src/" + folder + "/" + module, read(path)))


def rewrite_paths(module, folder):
	pattern = re.compile(r"\b(crate|warp)::" + module + r"\b")
	for path in rust_files():
		text = read(path)
		changed = rewrite_grouped_imports(pattern.sub(r"\1::" + folder + "::" + module, text), module, folder)
		if path == LIB:
			changed = re.sub(r"\bpub use " + module + r"::", "pub use " + folder + "::" + module + "::", changed)
		if changed != text:
			write(path, changed)


def deepen_moved_file(path):
	text = read(path)
	if "super::" in text:
		print(f"{path}: check its super:: paths, they meant the crate root before the move")
	text = re.sub(r'(include_(?:str|bytes)!\(")(?!/)', r"\1../", text)
	write(path, text)


def deepen_includes(directory):
	for root, _, files in os.walk(directory):
		for file in files:
			if file.endswith(".rs"):
				path = os.path.join(root, file)
				write(path, re.sub(r'(include_(?:str|bytes)!\(")(?!/)', r"\1../", read(path)))


def main():
	module, folder = sys.argv[1], sys.argv[2]
	lib, declaration, line = take_declaration(read(LIB), module)
	module_file, created = folder_module_file(folder)
	if created:
		lines = lib.split("\n")
		lib = "\n".join(lines[:line] + ["pub mod " + folder + ";"] + lines[line:])
	write(LIB, lib)
	declare(module_file, declaration)
	target = os.path.join(SOURCE, folder)
	git_mv(os.path.join(SOURCE, module + ".rs"), os.path.join(target, module + ".rs"))
	deepen_moved_file(os.path.join(target, module + ".rs"))
	if os.path.isdir(os.path.join(SOURCE, module)):
		git_mv(os.path.join(SOURCE, module), os.path.join(target, module))
		deepen_includes(os.path.join(target, module))
	rewrite_paths(module, folder)
	rewrite_file_mentions(module, folder)
	print(f"{module} → {folder}/{module}")


if __name__ == "__main__":
	main()
