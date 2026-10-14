#!/usr/bin/env python3
"""Pure move of Rust functions, consts, statics and type aliases between files (notes/restructuring.md).

move_fn.py <search_dir> <target_file> <name>... [--impl TYPE] [--from FILE]

--from takes the items from that one file only: names defined elsewhere too (`show`, `fail`) stay unambiguous.

Each named item (with its doc comments and attributes) is cut from the one file under <search_dir> that defines it and
appended to <target_file>: methods into the target's last `impl TYPE {` block (created when missing), free functions
at the end. A method leaving mod.rs or for a deeper module gets `pub(super)` widened to `pub(crate)`, a free function also when private. Nothing else
in the code changes; module declarations and imports are fixed by hand after the compiler names them.
"""
import argparse
import os
import re
import sys

NEW_FILE_HEADER = "use super::*;\n"


def function_line(lines, name):
	pattern = re.compile(r"^\s*(pub(\([^)]*\))? )?(((const )?(unsafe )?fn)|const|static|type) " + re.escape(name) + r"\b")
	return [index for index, line in enumerate(lines) if pattern.match(line)]


def is_value_item(line):
	"""A const, static or type alias: it ends at its `;`, not at a closing brace"""
	return re.match(r"^\s*(pub(\([^)]*\))? )?(const|static|type) (?!fn\b)", line) is not None


def statement_end(text, start):
	"""Offset just past the `;` ending the item at start, outside brackets"""
	depth, position = 0, start
	while position < len(text):
		char = text[position]
		if char == '"':
			position += 1
			while text[position] != '"':
				position += 2 if text[position] == "\\" else 1
		elif char == "'":
			literal = re.match(r"'(\\.[^']*|[^'\\])'", text[position:position + 12])
			position += literal.end() - 1 if literal else 0
		elif char in "([{":
			depth += 1
		elif char in ")]}":
			depth -= 1
		elif char == ";" and depth == 0:
			return position + 1
		position += 1
	raise ValueError("no ; ending the item")


def first_attached_line(lines, index):
	while index > 0:
		previous = lines[index - 1].strip()
		if previous.startswith("///") or previous.startswith("#[") or (previous.startswith("//") and not previous.startswith("//!")):
			index -= 1
		else:
			break
	return index


def body_end(text, start):
	"""Offset just past the `}` closing the first `{` at or after start, skipping strings, chars and comments"""
	depth, position, length = 0, start, len(text)
	while position < length:
		char = text[position]
		if text.startswith("//", position):
			position = text.find("\n", position)
			continue
		if text.startswith("/*", position):
			position = text.find("*/", position) + 2
			continue
		raw = re.match(r'b?r(#*)"', text[position:position + 12])
		if raw and (position == 0 or not (text[position - 1].isalnum() or text[position - 1] == "_")):
			closing = '"' + raw.group(1)
			position = text.find(closing, position + raw.end()) + len(closing)
			continue
		if char == '"':
			position += 1
			while text[position] != '"':
				position += 2 if text[position] == "\\" else 1
			position += 1
			continue
		if char == "'":
			literal = re.match(r"'(\\.[^']*|[^'\\])'", text[position:position + 12])
			position += literal.end() if literal else 1
			continue
		if char == "{":
			depth += 1
		elif char == "}":
			depth -= 1
			if depth == 0:
				return position + 1
		position += 1
	raise ValueError("unbalanced braces")


def cut_function(contents, path, name):
	lines = contents[path].split("\n")
	found = function_line(lines, name)
	if not found:
		return None
	start = first_attached_line(lines, found[0])
	text = "\n".join(lines)
	offset = len("\n".join(lines[:found[0]])) + 1 if found[0] else 0
	end_offset = (statement_end if is_value_item(lines[found[0]]) else body_end)(text, offset)
	end_line = text.count("\n", 0, end_offset)
	piece = lines[start:end_line + 1]
	rest = lines[:start] + lines[end_line + 1:]
	if start < len(rest) and start > 0 and rest[start - 1].strip() == "" and rest[start].strip() in ("", "}"):
		del rest[start - 1]
	contents[path] = "\n".join(rest)
	return piece


