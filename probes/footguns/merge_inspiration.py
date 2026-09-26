"""Appends each '### heading' block (matched against ## or ### headings) of probes/footguns/inspiration/*.md to the entry with the same heading in Footguns.md."""
import glob, re, sys

CATALOGUE = "Footguns.md"
NOTES = "probes/footguns/inspiration/*.md"
SKIPPED = "BRIEF.md"
MARKER = "Solved elsewhere:"

def sections(text):
	parts = re.split(r"^### (.+)$", text, flags=re.M)
	return {title.strip(): body.strip() for title, body in zip(parts[1::2], parts[2::2])}

def normalize(title):
	return re.sub(r"[^a-z0-9]+", " ", title.lower()).strip()

catalogue = open(CATALOGUE).read()
headings = {normalize(title): marks + " " + title for marks, title in re.findall(r"^(#{2,3}) (.+)$", catalogue, flags=re.M)}
unmatched = []
for path in sorted(glob.glob(NOTES)):
	if path.endswith(SKIPPED):
		continue
	for title, body in sections(open(path).read()).items():
		heading = headings.get(normalize(title))
		if not heading:
			unmatched.append(f"{path}: {title}")
			continue
		start = catalogue.index(f"{heading}\n")
		following = re.compile(r"^(#{1,3} )", re.M).search(catalogue, start + 4)
		end = following.start() if following else len(catalogue)
		entry = catalogue[start:end].rstrip("\n")
		if MARKER in entry:
			continue
		body = body.replace("../../../", "")
		catalogue = catalogue[:start] + entry + "\n\n" + body + "\n\n" + catalogue[end:]
open(CATALOGUE, "w").write(catalogue)
print("\n".join(["unmatched:"] + unmatched) if unmatched else "all matched", file=sys.stderr)
