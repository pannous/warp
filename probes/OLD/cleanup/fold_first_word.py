"""Folds `matches!(xs.first().map(Node::drop_meta), Some(Node::Symbol(w)) if test)` into
`xs.first().and_then(Node::symbol_name).is_some_and(|w| test)`, and the guardless `Some(Node::Symbol(_))` into
`word_headed(xs).is_some()` (lowering/nodes.rs). Usage: python3 probes/cleanup/fold_first_word.py <files...>"""
import re, sys
sys.path.insert(0, __file__.rsplit("/", 1)[0])
from fold_boxed_keys import closing

head = re.compile(r"matches!\(([\w.\[\]&]+?)\.first\(\)\.map\(Node::drop_meta\), Some\(Node::Symbol\((\w+)\)\)( if )?")
for path in sys.argv[1:]:
	text = open(path).read()
	out, index, count = "", 0, 0
	while (match := head.search(text, index)):
		end = closing(text, match.start() + len("matches!"))
		items, word, guarded = match.groups()
		if guarded:
			test = text[match.end():end]
			replacement = f"{items}.first().and_then(Node::symbol_name).is_some_and(|{word}| {test})"
		elif match.end() == end:
			replacement = f"word_headed({items if items.startswith('&') else items}).is_some()"
		else:
			out += text[index:match.end()]; index = match.end(); continue
		out += text[index:match.start()] + replacement
		index, count = end + 1, count + 1
	out += text[index:]
	if count:
		if "word_headed(" in out and not re.search(r"use (super|crate::lowering)::nodes::\{?[^;]*\bword_headed\b", out) and "fn word_headed" not in out:
			imported = re.search(r"^use (super|crate::lowering)::nodes::(\{[^}]*\}|\w+);", out, re.M)
			if imported:
				names = {n.strip() for n in imported.group(2).strip("{}").split(",") if n.strip()} | {"word_headed"}
				ordered = sorted(n for n in names if n[0].islower()) + sorted(n for n in names if not n[0].islower())
				out = out[:imported.start()] + f"use {imported.group(1)}::nodes::{{{', '.join(ordered)}}};" + out[imported.end():]
			else:
				first = re.search(r"^use ", out, re.M)
				out = out[:first.start()] + "use crate::lowering::nodes::word_headed;\n" + out[first.start():]
		open(path, "w").write(out)
		print(path, count)
