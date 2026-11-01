#!/usr/bin/env python3
# The keywords editors color (P165), run from the repository root:
#   keywords.py              → {"hard", "soft", "modules"} as JSON on stdout, for editor plugins
#   keywords.py keywords.js  → the playground's keywords.js (build.sh), and keywords.txt beside it: the two lists in
#                              words, part of what the assistant tells Claude about warp (assistant.json "guides")
# hard: never a name of the program's own; soft: a word with a meaning that a local may still take as its name. The
# self-hosted compiler keeps no list of them in one place, so they are the editors' own, here (the Rust compiler's
# soft_keywords.rs lists as they were at its end, 2026-10-29). The modules `use` completes are the compiler's standard
# modules (core/passes/rewrites.warp std_module_names).
import json, re, sys

HARD = ["if", "then", "else", "while", "for", "in", "return", "break", "continue", "def", "fun", "fn", "class", "var",
	"let", "const", "val", "global", "true", "false", "ø", "null", "nil", "use", "import", "include", "data"]
SOFT = ["emit", "send", "broadcast", "every", "whenever", "init", "new", "root", "listeners", "stored", "undo", "redo",
	"try", "catch", "except", "switch", "match", "case", "when", "do", "yield", "await", "go", "elif", "elsif", "elseif",
	"end", "until", "law", "on", "once", "within", "after", "of", "from", "to", "upto", "times", "is", "as", "with",
	"extends", "mixin", "static", "trait", "interface", "protocol", "struct", "record", "operator", "nonlocal", "lambda",
	"define", "function", "func", "result", "print", "assert", "linear", "shared", "ref", "meta", "raise", "throw",
	"fail", "panic", "warn", "require", "and", "or", "not", "xor", "fire", "trigger", "signal", "where", "loop", "alias",
	"finally", "fi", "version"]
STANDARD_MODULES = "core/passes/rewrites.warp"
MODULE_NAMES = re.compile(r"^std_module_names = \[(.*?)\]", re.M)

def keywords():
	modules = re.findall(r'"([^"]+)"', MODULE_NAMES.search(open(STANDARD_MODULES, encoding="utf-8").read()).group(1))
	return {"hard": HARD, "soft": SOFT, "modules": modules}

def in_words(found):
	return f"Keywords: {' '.join(found['hard'])}\nSoft keywords (a local may take the name): {' '.join(found['soft'])}\n"

if len(sys.argv) > 1:
	found = keywords()
	with open(sys.argv[1], "w", encoding="utf-8") as script:
		script.write(f"// made by build.sh (keywords.py; the modules of {STANDARD_MODULES})\nconst KEYWORDS = {json.dumps(found, ensure_ascii=False)};\n")
	with open(re.sub(r"\.js$", ".txt", sys.argv[1]), "w", encoding="utf-8") as text:
		text.write(in_words(found))
else:
	print(json.dumps(keywords(), ensure_ascii=False))
