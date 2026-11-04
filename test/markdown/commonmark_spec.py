#!/usr/bin/env python3
# CommonMark 0.31.2 spec examples through markdown_html (lib/markdown.warp); not part of smoke.
# usage: WARP_LIB=$PWD/lib test/markdown/commonmark_spec.py [--fails SECTION|all] [--out FILE]
# prints passed/total per section and in all; by design: raw HTML is escaped, not passed through
import json, os, subprocess, sys, tempfile, collections
here = os.path.dirname(os.path.abspath(__file__))
ex = json.load(open(os.path.join(here, "commonmark-0.31.2.json")))
W = lambda s: '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\t", "\\t") + '"'
prog = "use markdown\nxs = [" + ", ".join(W(e["markdown"]) for e in ex) + "]\nfor x in xs { print(markdown_html(x)); print(\"@@END@@\") }\n"
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False) as f: f.write(prog)
out = subprocess.run(["warp", f.name], capture_output=True, text=True).stdout.split("@@END@@\n")
args = sys.argv[1:]
if "--out" in args: open(args[args.index("--out") + 1], "w").write(json.dumps(out))
tot, ok = collections.Counter(), collections.Counter()
fails = []
for i, e in enumerate(ex):
    got = out[i][:-1] if i < len(out) else None
    tot[e["section"]] += 1
    if got == e["html"]: ok[e["section"]] += 1
    else: fails.append((e, got))
for s in tot: print(f"{ok[s]:4}/{tot[s]:<4} {s}")
print(f"{sum(ok.values())}/{len(ex)} total")
if "--fails" in args:
    want = args[args.index("--fails") + 1]
    for e, got in fails:
        if want in ("all", e["section"]): print(f"\n#{e['example']} {e['section']}\n--md {e['markdown']!r}\n--want {e['html']!r}\n--got  {got!r}")
