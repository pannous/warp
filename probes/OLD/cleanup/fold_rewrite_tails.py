#!/usr/bin/env python3
"""Fold the spelled-out recursion tail of a rewrite into `children_rewritten` (notes/cleanup.md):

	Node::Key(left, op, right) => key(f(*left, a), op, f(*right, a)),
	Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| f(item, a)).collect(), bracket, separator),
	Node::Meta { node, data } => Node::Meta { node: Box::new(f(*node, a)), data },
	other => other,
becomes
	other => children_rewritten(other, |child| f(child, a)),

The generic Meta arm is the anchor; Key and List each need either such a generic arm (removed) or an unguarded arm of
their own elsewhere in the match (kept), so no node recurses that did not before. Single-line arms only.
A match left with only the catch-all arm (clippy match_single_binding) becomes the call itself, by hand.
Usage: fold_rewrite_tails.py <file.rs>… (rewrites in place, prints each fold)
"""
import re
import sys

META_ARM = re.compile(r'^(\t+)Node::Meta \{ node, data \} => Node::Meta \{ node: Box::new\((.+?)\(\*node(.*)\)\), data \},$')
CATCH_ALL = '{indent}other => other,'
TOTAL_ARM = r'^{indent}Node::{variant}\((?:\w+, ){{2}}\w+\) =>(?! .* if )'
HELPER = 'children_rewritten'


def generic_arms(indent, call, args):
	"""Each variant's spellings of 'rewrite every child with call'"""
	item_call = call if not args and '.' not in call else f'|item| {call}(item{args})'
	return {
		'Key': {
			f'{indent}Node::Key(left, op, right) => key({call}(*left{args}), op, {call}(*right{args})),',
			f'{indent}Node::Key(left, op, right) => Node::Key(Box::new({call}(*left{args})), op, Box::new({call}(*right{args}))),',
		},
		'List': {f'{indent}Node::List(items, bracket, separator) => Node::List(items.into_iter().map({item_call}).collect(), bracket, separator),'},
	}


def match_start(lines, index, indent):
	"""The line of the `match` whose arms are at `indent`"""
	outer = indent[:-1]
	for line_index in range(index, -1, -1):
		line = lines[line_index]
		if line.startswith(outer) and not line.startswith(indent):
			return line_index if 'match ' in line else None
	return None


def fold_at(lines, index):
	"""Fold the match whose generic Meta arm is lines[index]: the lines left, or None"""
	indent, call, args = META_ARM.match(lines[index]).groups()
	if index + 1 >= len(lines) or lines[index + 1] != CATCH_ALL.format(indent=indent):
		return None
	start = match_start(lines, index, indent)
	if start is None or any(f'fn {HELPER}(' in line for line in lines[max(start - 3, 0):start + 1]):
		return None
	arms = range(start + 1, index)
	removed = {index}
	for variant, spellings in generic_arms(indent, call, args).items():
		generic = [line for line in arms if lines[line] in spellings]
		total = re.compile(TOTAL_ARM.format(indent=indent, variant=variant))
		if generic:
			removed.update(generic)
		elif not any(total.match(lines[line]) for line in arms):
			return None
	rewrite = call if not args and '.' not in call else f'|child| {call}(child{args})'
	lines[index + 1] = f'{indent}other => {HELPER}(other, {rewrite}),'
	return [line for number, line in enumerate(lines) if number not in removed]


def fold(lines):
	folded = []
	index = 0
	while index < len(lines):
		meta_arm = META_ARM.match(lines[index])
		result = fold_at(lines, index) if meta_arm else None
		if result is not None:
			folded.append((index + 1, meta_arm.group(2)))
			removed_before = len(lines) - len(result) - 1
			lines[:] = result
			index -= removed_before
		index += 1
	return folded


for path in sys.argv[1:]:
	lines = open(path).read().split('\n')
	folded = fold(lines)
	if folded:
		open(path, 'w').write('\n'.join(lines))
		for line, call in folded:
			print(f'{path}:{line}: {call}')
