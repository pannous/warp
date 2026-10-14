#!/usr/bin/env python3
"""Pure move of a module into a concept folder beside it (notes/restructuring.md).

move_module.py <module> <folder>                 e.g. move_module.py real numbers
move_module.py <parent>/<module> <folder>        e.g. move_module.py wasm_emitter/arithmetic numbers

src/<parent>/<module>.rs (and src/<parent>/<module>/) go to src/<parent>/<folder>/; the `mod` line with its attributes
leaves the parent's module file for the folder's mod.rs (created when missing and declared in place of the old line).
Every path to the module follows: `crate::`/`warp::` paths, the parent's bare paths, `super::` chains of the parent's
other files, and the moved files' own `super::` chains, `pub(super)` and include_str!/include_bytes! paths.
"""
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SOURCE = os.path.join(REPO, "src")
LIB = os.path.join(SOURCE, "lib.rs")
BINARY_ROOTS = [os.path.join(SOURCE, "main.rs")]
# A relative path in include_str!/include_bytes!, also when built with concat!
INCLUDE_PATH = re.compile(r'(include_(?:str|bytes)!\((?:concat!\()?")(?!/)')
PATH_DIRECTORIES = ["src", "tests", "benches", "examples", "crates", "build.rs", "probes"]
SUPER_CHAIN = re.compile(r"(?<![\w:])((?:super::)+)")
INLINE_MODULE = re.compile(r"^\s*(pub(\([^)]*\))? )?mod \w+ \{", re.M)
MOD_LINE = r"^(pub(\([^)]*\))? )?mod {};"


def read(path):
	return open(path).read()


def write(path, text):
	open(path, "w").write(text)


def git_mv(source, target):
	subprocess.run(["git", "-C", REPO, "mv", source, target], check=True)


def module_file(directory):
	"""The file declaring the modules of src/<directory> (lib.rs for the crate root)"""
	if not directory:
		return LIB
	for candidate in (os.path.join(SOURCE, directory, "mod.rs"), os.path.join(SOURCE, directory + ".rs")):
		if os.path.exists(candidate):
			return candidate
	return None


def depth_below(path, directory):
	"""How many `super::` lead from the module of path to the module of directory (0 for its module file)"""
	if path == module_file(directory):
		return 0
	parts = os.path.relpath(path, os.path.join(SOURCE, directory)).split(os.sep)
	return len(parts) - (1 if parts[-1] == "mod.rs" else 0)


def inline_module_spans(text):
	"""(start, end) of every inline `mod name { … }`; one more `super::` is needed inside each"""
	spans = []
	for opening in INLINE_MODULE.finditer(text):
		depth, index = 1, opening.end()
		while depth and index < len(text):
			depth += {"{": 1, "}": -1}.get(text[index], 0)
			index += 1
		spans.append((opening.start(), index))
	return spans


def nesting(spans, position):
	return sum(start <= position < end for start, end in spans)


def rust_files():
	for entry in PATH_DIRECTORIES:
		path = os.path.join(REPO, entry)
		if os.path.isfile(path):
			yield path
		for root, _, files in os.walk(path):
			yield from (os.path.join(root, file) for file in files if file.endswith(".rs"))


def files_below(directory):
	for root, _, files in os.walk(directory):
		yield from (os.path.join(root, file) for file in files if file.endswith(".rs"))


def rewrite_grouped_imports(text, module, folder, roots):
	"""`use <root>::{a, module, b::c}`: the module named at the group's first level"""
	output, position = [], 0
	for opening in re.finditer(r"\buse (" + roots + r")::\{", text):
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


def rewrite_super_paths(text, depth, module, folder):
	"""In a file `depth` modules below the parent: `super::…::module` reaching the parent gains the folder"""
	spans = inline_module_spans(text)

	def through_folder(match):
		chain = match.group(1)
		reaches_parent = chain.count("super::") == depth + nesting(spans, match.start())
		return chain + folder + "::" if reaches_parent else chain

	text = re.sub(SUPER_CHAIN.pattern + r"(?=" + module + r"\b)", through_folder, text)
	for supers in sorted({match.group(1).count("super::") for match in SUPER_CHAIN.finditer(text)}):
		text = rewrite_grouped_imports(text, module, folder, r"(?<![\w:])" + "super::" * (supers - 1) + "super")
	return text


def deepen_moved_file(text, depth, parent):
	"""The moved files sit one module deeper: chains reaching the parent or above gain a `super::`"""
	spans = inline_module_spans(text)

	def deeper(match):
		chain = match.group(1)
		return "super::" + chain if chain.count("super::") >= depth + nesting(spans, match.start()) else chain

	text = SUPER_CHAIN.sub(deeper, text)
	if depth == 1:
		meaning_parent = "pub(crate)" if not parent else "pub(in crate::" + parent.replace("/", "::") + ")"
		text = re.sub(r"pub\(super\)", lambda match: meaning_parent if not nesting(spans, match.start()) else match.group(0), text)
	return INCLUDE_PATH.sub(r"\1../", text)


