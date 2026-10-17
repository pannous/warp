"""Folds the construction `Node::List(statements, Bracket::Curly, Separator::Semicolon)` into `block(statements)`
(lowering/nodes.rs); patterns (`=>`, `let`, `matches!`) stay. Usage: python3 probes/cleanup/fold_blocks.py <files...>"""
import re, sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from fold_boxed_keys import closing, arguments

def add_import(text, name):
	if re.search(rf"use (super|crate::lowering)::nodes::\{{?[^;]*\b{name}\b", text) or f"fn {name}(" in text:
		return text
	imported = re.search(r"^use (super|crate::lowering)::nodes::(\{[^}]*\}|\w+);", text, re.M)
	if imported:
		names = {n.strip() for n in imported.group(2).strip("{}").split(",") if n.strip()} | {name}
		ordered = sorted(n for n in names if n[0].islower()) + sorted(n for n in names if not n[0].islower())
		return text[:imported.start()] + f"use {imported.group(1)}::nodes::{{{', '.join(ordered)}}};" + text[imported.end():]
	first = re.search(r"^use ", text, re.M)
	return text[:first.start()] + f"use crate::lowering::nodes::{name};\n" + text[first.start():]

for path in sys.argv[1:]:
	text = open(path).read()
	out, index, count = "", 0, 0
	while (at := text.find("Node::List(", index)) >= 0:
		end = closing(text, at + len("Node::List"))
		parts = arguments(text[at + len("Node::List("):end])
		line_start = text.rfind("\n", 0, at)
		line_end = text.find("\n", end)
		prefix, suffix = text[line_start:at], text[end + 1:line_end]
		is_pattern = re.search(r"\b(let|if let|matches!\()\s*$|\blet\s+$", prefix) or suffix.lstrip().startswith(("=>", "=", "|", "if ")) and not suffix.lstrip().startswith("==")
		if len(parts) == 3 and parts[1] == "Bracket::Curly" and parts[2] == "Separator::Semicolon" and not is_pattern and "@" not in parts[0] and not re.fullmatch(r"\w+|_|\.\.", parts[0]):
			out += text[index:at] + f"block({parts[0]})"
			count += 1
		else:
			out += text[index:at + len("Node::List(")]
			end = at + len("Node::List(") - 1
		index = end + 1
	out += text[index:]
	if count:
		open(path, "w").write(add_import(out, "block"))
		print(path, count)
