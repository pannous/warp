"""Folds `let mut found = false; x.visit(&mut |p| found |= test); found` into `any_part(x, |p| test)`
(lowering/nodes.rs). Only the one-statement closure form; block closures are left for a look by hand.
Usage: python3 probes/cleanup/fold_any_part.py <files...>"""
import re, sys
one_line = re.compile(r"let mut found = false;\n(\s*)(\w+)\.visit\(&mut \|(\w+)\| found \|= (.*?)\);\n\s*found\n", re.S)
for path in sys.argv[1:]:
	text = open(path).read()
	changed = one_line.sub(lambda m: f"any_part({m.group(2)}, |{m.group(3)}| {m.group(4)})\n", text)
	if changed != text:
		open(path, "w").write(changed)
		print(path, len(one_line.findall(text)))

# The block closures: `x.visit(&mut |p| { found |= test });` and `x.visit(&mut |p| { if let P = s { found |= test; } });`
block = re.compile(r"let mut found = false;\n(\s*)([\w.()]+)\.visit\(&mut \|(\w+)\| \{\n\s*found \|=\s*([^;{}]*?);?\n\s*\}\);\n\s*found\n")
if_let = re.compile(r"let mut found = false;\n(\s*)([\w.()]+)\.visit\(&mut \|(\w+)\| \{\n\s*if let (.*?) = ([\w.()]+) \{\n\s*found \|= ([^;{}]*?);?\n\s*\}\n\s*\}\);\n\s*found\n")
def collapse(code):
	return re.sub(r"\s*\n\s*", " ", code).strip()
for path in sys.argv[1:]:
	text = open(path).read()
	changed = block.sub(lambda m: f"any_part(&{m.group(2)}, |{m.group(3)}| {collapse(m.group(4))})\n", text)
	changed = if_let.sub(lambda m: f"any_part(&{m.group(2)}, |{m.group(3)}| matches!({m.group(5)}, {m.group(4)} if {collapse(m.group(6))}))\n", changed)
	if changed != text:
		open(path, "w").write(changed)
		print(path, "block forms")