def depth_below(search_dir, path):
	return os.path.relpath(path, search_dir).count(os.sep)


def widened(piece):
	"""pub(super) means another module after the move; a free function is also reached through re-exports"""
	item = r"(?:(?:const )?fn|const|static|type) "
	widen = re.compile(r"^(\s*)pub\(super\) " if piece[-1].startswith("\t") else r"^()(?:pub\(super\) )?(?=" + item + ")")
	return [widen.sub(r"\1pub(crate) ", line, count=1) if re.match(r"^\s*(pub\(super\) )?" + item, line) else line for line in piece]


def insert_methods(text, impl_type, methods):
	header = "impl " + impl_type + " {"
	position = text.rfind("\n" + header)
	if position < 0 and not text.startswith(header):
		return text.rstrip("\n") + "\n\n" + header + "\n" + methods + "}\n"
	position = 0 if position < 0 else position + 1
	closing = body_end(text, position) - 1
	before = text[:closing].rstrip("\n") + "\n"
	separator = "" if before.endswith("{\n") else "\n"
	return before + separator + methods + text[closing:]


def insert_after_header(text, values):
	"""Constants go to the top of the file: after its module docs, `use` and `mod` lines"""
	lines = text.split("\n")
	header = re.compile(r"^(//!|use |pub(\([^)]*\))? ?use |mod |pub(\([^)]*\))? ?mod |\s*$)")
	position = 0
	while position < len(lines) and header.match(lines[position]):
		position += 1
	while position > 0 and lines[position - 1].strip() == "":
		position -= 1
	return "\n".join(lines[:position] + [""] + values + lines[position:])


def main():
	parser = argparse.ArgumentParser()
	parser.add_argument("search_dir")
	parser.add_argument("target")
	parser.add_argument("names", nargs="+")
	parser.add_argument("--impl", default="WarpParser")
	parser.add_argument("--from", dest="source")
	arguments = parser.parse_args()
	sources = [os.path.join(root, file) for root, _, files in os.walk(arguments.search_dir) for file in files if file.endswith(".rs")]
	if arguments.source:
		sources = [arguments.source]
	located = []
	for name in arguments.names:
		holders = [path for path in sources if os.path.abspath(path) != os.path.abspath(arguments.target) and function_line(open(path).read().split("\n"), name)]
		if len(holders) != 1:
			sys.exit(f"{name}: defined in {holders or 'no file'}")
		located.append((holders[0], function_line(open(holders[0]).read().split("\n"), name)[0], name))
	located.sort()
	os.makedirs(os.path.dirname(arguments.target) or ".", exist_ok=True)
	methods, free, values, contents = [], [], [], {path: open(path).read() for path, _, _ in located}
	deeper = depth_below(arguments.search_dir, arguments.target) > 0
	for path, _, name in located:
		# every definition of the name: `#[cfg(feature = "native")] fn f` and its `#[cfg(not(…))]` twin go together
		while found := function_line(contents[path].split("\n"), name):
			value = is_value_item(contents[path].split("\n")[found[0]])
			piece = cut_function(contents, path, name)
			# what mod.rs keeps private its submodules still see; a sibling file does not
			piece = widened(piece) if deeper or os.path.basename(path) == "mod.rs" else piece
			(values if value else methods if piece[-1].startswith("\t") else free).append("\n".join(piece))
		print(f"{name}: {path} → {arguments.target}")
	text = open(arguments.target).read() if os.path.exists(arguments.target) else NEW_FILE_HEADER
	if values:
		text = insert_after_header(text, values)
	free = [piece + "\n" for piece in free]
	methods = [piece + "\n" for piece in methods]
	if free:
		text = text.rstrip("\n") + "\n\n" + "\n".join(free)
	if methods:
		text = insert_methods(text, arguments.impl, "\n".join(methods))
	open(arguments.target, "w").write(text)
	for path, text in contents.items():
		open(path, "w").write(text)


if __name__ == "__main__":
	main()
