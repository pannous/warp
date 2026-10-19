#!/usr/bin/env python3
"""Style rules that rustfmt and clippy don't cover, found in reviews (card style-linter, notes/style_lint.md).

style_lint.py [--fix] [--base REF] [path...]

Checks the .rs files changed since the merge base of HEAD with --base (default origin/main), or the given paths.
Each violation prints as `path:line: [rule] message`; the exit code is 1 while any is left. --fix repairs what it can
and reports the rest.

Rules (each approved by the user, 2026-10-15):
  doc-moved  a /// line that documents item X at the base now sits on a new item Y: Y was inserted between X and
             its doc. --fix moves the line back onto X when X has no doc of its own now.
"""
import argparse
import os
import re
import subprocess
import sys
from collections import Counter

DEFAULT_BASE = "origin/main"
SCOPE_KEYWORDS = re.compile(r"\b(mod|impl|trait)\b|\bextern\s+\"\"\s*$")
ITEM_NAME = re.compile(r"\bfn\s+(\w+)|\b(?:const|static|struct|enum|trait|type|mod|union)\s+(?:mut\s+)?(\w+)|\bmacro_rules!\s*(\w+)")
ENDS_AT_SEMICOLON = re.compile(r"^(pub(\([^)]*\))?\s+)?((const|static)\s+(?!fn\b)|use\b|type\b)")
ATTRIBUTE = re.compile(r"#!?(\[[^\]]*\])?\s*")
MIN_DOC_LINE_LENGTH = 12  # shorter doc lines ("/// # Panics") repeat too often to tell whose they are


class Line:
	def __init__(self, text):
		self.text = text
		self.code = ""  # the line without comments, string contents blanked

	@property
	def stripped(self):
		return self.text.strip()

	@property
	def is_outer_doc(self):
		return self.stripped.startswith("///") and not self.stripped.startswith("////")

	@property
	def is_comment_only(self):
		return bool(self.stripped) and not self.code.strip()


def strip_code(text):
	"""The lines of a source with their code part: comments removed, string and char literals blanked"""
	lines = [Line(t) for t in text.split("\n")]
	code = [[] for _ in lines]
	i, n, row, block_depth = 0, len(text), 0, 0
	while i < n:
		c = text[i]
		if c == "\n":
			row, i = row + 1, i + 1
		elif block_depth:
			if text.startswith("*/", i):
				block_depth, i = block_depth - 1, i + 2
			elif text.startswith("/*", i):
				block_depth, i = block_depth + 1, i + 2
			else:
				i += 1
		elif text.startswith("//", i):
			end = text.find("\n", i)
			i = n if end < 0 else end
		elif text.startswith("/*", i):
			block_depth, i = 1, i + 2
		elif (raw := re.match(r'b?r(#*)"', text[i:i + 300])) and not (i and (text[i - 1].isalnum() or text[i - 1] == "_")):
			closing = '"' + raw.group(1)
			end = text.find(closing, i + raw.end())
			end = n if end < 0 else end + len(closing)
			row = blank_literal(text, i, end, row, code)
			i = end
		elif c == '"':
			end = i + 1
			while end < n and text[end] != '"':
				end += 2 if text[end] == "\\" else 1
			row = blank_literal(text, i, end + 1, row, code)
			i = end + 1
		elif c == "'" and (char := re.match(r"'(\\(u\{[0-9a-fA-F]+\}|x..|.)|[^\\'\n])'", text[i:i + 12])):
			code[row].append("' '")
			i += char.end()
		else:
			code[row].append(c)
			i += 1
	for line, parts in zip(lines, code):
		line.code = "".join(parts)
	return lines


def blank_literal(text, start, end, row, code):
	"""Blanks a string literal in the code view, keeping its line breaks; returns the row it ends on"""
	code[row].append('"')
	row += text.count("\n", start, end)
	code[row].append('"')
	return row


class Item:
	def __init__(self, scope, start):
		self.scope = scope  # the names of the enclosing mods and impls
		self.start = start  # its first code line, attributes included
		self.header = ""

	@property
	def keyword_line(self):
		return ATTRIBUTE.sub("", self.header).strip()

	@property
	def name(self):
		if self.keyword_line.startswith("impl"):
			return " ".join(self.keyword_line.split()).rstrip("{ ")
		found = ITEM_NAME.search(self.keyword_line)
		return next(g for g in found.groups() if g) if found else self.keyword_line.split("(")[0]

	@property
	def identity(self):
		return (self.scope, self.name)


def parse_items(lines):
	"""The items of every module-like scope (file, mod, impl, trait, extern block) in order, with their scope path"""
	items = []
	stack = [{"items": True, "path": "", "item": None, "owner": None}]  # one entry per open brace
	paren_depth = 0
	for row, line in enumerate(lines):
		for c in line.code:
			scope = stack[-1]
			if scope["items"] and scope["item"] is None and not c.isspace() and c not in "};,":
				scope["item"] = Item(scope["path"], row)
				items.append(scope["item"])
			item = scope["item"] if scope["items"] else None
			if item is not None and c != "{":
				item.header += c
			if c in "([":
				paren_depth += 1
			elif c in ")]":
				paren_depth = max(0, paren_depth - 1)
			elif c == "{" and paren_depth == 0:
				opens_scope = item is not None and SCOPE_KEYWORDS.search(item.keyword_line) and "fn " not in item.keyword_line
				path = scope["path"] + "/" + item.name if opens_scope else scope["path"]
				if item is not None:
					item.header += "{"
				stack.append({"items": bool(opens_scope), "path": path, "item": None, "owner": item})
			elif c == "}" and paren_depth == 0 and len(stack) > 1:
				owner = stack.pop()["owner"]
				if owner is not None and owner is stack[-1]["item"] and not ENDS_AT_SEMICOLON.match(owner.keyword_line):
					stack[-1]["item"] = None
			elif c == ";" and paren_depth == 0 and item is not None:
				scope["item"] = None
		if stack[-1]["item"] is not None:
			stack[-1]["item"].header += " "
	return items


