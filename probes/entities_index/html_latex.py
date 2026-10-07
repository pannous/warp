# The uniscript names HTML and LaTeX define differently, with both readings and the one uniscript takes (user decision
# P198: a letter takes the HTML reading, anything else the LaTeX one), as the markdown table of notes/footguns.md.
# Reads the sources of a uniscript checkout; latex.wasp keeps the readings P198 overrides as `// name: "…"  // P198` lines.
# Run: python3 probes/entities_index/html_latex.py ~/dev/uniscript
import glob, re, sys, unicodedata

ENTRY = re.compile(r'^\t(?:// )?([^\s:]+): "((?:[^"\\]|\\.)*)"', re.M)
LETTER_CATEGORIES = ("Lu", "Ll", "Lt", "Lo")


def section(path, name):
	text = open(path).read()
	if name + " {" not in text:
		return {}
	body = text[text.index(name + " {"):]
	body = body[:body.index("\n}")]
	unescaped = lambda value: re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda match: chr(int(match.group(1), 16)), value).replace('\\"', '"')
	entries = {}
	for key, value in ENTRY.findall(body):
		entries.setdefault(key, unescaped(value))
	return entries


def shown(text):
	categories = ",".join(unicodedata.category(character) for character in text)
	visible = "◌" + text if unicodedata.category(text[0]).startswith("M") else text
	return f"{visible} ({categories})"


root = sys.argv[1]
html = section(f"{root}/data/entities/html.wasp", "html")
latex = section(f"{root}/data/entities/latex.wasp", "latex")
# the sections before latex and html win over both (uniscript's own names, then the Unicode names)
higher = section(f"{root}/data/entities/uniscript.wasp", "uniscript")
for path in sorted(glob.glob(f"{root}/data/entities/unicode/*.wasp")):
	for key, value in section(path, "names").items():
		higher.setdefault(key, value)
differing = sorted((name for name in html.keys() & latex.keys() if html[name] != latex[name]), key=str.lower)
print(f"{len(differing)} names differ:\n")
print("| name | HTML | LaTeX | P198 picks | in effect |")
print("|---|---|---|---|---|")
for name in differing:
	is_letter = len(html[name]) == 1 and unicodedata.category(html[name]) in LETTER_CATEGORIES
	picked = html[name] if is_letter else latex[name]
	effect = f"{shown(higher[name])}, the Unicode name wins" if name in higher and higher[name] != picked else shown(picked)
	print(f"| {name} | {shown(html[name])} | {shown(latex[name])} | {'HTML' if is_letter else 'LaTeX'} | {effect} |")
