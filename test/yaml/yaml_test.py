#!/usr/bin/env python3
# A subset of yaml-test-suite (vendored in test/yaml/yaml-test-suite) against lib/yaml's parse_yaml_all; not part of smoke.
# usage: WARP_LIB=$PWD/lib test/yaml/yaml_test.py [--fails TAG|all]
# Each case's documents, as JSON, must equal its in.json (one JSON value per document). Counts are by tag: a case counts
# under each of its tags; the total counts each case once.
# Selection (see README): valid cases with an in.json, tagged with at least one of sequence mapping flow scalar anchor
# alias comment document simple spec literal folded double single indent whitespace empty header footer, and none of
# complex-key explicit-key tag local-tag unknown-tag directive 1.3-err 1.3-mod libyaml-err error duplicate-key
# empty-key edge upto-1.2; no line starting with % ? or !, no " !" tag, no tab characters.
import json, os, subprocess, sys, tempfile, collections, math
here = os.path.dirname(os.path.abspath(__file__)); base = os.path.join(here, "yaml-test-suite")
cases = []
for root, dirs, files in os.walk(base):
    if "in.yaml" in files: cases.append(os.path.relpath(root, base))
cases.sort()
W = lambda s: '"' + "".join('\\\\' if c == "\\" else '\\"' if c == '"' else "\\n" if c == "\n" else "\\t" if c == "\t" else c if ord(c) >= 32 else '" + chr(%d) + "' % ord(c) for c in s) + '"'
prog = 'use yaml\nuse json\ncheck(s: text): text := try { "OK " + to_json(parse_yaml_all(s)) } catch e { "ERR " + e.message.replace("\\n", " ") }\n'
for c in cases: prog += 'print(check(%s))\nprint("@@")\n' % W(open(os.path.join(base, c, "in.yaml"), encoding="utf-8").read())
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as f: f.write(prog)
outs = [o.strip() for o in subprocess.run(["warp", f.name], capture_output=True, text=True, errors="replace").stdout.split("@@\n")]
def docs(t):
    d, out, at = json.JSONDecoder(), [], 0
    t = t.strip()
    while at < len(t):
        v, at = d.raw_decode(t, at); out.append(v)
        while at < len(t) and t[at].isspace(): at += 1
    return out
def norm(x):
    if isinstance(x, bool) or x is None: return x
    if isinstance(x, (int, float)): return float(x) if not (isinstance(x, float) and math.isnan(x)) else "nan"
    if isinstance(x, list): return [norm(v) for v in x]
    if isinstance(x, dict): return {str(k): norm(v) for k, v in x.items()}
    return x
tot, ok, fails, total = collections.Counter(), collections.Counter(), [], 0
for i, c in enumerate(cases):
    tags = open(os.path.join(base, c, "tags")).read().split()
    want = docs(open(os.path.join(base, c, "in.json"), encoding="utf-8").read())
    o = outs[i] if i < len(outs) else "CRASH"
    try: good = o.startswith("OK ") and norm(json.loads(o[3:])) == norm(want)
    except ValueError: good = False
    total += good
    for t in tags: tot[t] += 1; ok[t] += good
    if not good: fails.append((c, tags, o, want))
print(f"yaml subset: {total}/{len(cases)}")
for t in sorted(tot): print(f"  {ok[t]:4}/{tot[t]:<4} {t}")
if "--fails" in sys.argv:
    w = sys.argv[sys.argv.index("--fails") + 1]
    for c, tags, o, want in fails:
        if w == "all" or w in tags:
            print(f"\n{c} [{' '.join(tags)}] {open(os.path.join(base, c, '===')).read().strip()}\n--yaml {open(os.path.join(base, c, 'in.yaml')).read()!r}\n--want {json.dumps(want)[:300]}\n--got  {o[:300]}")
