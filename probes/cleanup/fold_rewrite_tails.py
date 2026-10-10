#!/usr/bin/env python3
"""Fold the spelled-out recursion tail of a rewrite into `children_rewritten` (notes/cleanup.md):

	Node::Key(left, op, right) => key(f(*left, a), op, f(*right, a)),
	Node::Meta { node, data } => Node::Meta { node: Box::new(f(*node, a)), data },
	other => other,
becomes
	other => children_rewritten(other, |child| f(child, a)),

only where an earlier arm of the same match takes every list (an unguarded `Node::List(…) =>`), so lists keep their
own handling and nothing new recurses. Usage: fold_rewrite_tails.py <file.rs>… (rewrites in place, prints each fold)
"""
import re
import sys

KEY_ARM = re.compile(r'^(\t+)Node::Key\((\w+), (\w+), (\w+)\) => key\((.+)\(\*\2(.*)\), \3, \5\(\*\4\6\)\),$')
META_ARM = r'^{indent}Node::Meta \{{ (\w+), (\w+) \}} => Node::Meta \{{ \1: Box::new\({call}\(\*\1{args}\)\), \2 \}},$'
CATCH_ALL = re.compile(r'^(\t+)(\w+) => \2,$')
TOTAL_LIST_ARM = re.compile(r'^\t+Node::List\(\w+, \w+, \w+\) =>')
HELPER = 'children_rewritten'


def match_start(lines, index, indent):
	"""The line of the `match` whose arms are at `indent`"""
	outer = indent[:-1]
	for line_index in range(index, -1, -1):
		line = lines[line_index]
		if line.startswith(outer) and not line.startswith(indent) and 'match ' in line:
			return line_index
	return None


def fold(lines):
	folded = []
	index = 0
	while index + 2 < len(lines):
		key_arm = KEY_ARM.match(lines[index])
		if key_arm:
			indent, _, _, _, call, args = key_arm.groups()
			meta_arm = re.match(META_ARM.format(indent=indent, call=re.escape(call), args=re.escape(args)), lines[index + 1])
			catch_all = CATCH_ALL.match(lines[index + 2])
			start = match_start(lines, index, indent)
			arms = lines[start:index] if start is not None else []
			takes_lists = any(TOTAL_LIST_ARM.match(line) and line.startswith(indent + 'Node::List') for line in arms)
			if meta_arm and catch_all and catch_all.group(1) == indent and takes_lists:
				rewrite = call if not args and '.' not in call else f'|child| {call}(child{args})'
				lines[index:index + 3] = [f'{indent}other => {HELPER}(other, {rewrite}),']
				folded.append((index + 1, call))
		index += 1
	return folded


for path in sys.argv[1:]:
	lines = open(path).read().split('\n')
	folded = fold(lines)
	if folded:
		open(path, 'w').write('\n'.join(lines))
		for line, call in folded:
			print(f'{path}:{line}: {call}')