def rewrite_paths(parent, module, folder):
	parent_path = parent.replace("/", "::")
	prefix = parent_path + "::" if parent else ""
	parent_directory = os.path.normpath(os.path.join(SOURCE, parent))
	for path in rust_files():
		in_this_crate = not os.path.relpath(path, REPO).startswith("crates")
		roots = "crate|warp" if in_this_crate else "warp"
		text = read(path)
		changed = re.sub(r"\b(" + roots + r")::" + prefix + module + r"\b", r"\1::" + prefix + folder + "::" + module, text)
		group_root = "(?:" + roots + ")" + ("::" + parent_path if parent else "")
		changed = rewrite_grouped_imports(changed, module, folder, group_root)
		if path == LIB and parent:
			changed = re.sub(r"(?<![\w:$])" + parent_path + "::" + module + r"\b", parent_path + "::" + folder + "::" + module, changed)
		if path.startswith(parent_directory + os.sep) and path not in BINARY_ROOTS:
			depth = depth_below(path, parent)
			globs_parent = re.search(r"^use " + "super::" * depth + r"\*;", changed, re.M)
			if depth == 0 or globs_parent:
				changed = re.sub(r"(?<![\w:$])(self::)?" + module + r"::", r"\g<1>" + folder + "::" + module + "::", changed)
			changed = rewrite_super_paths(changed, depth, module, folder)
		if changed != text:
			write(path, changed)


def rewrite_file_mentions(parent, module, folder):
	"""Comments, scripts and notes naming the module's file (not the decision history, not AGENTS.md: the user edits that)"""
	old = "src/" + (parent + "/" if parent else "") + module
	listed = subprocess.run(["git", "-C", REPO, "grep", "-lE", old + r"(\.rs|/)"], capture_output=True, text=True)
	new = "src/" + (parent + "/" if parent else "") + folder + "/" + module
	for relative in listed.stdout.split():
		if relative not in ("notes/decisions.md", "AGENTS.md"):
			path = os.path.join(REPO, relative)
			write(path, re.sub(r"\b" + re.escape(old) + r"(?=\.rs|/)", new, read(path)))


def take_declaration(text, module):
	"""The `mod` line of module with the attribute lines right above it, removed from text"""
	lines = text.split("\n")
	pattern = re.compile(MOD_LINE.format(module))
	index = next(i for i, line in enumerate(lines) if pattern.match(line))
	start = index
	while start > 0 and lines[start - 1].lstrip().startswith("#["):
		start -= 1
	return "\n".join(lines[:start] + lines[index + 1:]), lines[start:index + 1], start


def opened_to_parent(declaration, parent):
	"""A private `mod` was visible to the whole parent; inside the folder it needs saying so"""
	opened = "pub(crate) mod " if not parent else "pub(super) mod "
	return [re.sub(r"^mod ", opened, line) for line in declaration]


def declare(folder_file, declaration):
	lines = read(folder_file).split("\n") if os.path.exists(folder_file) else []
	last_mod = max((i for i, line in enumerate(lines) if re.match(r"^(pub(\([^)]*\))? )?mod \w+;", line)), default=None)
	position = last_mod + 1 if last_mod is not None else len(lines)
	write(folder_file, "\n".join(lines[:position] + declaration + lines[position:]).rstrip("\n") + "\n")


def declare_folder(parent_text, folder, line, public):
	"""`mod folder;` in the parent: pub when a member is pub (warp::… paths reach it), else pub(crate)"""
	visibility = "pub" if public else "pub(crate)"
	lines = parent_text.split("\n")
	existing = next((i for i, text in enumerate(lines) if re.match(MOD_LINE.format(folder), text)), None)
	if existing is None:
		lines.insert(line, visibility + " mod " + folder + ";")
	elif public:
		lines[existing] = re.sub(r"^(pub(\([^)]*\))? )?mod ", "pub mod ", lines[existing])
	return "\n".join(lines)


def split_target(argument):
	parent, _, module = argument.rpartition("/")
	return parent, module


def main():
	(parent, module), folder = split_target(sys.argv[1]), sys.argv[2]
	parent_file = module_file(parent)
	parent_text, declaration, line = take_declaration(read(parent_file), module)
	public = any(re.match(r"^pub mod ", text) for text in declaration)
	write(parent_file, declare_folder(parent_text, folder, line, public))
	source_directory = os.path.join(SOURCE, parent)
	target = os.path.join(source_directory, folder)
	# checked before the folder exists: a folder named like its member (reflection/reflection.rs) is not the member's
	module_directory = os.path.join(source_directory, module)
	has_directory = os.path.isdir(module_directory)
	moved = [os.path.join(source_directory, module + ".rs")] + (list(files_below(module_directory)) if has_directory else [])
	os.makedirs(target, exist_ok=True)
	declare(os.path.join(target, "mod.rs"), opened_to_parent(declaration, parent))
	rewrite_paths(parent, module, folder)
	for path in moved:
		write(path, deepen_moved_file(read(path), depth_below(path, parent), parent))
	git_mv(moved[0], os.path.join(target, module + ".rs"))
	if has_directory:
		git_mv(module_directory, os.path.join(target, module))
	rewrite_file_mentions(parent, module, folder)
	print(f"{sys.argv[1]} → {os.path.join(parent, folder, module)}")


if __name__ == "__main__":
	main()
