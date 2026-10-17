"""Folds the hand-written `let (a, b) = items.split_first()?; let Node::Symbol(a) = a.drop_meta() else { return None };`
into `nodes::word_headed(items)`, and a preceding `let Node::List(items, Bracket::Round, _) = x.drop_meta() else {…}`
into `nodes::round_call(x)`. Usage: python3 probes/cleanup/fold_word_headed.py <files...>; then cargo check for the
`name.clone()` that now needs `name.to_string()` and the `use` line."""
import re, sys
pair = re.compile(r"let \((\w+), (\w+)\) = (\w+)\.split_first\(\)\?;\n(\s*)let Node::Symbol\((\w+)\) = (\w+)\.drop_meta\(\) else \{\n\s*return None;\n\s*\};")
round_list = re.compile(r"let Node::List\((\w+), Bracket::Round, _\) = (\w+)\.drop_meta\(\) else \{\n\s*return None;\n\s*\};\n\s*let \((\w+), (\w+)\) = word_headed\(\1\)\?;")
for path in sys.argv[1:]:
	text = open(path).read()
	def folded(match):
		head, rest, items, indent, name, taken = match.groups()
		return f"let ({name}, {rest}) = word_headed({items})?;" if head == taken else match.group(0)
	changed = pair.sub(folded, text)
	changed = round_list.sub(lambda m: f"let ({m.group(3)}, {m.group(4)}) = round_call({m.group(2)})?;", changed)
	if changed != text:
		open(path, "w").write(changed)
		print(path)
