#!/usr/bin/env python3
"""Pack the emitter's instruction arrays into rows, one logical step per row, read like WAT (card emit-rows, user
decision 2026-10-14). rustfmt lays every `I::…` element on its own line; the packed statement carries
`#[rustfmt::skip]` so it stays packed. Only whitespace changes: --check compares each file's tokens with git HEAD.

A row ends after an instruction consuming the stack (LocalSet, BrIf, stores …) and after the opener of a block
(`cond, I::If(…)` reads as WAT's `(if cond`); the rows inside Block/Loop/If/TryTable indent one tab deeper, Else and
End stand alone at the outer depth.

Usage: emit_rows.py [--write] files…   (prints the lines saved per file)
       emit_rows.py --check files…     (fails on any non-whitespace difference to git HEAD)
"""
import re
import subprocess
import sys

MAX_WIDTH = 120
TAB_WIDTH = 4
MIN_ELEMENTS = 4
SKIP = "#[rustfmt::skip]"
INSTRUCTION = re.compile(r"^(I::|Instruction::|I32Const\(|I64Const\(|F64Const\(|field\()")
STEP_END = re.compile(r"^I::(\w*Store\w*|LocalSet|GlobalSet|Drop|Br|BrIf|BrTable|BrOn\w*|Return|Unreachable|Throw|"
	r"StructSet|ArraySet|ArrayCopy|ArrayFill|MemoryCopy|MemoryFill)\b")
OPENER = re.compile(r"^I::(Block|Loop|If|TryTable)\(")
CLOSER = re.compile(r"^I::(Else|End)$")
STATEMENT_BEFORE = (";", "{", "}")


def indent_of(line):
	return len(line) - len(line.lstrip("\t"))


def balanced(text):
	depth = 0
	for character in text:
		depth += character in "([{"
		depth -= character in ")]}"
		if depth < 0:
			return False
	return depth == 0


def element_of(line):
	"""The array element on a line of its own: `I::LocalGet(0),` → `I::LocalGet(0)`"""
	text = line.strip()
	if not text.endswith(",") or "//" in text or not balanced(text[:-1]):
		return None
	return text[:-1]


def joined(pieces):
	"""Lines of one statement on one line: `Self::emit_list(` `func,` `&[` → `Self::emit_list(func, &[`"""
	text = ""
	for piece in pieces:
		text += (" " if text.endswith(",") else "") + piece
	return re.sub(r",\s*([)\]])", r"\1", text)


def statement_start(lines, opening):
	"""The line starting the statement whose array opens on line `opening`, or None"""
	for start in range(opening, max(opening - 6, 0), -1):
		if "//" in lines[start] or SKIP in lines[start - 1]:
			return None
		before = lines[start - 1].strip()
		if indent_of(lines[start]) <= min(indent_of(line) for line in lines[start:opening + 1]) and (
				before.endswith(STATEMENT_BEFORE) or before.startswith("//") or not before):
			return start
	return None


def statement_end(lines, closing, indent):
	"""The line ending the statement after the array's `]` line: `);` at the statement's indent, or None"""
	for end in range(closing, min(closing + 4, len(lines))):
		text = lines[end].rstrip()
		if text.endswith(";") and indent_of(lines[end]) == indent and re.fullmatch(r"\t*[\]),;]+", text):
			return end
		if not re.fullmatch(r"\t*[\]),]+", text):
			return None
	return None


def rows(elements, indent):
	"""The elements in rows: a step per row, wrapped at MAX_WIDTH, nested blocks one tab deeper"""
	result, row, depth = [], [], 0

	def flush(at_depth):
		if row:
			result.append((at_depth, list(row)))
			row.clear()

	for element in elements:
		if CLOSER.match(element):
			flush(depth)
			depth = max(depth - 1, 0)
			if element == "I::End" and result and result[-1][1][-1] == "I::End" and result[-1][0] == depth + 1:
				result[-1] = (depth, result[-1][1] + [element])
			else:
				result.append((depth, [element]))
			if element == "I::Else":
				depth += 1
			continue
		width = (indent + 1 + depth) * TAB_WIDTH + len(", ".join(row + [element])) + 1
		if row and width > MAX_WIDTH:
			flush(depth)
		row.append(element)
		if OPENER.match(element):
			flush(depth)
			depth += 1
		elif STEP_END.match(element):
			flush(depth)
	flush(depth)
	return ["\t" * (indent + 1 + depth) + ", ".join(items) + "," for depth, items in result]


def packed(lines):
	"""The file's lines with each instruction-array statement packed; the number of lines saved"""
	output, index, saved = [], 0, 0
	while index < len(lines):
		line = lines[index]
		end = None
		if line.rstrip().endswith("[") and index + 1 < len(lines):
			elements, closing = [], index + 1
			while closing < len(lines) and (element := element_of(lines[closing])) is not None:
				elements.append(element)
				closing += 1
			start = statement_start(lines, index)
			if (len(elements) >= MIN_ELEMENTS and all(INSTRUCTION.match(element) for element in elements) and start is not None
					and closing < len(lines) and lines[closing].strip().startswith("]")):
				indent = indent_of(lines[start])
				end = statement_end(lines, closing, indent)
		if end is None:
			output.append(line)
			index += 1
			continue
		head_lines = output[len(output) - (index - start):] if index > start else []
		del output[len(output) - len(head_lines):]
		prefix = "\t" * indent
		head = joined([piece.strip() for piece in head_lines + [line]])
		tail = joined([piece.strip() for piece in lines[closing:end + 1]])
		statement = [prefix + SKIP, prefix + head] + rows(elements, indent) + [prefix + tail]
		saved += (end + 1 - start) - len(statement)
		output.extend(statement)
		index = end + 1
	return output, saved


def tokens(text):
	"""The text without whitespace, skip attributes and trailing commas: equal for a pure re-layout"""
	text = re.sub(r"\s+", "", text.replace(SKIP, ""))
	return re.sub(r",([)\]}])", r"\1", text)


def main():
	arguments = sys.argv[1:]
	paths = [argument for argument in arguments if not argument.startswith("--")]
	if "--check" in arguments:
		changed = [path for path in paths if tokens(open(path).read()) != tokens(
			subprocess.run(["git", "show", f"HEAD:{path}"], capture_output=True, text=True).stdout)]
		print("\n".join(f"tokens differ: {path}" for path in changed) or f"{len(paths)} files: same tokens")
		sys.exit(1 if changed else 0)
	total = 0
	for path in paths:
		lines = open(path).read().split("\n")
		output, saved = packed(lines)
		if saved > 0:
			total += saved
			print(f"{saved:5} {path}")
			if "--write" in arguments:
				open(path, "w").write("\n".join(output))
	print(f"{total:5} lines saved")


if __name__ == "__main__":
	main()
