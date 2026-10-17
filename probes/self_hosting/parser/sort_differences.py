#!/usr/bin/env python3
"""Sorts data/self_parser/differences.txt (test parser_differences) into kinds of forms by the Rust tree where the two
first differ: the keep/drop survey of card self-parser (decision self-hosting-minimal)"""
import collections, re, sys

DIFFERENCES = "data/self_parser/differences.txt"
# kind: (pattern in the Rust text around the difference, or in the path), first match wins
KINDS = [
	("global declarations", r'"global"'),
	("text holes ${…}", r'\$\{'),
	("durations and units (1 day, 5 km)", r'\["data" |\["key" \'\*\' [0-9.]+ \["symbol"'),
	("optional types Team?", r'"symbol" "[A-Za-z_]+\?"|\["type" \["symbol" "[A-Za-z]+\?"'),
	("try … else", r'try·'),
	("event handlers on click {…}", r'"on"\]|"click"'),
	("shader/wgsl blocks", r'shader|wgsl|@fragment'),
	("user operators ↑ ↓", r'operator_'),
	("slices", r'"slice"'),
	("if … then … else chains", r'"else" \["key" "then"|"then" \["key" "if"|\["key" "if" ø'),
	("do … while, indented bodies", r'"do" \["key" "while"|indent/|"while"'),
	("char literals", r'\["char" '),
	("ruby blocks |x|", r'"->" \["symbol"'),
	("dollar names $a", r'"\$[a-z]'),
	("enum declarations", r'"enum"'),
	("python/ruby/foreign samples", r'python|ruby|samples/r\.warp|samples/b\.warp'),
	("bitwise &", r"'&'"),
	("phrase definitions to get …", r'"to"\]|"infix"|"get"'),
	("go blocks", r'"go"\]'),
	("instance literals Person {…}", r'\["key" "" \["symbol" "[A-Z]'),
	("error nodes for malformed input", r'\["error" '),
	("rust/python reading (println!, generators, yield from)", r'rust_reading|generators/|"println"'),
	("markup and css", r'markup/|"style"'),
	("named constants π", r'\["number" '),
	("casts as float", r'\["key" "as"'),
	("spread *xs", r'"\*[a-z]'),
	("for each x in", r'"each"|"for"\] \["symbol" "[a-z]+"\] \["symbol" "in"'),
	("data(…) and print(…) word calls", r'"data"\]|\["symbol" "print"\]'),
	("statement grouping", r'^.{0,32}\["list" "" \' \' |\["list" "" \' \' \["list" "" \' \''),
]

def entries(text):
	lines = text.split("\n")
	for at in range(0, len(lines) - 2):
		if lines[at] and not lines[at].startswith("  ") and lines[at + 1].startswith("  warp:"):
			yield lines[at], lines[at + 1][9:], lines[at + 2][9:]

def first_difference(a, b):
	n = 0
	while n < min(len(a), len(b)) and a[n] == b[n]:
		n += 1
	return n

kinds = collections.defaultdict(list)
for path, warp, rust in entries(open(DIFFERENCES).read()):
	n = first_difference(warp, rust)
	around = rust[max(0, n - 30):n + 60]
	kind = next((name for name, pattern in KINDS if re.search(pattern, around) or re.search(pattern, path)), "other")
	kinds[kind].append((path, warp[max(0, n - 30):n + 60], around))
for kind, found in sorted(kinds.items(), key=lambda item: -len(item[1])):
	print(f"{len(found):4} {kind}")
	if "-v" in sys.argv or kind == "other" and "-o" in sys.argv:
		for path, warp, rust in found:
			print(f"       {path}\n         w: {warp}\n         r: {rust}".replace("\n'", "⏎'"))
