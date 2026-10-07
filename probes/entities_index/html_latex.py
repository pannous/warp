# The uniscript names HTML and LaTeX define differently, with both readings and the one uniscript takes (user decision
# P198: the LaTeX reading, except for letters with a diacritic, which take the HTML one), as the table of notes/footguns.md.
# Reads the sources of a uniscript checkout; latex.wasp keeps the LaTeX readings HTML wins over as `// name: "…"` lines.
# Run: python3 probes/entities_index/html_latex.py ~/dev/uniscript
import glob, re, sys, unicodedata

ENTRY = re.compile(r'^\t(// )?([^\s:]+): "((?:[^"\\]|\\.)*)"', re.M)
LETTER_CATEGORIES = ("Lu", "Ll", "Lt", "Lo")


def section(path, name, commented=None):
	"""The entries of a section; `commented` collects the names whose entry is commented out"""
	text = open(path).read()
	if name + " {" not in text:
		return {}
	body = text[text.index(name + " {"):]
	body = body[:body.index("\n}")]
	unescaped = lambda value: re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda match: chr(int(match.group(1), 16)), value).replace('\\"', '"')
	entries = {}
	for comment, key, value in ENTRY.findall(body):
		entries.setdefault(key, unescaped(value))
		if comment and commented is not None:
			commented.add(key)
	return entries


def shown(text):
	categories = ",".join(unicodedata.category(character) for character in text)
	visible = "◌" + text if unicodedata.category(text[0]).startswith("M") else text
	return f"{visible} ({categories})"


root = sys.argv[1]
html = section(f"{root}/data/entities/html.wasp", "html")
html_wins = set()
latex = section(f"{root}/data/entities/latex.wasp", "latex", html_wins)
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
	picked = html[name] if name in html_wins else latex[name]
	pick = "HTML" if name in html_wins else "LaTeX (exception: a math name)" if is_letter else "LaTeX"
	effect = f"{shown(higher[name])}, the Unicode name wins" if name in higher and higher[name] != picked else shown(picked)
	print(f"| {name} | {shown(html[name])} | {shown(latex[name])} | {pick} | {effect} |")
