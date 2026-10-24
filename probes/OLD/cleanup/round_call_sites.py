"""Sites that take a round-bracket call apart by hand: `let Node::List(items, Bracket::Round, …) = x.drop_meta() else`
followed within a few lines by `items.split_first()?` and `let Node::Symbol(name) = head.drop_meta() else`.
Usage: python3 probes/cleanup/round_call_sites.py <files...>"""
import sys, re
for path in sys.argv[1:]:
	lines = open(path).read().split('\n')
	for number, line in enumerate(lines):
		if 'split_first()' in line and re.search(r'let \(\w+, \w+\) = \w+\.split_first\(\)\?', line):
			window = '\n'.join(lines[max(0, number - 4):number + 4])
			if re.search(r'Node::Symbol\(\w+\) = \w+\.drop_meta\(\)', window):
				bracket = re.search(r'Node::List\(\w+, (Bracket::\w+|\w+|_)', window)
				print(f"{path}:{number + 1}: {bracket.group(1) if bracket else '?'}")
