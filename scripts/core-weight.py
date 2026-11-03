#!/usr/bin/env python3
"""Core weight: lines of code of the compiler per landing on main, core without extensions and extensions apart.

scripts/core-weight.py [since] [out.html]  (defaults: 2026-10-29, the start of self-hosting; data/core-weight/core-weight.html)
Reconstructs every first-parent commit of origin/main from git history (about 6 s), fills scripts/core-weight.html
and prints the summary. The nightly run (scripts/integrate/nightly.sh) calls it; a Claude session republishes the
page to its artifact URL (notes/core_hygiene.md "Core weight").
"""
import collections, json, os, subprocess, sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SINCE = sys.argv[1] if len(sys.argv) > 1 else "2026-10-29"
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(REPO, "data/core-weight/core-weight.html")
TEMPLATE = os.path.join(REPO, "scripts/core-weight.html")
CODE_SUFFIXES = (".warp", ".c", ".h", ".js", ".sh", ".py")
CORE_GROUPS = [("parsing", ["core/parsing"]), ("passes", ["core/passes"]),
               ("analysis", ["core/analysis", "core/commands", "core/(top)"]), ("emitter", ["core/emitter"]),
               ("runtime", ["core/runtime"]), ("native", ["core/native"])]
EXTENSIONS = "extensions"  # core/extensions/, and lib/sugar/ before it moved there (2026-11-03)


def component(path):
    if not path.endswith(CODE_SUFFIXES):
        return None
    parts = path.split("/")
    if parts[0] == "core":
        if len(parts) > 2 and parts[1] == "extensions":
            return EXTENSIONS
        return "core/" + parts[1] if len(parts) > 2 else "core/(top)"
    if parts[0] == "lib":
        return EXTENSIONS if len(parts) > 2 and parts[1] == "sugar" else "lib"
    return parts[0] if parts[0] in ("test", "samples") else None


def git(*args):
    return subprocess.run(["git", "-C", REPO, *args], capture_output=True, text=True, check=True).stdout


def lines_at(commit):
    totals = collections.Counter()
    wanted = [f for f in git("ls-tree", "-r", "--name-only", commit).splitlines() if component(f)]
    blob = subprocess.run(["git", "-C", REPO, "cat-file", "--batch"], capture_output=True, check=True,
                          input="".join(f"{commit}:{f}\n" for f in wanted).encode()).stdout
    position = 0
    for f in wanted:
        header_end = blob.index(b"\n", position)
        size = int(blob[position:header_end].split()[-1])
        totals[component(f)] += blob[header_end + 1:header_end + 1 + size].count(b"\n")
        position = header_end + 2 + size
    return totals


def group_sums(counter):
    return [sum(counter.get(c, 0) for c in names) for _, names in CORE_GROUPS]


commits = git("log", "--first-parent", "--reverse", "--format=%H %ad %s", "--date=iso-strict",
              "--since=" + SINCE, "origin/main").splitlines()
base = git("rev-parse", commits[0].split()[0] + "^1").strip()
totals = lines_at(base)
rows = []
for line in commits:
    sha, date, subject = line.split(" ", 2)
    change = collections.Counter()
    for stat in git("diff", "--numstat", "--no-renames", sha + "^1", sha).splitlines():
        added, removed, path = stat.split("\t", 2)
        if added != "-" and component(path):
            change[component(path)] += int(added) - int(removed)
    totals.update(change)
    rows.append([date[:16].replace("T", " "), sha[:10], subject[:140], group_sums(totals) + [totals[EXTENSIONS]],
                 sum(group_sums(change)), change[EXTENSIONS], totals["lib"], totals["test"]])

data = {"groups": [name for name, _ in CORE_GROUPS] + [EXTENSIONS], "base": base[:10], "rows": rows}
os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(TEMPLATE) as template, open(OUT, "w") as page:
    page.write(template.read().replace("__DATA__", json.dumps(data, separators=(",", ":")).replace("</", "<\\/")))
first, last = rows[0], rows[-1]
core = lambda row: sum(row[3][:-1])
print(f"core {core(first)} -> {core(last)}, extensions {first[3][-1]} -> {last[3][-1]}, "
      f"lib {first[6]} -> {last[6]}, test {first[7]} -> {last[7]} over {len(rows)} landings: {OUT}")
