#!/usr/bin/env python3
# The keywords editors color (P165), from their definitions in the sources, run from the repository root:
#   keywords.py              → {"hard", "soft", "modules"} as JSON on stdout, for editor plugins
#   keywords.py keywords.js  → the playground's keywords.js (build.sh), and keywords.txt beside it: the two lists in
#                              words, part of what the assistant tells Claude about warp (assistant.json "guides")
# hard and soft: src/lowering/soft_keywords.rs's lists; soft also every `const …_KEYWORD: &str = "word"` in src/, so a
# pass registers the word it gives meaning by naming its constant so (card where-infix)
import glob, json, re, sys

KEYWORD_LISTS = "src/lowering/soft_keywords.rs"
MODULES = "src/modules.rs"
SOURCES = "src/**/*.rs"
KEYWORD_CONSTANT = re.compile(r'const [A-Z_]+_KEYWORD: &str = "([a-z_]+)"')

def read(path):
	return open(path, encoding="utf-8").read()

def listed(source, name):
	return re.findall(r'"([^"]+)"', re.search(rf"const {name}: \[&str; \d+\] = \[(.*?)\];", source, re.S).group(1))

# the standard modules `use` completes (completion.js): STD_MODULES' names, their aliases, the named constants'
def module_names(modules):
	std_names = re.findall(r'\(\s*(?:"([a-z0-9_]+)"|([A-Z_]+_MODULE)),\s*include_str!', modules)
	constant = lambda name: re.search(rf'const {name}: &str = "([^"]+)"', modules).group(1)
	aliases = re.findall(r'\("([a-z_]+)", "[a-z_]+"\)', re.search(r"STD_MODULE_ALIASES.*?\];", modules, re.S).group(0))
	return [text or constant(name) for text, name in std_names] + aliases

def keywords():
	lists = read(KEYWORD_LISTS)
	hard = listed(lists, "HARD_KEYWORDS")
	constants = [word for path in sorted(glob.glob(SOURCES, recursive=True)) for word in KEYWORD_CONSTANT.findall(read(path))]
	soft = [word for word in dict.fromkeys(listed(lists, "SOFT_KEYWORDS") + listed(lists, "HIGHLIGHTED_WORDS") + constants) if word not in hard]
	return {"hard": hard, "soft": soft, "modules": module_names(read(MODULES))}

def in_words(found):
	return f"Keywords: {' '.join(found['hard'])}\nSoft keywords (a local may take the name): {' '.join(found['soft'])}\n"

if len(sys.argv) > 1:
	found = keywords()
	with open(sys.argv[1], "w", encoding="utf-8") as script:
		script.write(f"// made by build.sh (keywords.py) from {KEYWORD_LISTS} and the _KEYWORD constants in src/\nconst KEYWORDS = {json.dumps(found, ensure_ascii=False)};\n")
	with open(re.sub(r"\.js$", ".txt", sys.argv[1]), "w", encoding="utf-8") as text:
		text.write(in_words(found))
else:
	print(json.dumps(keywords(), ensure_ascii=False))
