# The core parameters typed Node or any whose body already knows their shape (card beautiful-types):
# run from the repo root, `python3 probes/types/signature_shapes.py`; "match-only" parameters with a `_` arm stay Node

import re, glob, collections

SIGNATURE = re.compile(r'^(?:def\s+)?([a-z_]\w*)\(([^()]*)\)\s*(?::\s*([\w? ]+?))?\s*(:=|\{)(.*)$')
KEY_READERS = {'key_of', 'value_of', 'op_of'}
LIST_READERS = {'items_of', 'bracket_of', 'separator_of'}
counts = collections.Counter()
examples = collections.defaultdict(list)


def body_of(lines, start, rest):
	body = [rest]
	j = start + 1
	while j < len(lines) and (lines[j].startswith('\t') or lines[j] == ''):
		body.append(lines[j])
		j += 1
	return '\n'.join(body)


def shape(parameter, body):
	arms = re.findall(r'^\t(\w+)\(', body, re.M) if re.search(r'match ' + re.escape(parameter) + r' \{', body) else []
	cases = {arm for arm in arms if arm in ('Key', 'List', 'Symbol', 'Text', 'Number')}
	readers = set(re.findall(r'\b(\w+_of)\(' + re.escape(parameter) + r'\)', body))
	if len(cases) == 1:
		return 'match-only ' + cases.pop()
	if readers and readers <= KEY_READERS:
		return 'key readers'
	if readers and readers <= LIST_READERS:
		return 'list readers'
	if readers == {'name_of'}:
		return 'name reader'
	return None


for path in sorted(glob.glob('core/*.warp')):
	lines = open(path).read().split('\n')
	for i, line in enumerate(lines):
		match = SIGNATURE.match(line)
		if not match:
			continue
		name, parameters, _, _, rest = match.groups()
		body = body_of(lines, i, rest)
		for parameter in parameters.split(','):
			if ':' not in parameter:
				continue
			parameter_name, written = [part.strip() for part in parameter.split(':', 1)]
			if written not in ('Node', 'any'):
				continue
			found = shape(parameter_name, body)
			if found:
				counts[found] += 1
				examples[found].append(f"{path}:{i + 1} {name}({parameter_name})")
for found, count in counts.items():
	print(found, count, examples[found][:4])
