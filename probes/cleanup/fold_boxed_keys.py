"""Folds `Node::Key(Box::new(a), op, Box::new(b))` into `key(a, op, b)` (lowering/nodes.rs), with a balanced-bracket
reader; a `use super::nodes::…` line gains `key`. Usage: python3 probes/cleanup/fold_boxed_keys.py <files...>"""
import re, sys

def closing(text, start):
	"""The index of the bracket closing the one at `start`, skipping strings and chars"""
	depth, index = 0, start
	while index < len(text):
		char = text[index]
		if char == '"':
			index += 1
			while text[index] != '"':
				index += 2 if text[index] == '\\' else 1
		elif char in "([{":
			depth += 1
		elif char in ")]}":
			depth -= 1
			if depth == 0:
				return index
		index += 1
	raise ValueError("unbalanced")

def arguments(text):
	"""The top-level comma-separated parts of an argument list"""
	parts, depth, current, index = [], 0, "", 0
	while index < len(text):
		char = text[index]
		if char == '"':
			end = index + 1
			while text[end] != '"':
				end += 2 if text[end] == '\\' else 1
			current += text[index:end + 1]
			index = end + 1
			continue
		if char in "([{": depth += 1
		if char in ")]}": depth -= 1
		if char == "," and depth == 0:
			parts.append(current.strip()); current = ""
		else:
			current += char
		index += 1
	if current.strip(): parts.append(current.strip())
	return parts

def boxed(part):
	if part.startswith("Box::new(") and closing(part, len("Box::new")) == len(part) - 1:
		return part[len("Box::new("):-1]

def fold(text):
	out, index, count = "", 0, 0
	while True:
		at = text.find("Node::Key(Box::new(", index)
		if at < 0:
			return out + text[index:], count
		end = closing(text, at + len("Node::Key"))
		parts = arguments(text[at + len("Node::Key("):end])
		inner = fold(text[at + len("Node::Key("):end])[0]
		parts = arguments(inner)
		if len(parts) == 3 and boxed(parts[0]) is not None and boxed(parts[2]) is not None:
			out += text[index:at] + f"key({boxed(parts[0])}, {parts[1]}, {boxed(parts[2])})"
			count += 1
		else:
			out += text[index:at] + "Node::Key(" + inner + ")"
		index = end + 1

for path in sys.argv[1:]:
	text = open(path).read()
	changed, count = fold(text)
	if not count:
		continue
	if not re.search(r"\bfn key\(", changed):
		imported = re.search(r"^use (super|crate::lowering)::nodes::(\{[^}]*\}|\w+);", changed, re.M)
		if imported:
			names = {name.strip() for name in imported.group(2).strip("{}").split(",") if name.strip()} | {"key"}
			ordered = sorted(n for n in names if n[0].islower()) + sorted(n for n in names if not n[0].islower())
			changed = changed[:imported.start()] + f"use {imported.group(1)}::nodes::{{{', '.join(ordered)}}};" + changed[imported.end():]
		else:
			first = re.search(r"^use ", changed, re.M)
			changed = changed[:first.start()] + "use crate::lowering::nodes::key;\n" + changed[first.start():]
	open(path, "w").write(changed)
	print(path, count)
