"""Folds `let Node::Symbol(x) = y.drop_meta() else { return None; };` into `let x = y.symbol_name()?;` (Node::symbol_name
looks through metadata the same way). Then `cargo check` names the `x.clone()` that now has to be `x.to_string()`.
Usage: python3 probes/cleanup/fold_symbol_else.py <files...>"""
import re, sys
let_else = re.compile(r"let Node::Symbol\((\w+)\) = ([\w.]+(?:\[\d+\])?)\.drop_meta\(\) else \{\s*return None;?\s*\};")
for path in sys.argv[1:]:
	text = open(path).read()
	changed, count = let_else.subn(lambda m: f"let {m.group(1)} = {m.group(2)}.symbol_name()?;", text)
	if count:
		open(path, "w").write(changed)
		print(path, count)
