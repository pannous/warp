#!/usr/bin/env python3
# test/path/path_cases.json (gen_path_cases.py) through lib/path and lib/file: counts by word group; --fails GROUP
# (or all) lists the differences. File cases run in a fresh temp dir (TMP). Outside smoke; run from the repo root.
import json, os, subprocess, sys, tempfile, collections, shutil
here = os.path.dirname(os.path.abspath(__file__))
cases = json.load(open(os.path.join(here, "path_cases.json"), encoding="utf-8"))
tmp = tempfile.mkdtemp(prefix="warp-path-")
for d in ("l", "g", "h"): os.makedirs(os.path.join(tmp, d), exist_ok=True)
def W(s): return s.replace("TMP", tmp)
prog = "use path\nuse file\nuse json\n" + "".join('print(try { to_json(%s) } catch e { "ERR " + e })\nprint("@@")\n' % ("{ " + W(e) + " }" if ";" in e else W(e)) for g, e, w, r in cases)
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
out = subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace", timeout=600).stdout.split("@@\n")
os.unlink(fh.name); shutil.rmtree(tmp, ignore_errors=True)
# differences kept on purpose, and ones that wait on a decision (a fix would change an existing word's answer)
BY_DESIGN = {'normalize_path("//")': "a leading // is one / (POSIX leaves // implementation-defined)",
             'dirname("//a")': "a leading // is one /", 'basename(".")': "posixpath.basename keeps .",
             'stem(".")': "the stem of . is . (as basename)", 'glob_match("*", "")': "fnmatch: * matches the empty text"}
HELD = {'extension("a.")': "pathlib: a trailing dot is no suffix; lib gives .", 'stem("a.")': "pathlib: a.; lib gives a"}
ok, tot, fails = collections.Counter(), collections.Counter(), []
known = collections.Counter()
for (g, e, w, r), o in zip(cases, out + ["MISSING"] * len(cases)):
    o = o.strip("\n"); tot[g] += 1
    try: got = json.loads(o)
    except Exception: got = o
    if isinstance(got, list) and all(isinstance(x, str) for x in got): got = [x.replace(tmp, "TMP") for x in got]
    want = w
    if got == want: ok[g] += 1
    elif e in BY_DESIGN: known["by design"] += 1; fails.append((g, e, w, got, "by design: " + BY_DESIGN[e]))
    elif e in HELD: known["held back"] += 1; fails.append((g, e, w, got, "held back: " + HELD[e]))
    else: fails.append((g, e, w, got, r))
print("path and file: %d/%d, %d by design, %d held back for a decision" % (sum(ok.values()), len(cases), known["by design"], known["held back"]))
for g in tot: print("  %3d/%-3d %s" % (ok[g], tot[g], g))
if "--fails" in sys.argv:
    k = sys.argv[sys.argv.index("--fails") + 1]
    for g, e, w, got, r in fails:
        if k in ("all", g): print("  [%s] %s\n      want %r (%s)\n      got  %r" % (g, e, w, r, got))
