"""Joins adjacent instruction emissions in the wasm emitter (card cleanup-sweep-emit): back-to-back
`Self::emit_list(func, &[…]);` and `func.instruction(&…);` statements on the same function become one emit_list, which
emits the same instructions in the same order. Usage: python3 probes/cleanup/join_emit_lists.py [--write] files…"""
import re
import sys

write = "--write" in sys.argv
paths = [arg for arg in sys.argv[1:] if arg != "--write"]
LIST_CALL = re.compile(r"Self::emit_list\(\s*(\w+),\s*&\[")
SINGLE = re.compile(r"(\w+)\.instruction\(&")


def closing(text, start, opening, close):
	"""index just after the bracket matching text[start] == opening"""
	depth, index = 0, start
	while index < len(text):
		char = text[index]
		if char == '"':
			index = text.index('"', index + 1)
		elif char == opening:
			depth += 1
		elif char == close:
			depth -= 1
			if depth == 0:
				return index + 1
		index += 1
	raise ValueError("unbalanced")


def statement_at(text, position):
	"""(function variable, [instruction texts], end) of an emission statement starting at position, else None"""
	if m := LIST_CALL.match(text, position):
		bracket = m.end() - 1
		end = closing(text, bracket, "[", "]")
		tail = re.match(r",?\s*\);", text[end:])
		if not tail:
			return None
		return m.group(1), split_items(text[bracket + 1 : end - 1]), end + tail.end()
	if m := SINGLE.match(text, position):
		paren = m.end() - 2
		end = closing(text, paren, "(", ")")
		if not text[end:].startswith(";"):
			return None
		return m.group(1), [text[m.end() : end - 1].strip()], end + 1
	return None


def split_items(inner):
	items, depth, current, index = [], 0, "", 0
	while index < len(inner):
		char = inner[index]
		if char == '"':
			stop = inner.index('"', index + 1) + 1
			current += inner[index:stop]
			index = stop
			continue
		if char in "([{":
			depth += 1
		elif char in ")]}":
			depth -= 1
		if char == "," and depth == 0:
			items.append(current.strip())
			current = ""
		else:
			current += char
		index += 1
	if current.strip():
		items.append(current.strip())
	return [" ".join(item.split()) for item in items]


def joined(text):
	out, index, joins = [], 0, 0
	starts = re.compile(r"(?m)^([ \t]*)(?=Self::emit_list\(|\w+\.instruction\(&)")
	while m := starts.search(text, index):
		out.append(text[index : m.start()])
		indent, position = m.group(1), m.end()
		run = []
		while (found := statement_at(text, position)) and (not run or found[0] == run[0][0]):
			run.append(found)
			after = re.match(r"[ \t]*\n[ \t]*", text[found[2]:])
			if not after or after.group(0).count("\n") != 1:
				position = found[2]
				break
			position = found[2] + after.end()
		if len(run) < 2:
			end = run[0][2] if run else m.end() + 1
			out.append(text[m.start() : end])
			index = end
			continue
		last_end = run[-1][2]
		items = [item for statement in run for item in statement[1]]
		out.append(f"{indent}Self::emit_list({run[0][0]}, &[{', '.join(items)}]);")
		joins += len(run) - 1
		index = last_end
	out.append(text[index:])
	return "".join(out), joins


total = 0
for path in paths:
	text = open(path).read()
	new, joins = joined(text)
	total += joins
	if joins:
		print(f"{joins:4} {path}")
	if write and joins:
		open(path, "w").write(new)
print(f"{total} joins")
