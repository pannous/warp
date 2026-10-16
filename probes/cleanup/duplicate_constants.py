#!/usr/bin/env python3
"""Word constants defined more than once in src/ (notes/restructuring.md, card duplicate-constants).

duplicate_constants.py [src_dir]

Reads every `const`/`static` whose value is a string or a list of strings (`[&str; N]`, `&[&str]`, phf sets) and
prints: one-string constants with the same text, lists with the same set of words, and lists that hold another
list entirely (a subset worth a look).
"""
import collections
import os
import re
import sys

ITEM = re.compile(r"^[ \t]*(?:pub(?:\([^)]*\))? )?(?:const|static) ([A-Z_0-9]+)\s*:[^=]*=\s*", re.M)
STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')
MIN_SUBSET = 3


def item_value(text, start):
	"""The text of the value starting at start, up to its `;` outside brackets"""
	depth, position = 0, start
	while position < len(text):
		char = text[position]
		raw = re.match(r'r(#*)"', text[position:position + 8])
		if raw:
			position = text.index('"' + raw.group(1), position + len(raw.group(0))) + len(raw.group(1))
		elif re.match(r"'(\\.|[^\\'])'", text[position:position + 4]):
			position = text.index("'", position + 2)
		elif char == '"':
			position += 1
			while text[position] != '"':
				position += 2 if text[position] == "\\" else 1
		elif char in "([{":
			depth += 1
		elif char in ")]}":
			depth -= 1
		elif char == ";" and depth == 0:
			return text[start:position]
		position += 1
	return ""


def word_constants(source_dir):
	for root, _, files in os.walk(source_dir):
		for name in files:
			if not name.endswith(".rs"):
				continue
			path = os.path.join(root, name)
			text = open(path, encoding="utf-8").read()
			for match in ITEM.finditer(text):
				value = item_value(text, match.end())
				words = STRING.findall(value)
				# only plain word values: a string literal or brackets of string literals
				if words and not re.sub(STRING, "", value).strip(" \t\n,[]&()phf_set!{}"):
					line = text.count("\n", 0, match.start()) + 1
					yield match.group(1), f"{os.path.relpath(path, source_dir)}:{line}", words, value.lstrip().startswith('"')


def main():
	source_dir = sys.argv[1] if len(sys.argv) > 1 else "src"
	constants = list(word_constants(source_dir))
	singles = collections.defaultdict(list)
	sets = collections.defaultdict(list)
	for name, place, words, is_single in constants:
		(singles[words[0]] if is_single else sets[frozenset(words)]).append(f"{name} {place}")
	print("## Same one-word constant")
	for word, places in sorted(singles.items()):
		if len(places) > 1:
			print(f'"{word}": ' + ", ".join(places))
	print("\n## Same word list")
	for words, places in sorted(sets.items(), key=lambda item: -len(item[0])):
		if len(places) > 1:
			print(f"{sorted(words)}: " + ", ".join(places))
	print("\n## A list holding another")
	lists = [(frozenset(words), f"{name} {place}") for name, place, words, single in constants if not single]
	for words, place in lists:
		for other, other_place in lists:
			if len(other) >= MIN_SUBSET and other < words:
				print(f"{other_place} ⊂ {place}")


if __name__ == "__main__":
	main()
