#!/usr/bin/env python3
"""Merge copies of a word constant into one (notes/restructuring.md, card duplicate-constants).

merge_constant.py [--home <file>] <NAME> <file>:<LOCAL_NAME>...

NAME is the constant in the home file (src/words.rs unless --home names another); when missing it is added there,
with the value and doc comment of the first copy. Each copy is cut from its file (with its doc comment), its uses in
that file are renamed to NAME and the file imports it (`use crate::words::NAME;`). A copy must hold the same value as
NAME. Uses of a copy from other files (`x::LOCAL`) are left for the compiler to name.
"""
import re
import sys

WORDS_FILE = "src/words.rs"


def definition(text, name):
	"""The match of the `const NAME …;` item with its doc comment, or None"""
	return re.search(r"((?:^[ \t]*///[^\n]*\n)*)^[ \t]*(?:pub(?:\([^)]*\))? )?const " + name + r"\b:\s*([^=]*?)\s*=\s*([^;]*);\n", text, re.M)


def visible_to_crate(text, name):
	"""The text with a private or pub(super) NAME widened to pub(crate): the copies' files use it"""
	return re.sub(r"^([ \t]*)(?:pub\(super\) )?const " + name + r"\b", r"\1pub(crate) const " + name, text, flags=re.M)


def module_path(path):
	"""crate::a::b of src/a/b.rs (src/a/mod.rs is crate::a)"""
	parts = path.removeprefix("src/").removesuffix(".rs").split("/")
	return "::".join(["crate"] + [part for part in parts if part != "mod"])


def with_import(text, home, name):
	"""The file importing home's NAME after its first use line (or at the top past the module docs)"""
	line = f"use {module_path(home)}::{name};\n"
	first_use = re.search(r"^use [^\n]*\n", text, re.M)
	if first_use:
		return text[:first_use.start()] + line + text[first_use.start():]
	module_docs = re.match(r"(?://![^\n]*\n)*", text)
	return text[:module_docs.end()] + line + text[module_docs.end():]


def main():
	arguments = sys.argv[1:]
	home = WORDS_FILE
	if arguments[0] == "--home":
		home, arguments = arguments[1], arguments[2:]
	name, copies = arguments[0], [argument.split(":") for argument in arguments[1:]]
	words = open(home).read()
	canonical = definition(words, name)
	if canonical:
		words = visible_to_crate(words, name)
		open(home, "w").write(words)
	for path, local in copies:
		text = open(path).read()
		copy = definition(text, local)
		if not copy:
			sys.exit(f"{path}: no const {local}")
		if canonical and canonical.group(3).strip() != copy.group(3).strip():
			sys.exit(f"{path}: {local} = {copy.group(3)}, but {module_path(home)}::{name} = {canonical.group(3)}")
		if not canonical:
			words = words.rstrip("\n") + f"\n{copy.group(1)}pub const {name}: {copy.group(2)} = {copy.group(3)};\n"
			open(home, "w").write(words)
			canonical = definition(words, name)
		text = text[:copy.start()] + text[copy.end():]
		text = re.sub(r"(?<![\w:])" + local + r"\b", name, text)
		imported = re.escape(module_path(home)) + r"::(\{[^}]*\b" + name + r"\b|" + name + ";)"
		if not re.search(r"^use " + imported, text, re.M):
			text = with_import(text, home, name)
		open(path, "w").write(text)
		print(f"{path}: {local} → {module_path(home)}::{name}")


if __name__ == "__main__":
	main()