def doc_rows(lines, item):
	"""The /// lines in the docs and comments right above an item"""
	row = item.start
	while row > 0 and lines[row - 1].is_comment_only:
		row -= 1
	return [r for r in range(row, item.start) if lines[r].is_outer_doc]


class Finding:
	def __init__(self, path, row, rule, message, fix=None):
		self.path, self.row, self.rule, self.message, self.fix = path, row, rule, message, fix

	def __str__(self):
		return f"{self.path}:{self.row + 1}: [{self.rule}] {self.message}"


def base_doc_owners(base_text):
	"""Each doc line of the base that occurs once there, to the identity of the item it documents; and all identities"""
	lines = strip_code(base_text)
	counts = Counter(line.stripped for line in lines if line.is_outer_doc)
	owners = {}
	items = parse_items(lines)
	for item in items:
		for r in doc_rows(lines, item):
			text = lines[r].stripped
			if counts[text] == 1 and len(text) >= MIN_DOC_LINE_LENGTH:
				owners[text] = item.identity
	return owners, {item.identity for item in items}


def check_moved_docs(path, lines, items, base_text, base_name):
	owner_of, base_identities = base_doc_owners(base_text)
	identity_counts = Counter(item.identity for item in items)
	current = {item.identity: item for item in items}
	findings = []
	for item in items:
		if item.identity in base_identities:
			continue  # only a new item takes another's doc by being inserted; an old one may get its doc back
		moved = [r for r in doc_rows(lines, item) if owner_of.get(lines[r].stripped, item.identity) != item.identity]
		for owner_identity in dict.fromkeys(owner_of[lines[r].stripped] for r in moved):
			owner = current.get(owner_identity)
			if owner is None or identity_counts[owner_identity] > 1:
				continue  # X is gone or ambiguous: the doc may have been meant for Y
			rows = [r for r in moved if owner_of[lines[r].stripped] == owner_identity]
			fix = (rows, owner.start) if not doc_rows(lines, owner) else None
			findings.append(Finding(path, rows[0], "doc-moved",
				f"this doc line documents `{owner.name}` (line {owner.start + 1}) on {base_name}, now it sits on `{item.name}`", fix))
	return findings


def move_doc_lines(texts, rows, owner_start):
	moved = [texts[r] for r in rows]
	for r in reversed(rows):
		del texts[r]
	target = owner_start - sum(1 for r in rows if r < owner_start)
	texts[target:target] = moved


def lint(repo, path, base, fix):
	file = os.path.join(repo, path)
	shown = subprocess.run(["git", "-C", repo, "show", f"{base}:{path}"], capture_output=True, text=True)
	if shown.returncode:
		return []  # a new file has no doc to move
	lines = strip_code(open(file).read())
	findings = check_moved_docs(path, lines, parse_items(lines), shown.stdout, base[:9])
	fixable = [f for f in findings if f.fix]
	if not (fix and fixable):
		return findings
	texts = [line.text for line in lines]
	move_doc_lines(texts, *fixable[0].fix)  # one at a time: a move shifts the rows of the others
	with open(file, "w") as out:
		out.write("\n".join(texts))
	print(f"fixed {fixable[0]}")
	return lint(repo, path, base, fix)


def git(repo, *arguments):
	return subprocess.run(["git", "-C", repo, *arguments], capture_output=True, text=True).stdout.strip()


def main():
	parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
	parser.add_argument("paths", nargs="*", help="files to check (default: the .rs files changed since the base)")
	parser.add_argument("--fix", action="store_true", help="repair what can be repaired safely")
	parser.add_argument("--base", default=DEFAULT_BASE, help="compare with the merge base of HEAD and this ref (default origin/main)")
	args = parser.parse_args()
	repo = git(os.getcwd(), "rev-parse", "--show-toplevel")
	base = git(repo, "merge-base", "HEAD", args.base)
	if not base:
		sys.exit(f"style_lint: no merge base of HEAD and {args.base}")
	if args.paths:
		paths = [os.path.relpath(os.path.abspath(p), repo) for p in args.paths]
	else:
		paths = [p for p in git(repo, "diff", "--name-only", base, "--", "*.rs").split() if os.path.exists(os.path.join(repo, p))]
	findings = [finding for path in paths for finding in lint(repo, path, base, args.fix)]
	for finding in findings:
		print(finding)
	if findings:
		print(f"STYLE {len(findings)} violations" + ("" if args.fix else " (scripts/style_lint.py --fix repairs most)"), file=sys.stderr)
	sys.exit(1 if findings else 0)


if __name__ == "__main__":
	main()
