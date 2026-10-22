# lib/ functions that rely on ø surviving a `list` declaration (decision optional-result: ø reaching `list` becomes []):
# a `list` parameter tested against ø, or a `: list` function answering ø. usage: python3 list_nothing_tests.py <lib>
import re, sys, pathlib

HEAD = re.compile(r'^(?:def\s+)?([a-z_][\w]*)\(([^()]*(?:\([^()]*\)[^()]*)*)\)\s*:?\s*([\w?\[\]]*)\s*(?::=|\{)')
NOTHING_TEST = r'(?:\b{0}\s*(?:==|!=|is not|is)\s*ø|ø\s*(?:==|!=)\s*{0}\b)'

def bodies(lines):
	"""(line number, name, parameters, result, body text) of each top-level function: its lines until the next unindented one"""
	for i, line in enumerate(lines):
		head = HEAD.match(line)
		if not head: continue
		end = i + 1
		while end < len(lines) and (lines[end].startswith(('\t', ' ')) or lines[end].strip() in ('}', '')): end += 1
		yield i + 1, head.group(1), head.group(2), head.group(3), '\n'.join([line[head.end():]] + lines[i + 1:end])

for path in sorted(pathlib.Path(sys.argv[1]).rglob('*.warp')):
	lines = path.read_text().split('\n')
	for number, name, parameters, result, body in bodies(lines):
		listed = re.findall(r'(\w+)\s*:\s*list\b(?!\?)', parameters)
		for parameter in listed:
			if re.search(NOTHING_TEST.format(re.escape(parameter)), body):
				print(f'{path}:{number}: parameter {parameter}: list of {name} is tested against ø')
		if result == 'list' and re.search(r'return\s+ø|else\s+ø|^\s*ø\s*$|:\s*ø\s*$', body, re.M):
			print(f'{path}:{number}: {name}: list can answer ø')
