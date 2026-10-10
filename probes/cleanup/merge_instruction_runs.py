#!/usr/bin/env python3
"""Merge runs of consecutive `f.instruction(&I::…);` lines (same indent, same function variable, no comment) into one
`Self::emit_list(f, &[…]);`, wrapped at MAX_WIDTH. The emitted bytes stay the same: emit_list emits the items in order.
Usage: merge_instruction_runs.py <file.rs>…  (rewrites in place, prints the merged run count per file)"""
import re
import sys

MAX_WIDTH = 140
MIN_RUN = 2
SINGLE = re.compile(r'^(\t+)(f|func)\.instruction\(&((?:I::|I32Const\().*)\);$')


def balanced(expression):
	depth = 0
	for character in expression:
		depth += character in '([{'
		depth -= character in ')]}'
		if depth < 0:
			return False
	return depth == 0


def instruction_of(line):
	match = SINGLE.match(line.rstrip('\n'))
	if not match or '//' in line or not balanced(match.group(3)):
		return None
	return match.group(1), match.group(2), match.group(3)


def merged(indent, function, items):
	one_line = f"{indent}Self::emit_list({function}, &[{', '.join(items)}]);"
	if len(one_line.expandtabs(4)) <= MAX_WIDTH:
		return [one_line]
	lines, current = [f"{indent}Self::emit_list({function}, &["], []
	for item in items:
		candidate = ', '.join(current + [item]) + ','
		if current and len((indent + '\t' + candidate).expandtabs(4)) > MAX_WIDTH:
			lines.append(indent + '\t' + ', '.join(current) + ',')
			current = []
		current.append(item)
	lines.append(indent + '\t' + ', '.join(current) + ',')
	lines.append(f"{indent}]);")
	return lines


def ends_a_return(source, run_start, position):
	"""`return …; end` closes a case: what follows starts a new run"""
	if position - run_start < 2:
		return False
	return [instruction_of(line)[2] for line in source[position - 2:position]] == ['I::Return', 'I::End']


def merge_file(path):
	source = open(path).read().split('\n')
	result, index, runs = [], 0, 0
	while index < len(source):
		first = instruction_of(source[index])
		run_end = index + 1
		while first and run_end < len(source):
			following = instruction_of(source[run_end])
			if not following or following[:2] != first[:2] or ends_a_return(source, index, run_end):
				break
			run_end += 1
		if first and run_end - index >= MIN_RUN:
			result.extend(merged(first[0], first[1], [instruction_of(line)[2] for line in source[index:run_end]]))
			runs += 1
		else:
			result.extend(source[index:run_end])
		index = run_end
	open(path, 'w').write('\n'.join(result))
	return runs


if __name__ == '__main__':
	for path in sys.argv[1:]:
		print(path, merge_file(path))
