#!/usr/bin/env python3
# nst's JSONTestSuite (test/json/JSONTestSuite) through lib/json: parse_json on each test_parsing file, y_ must parse
# (its value compared with python's json.loads), n_ must be refused, i_ is reported; every y_ and test_transform
# file also round trips, parse_json(to_json(parse_json(x))) == parse_json(x). A refused n_ case accepted for one of
# lib/json's documented JSON5 extensions counts as "by design" under that extension. Files that aren't UTF-8 are
# skipped. Each batch has a time limit; a batch that dies or hangs is run case by case (TIMEOUT s each).
# --fails y|n|i|rt lists failures; --depth finds the deepest [ nesting parsed. Outside smoke; run from the repo root.
import json, os, subprocess, sys, tempfile, re, math, collections
here = os.path.dirname(os.path.abspath(__file__)); TIMEOUT = 5
def W(s):
    out = []
    for ch in s:
        o = ord(ch)
        if ch == "\\": out.append("\\\\")
        elif ch == '"': out.append('\\"')
        elif o < 32 or o == 127 or 0xD800 <= o < 0xE000: out.append('" + chr(%d) + "' % o)
        else: out.append(ch)
    return '"' + "".join(out) + '"'
HEAD = 'use json\nattempt(t: text): text := try { "OK " + to_json(parse_json(t)) } catch e { "ERR " + e }\nround(t: text): text := try { if parse_json(to_json(parse_json(t))) == parse_json(t) then "SAME" else "DIFF " + to_json(parse_json(to_json(parse_json(t)))) } catch e { "ERR " + e }\n'
def run(texts, fn, timeout):
    prog = HEAD + "".join('print(%s(%s))\nprint("@@")\n' % (fn, W(t)) for t in texts)
    with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
    try: r = subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace", timeout=timeout); out = r.stdout
    except subprocess.TimeoutExpired: out = None
    os.unlink(fh.name)
    if out is None: return None
    parts = [p.strip("\n") for p in out.split("@@\n")]
    return parts[:len(texts)] if len(parts) > len(texts) else None
def run_all(texts, fn):
    # in chunks of 40: one that dies or hangs is run case by case
    res = []
    for k in range(0, len(texts), 40):
        chunk = texts[k:k + 40]
        outs = run(chunk, fn, 30 + TIMEOUT * len(chunk) // 4)
        if outs is None:
            outs = []
            for t in chunk:
                o = run([t], fn, TIMEOUT); outs.append(o[0] if o else "HANG or CRASH")
        res += outs
    return res
def same(a, b):
    if isinstance(a, float) or isinstance(b, float):
        try: return a == b or (math.isclose(float(a), float(b), rel_tol=1e-12))
        except Exception: return False
    if isinstance(a, list): return isinstance(b, list) and len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    if isinstance(a, dict): return isinstance(b, dict) and a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    return a == b and type(a) == type(b) or (type(a) in (int, float) and type(b) in (int, float) and a == b and not isinstance(a, bool) and not isinstance(b, bool))
def extension(name, text):
    # the documented JSON5 extension an accepted n_ case uses, or None
    t = text
    if re.search(r"/\*|//", t): return "comments"
    if "'" in t and re.search(r"[\[{:,]\s*'|^\s*'", t): return "single quotes"
    if re.search(r",\s*[\]}]", t): return "trailing commas"
    if re.search(r"[{,]\s*[A-Za-z_$][\w$]*\s*:", t): return "bare keys"
    if re.search(r"[{,]\s*[0-9][\w.+-]*\s*:", t): return "bare keys (numeric)"
    if re.search(r'\\[^"\\/bfnrtu]', t): return "unknown escapes (\\x is x)"
    if any(ord(c) < 32 and c not in " \n\r\t" for c in t) or re.search(r'"[^"]*[\t\n][^"]*"', t): return "raw control characters in strings"
    if re.search(r"(^|[\[,:\s])-?[+.]\d|\d\.($|[^\d])", t): return "JSON5 numbers (+1, .5, 1.)"
    return None
cases = []
skipped = collections.Counter()
for d in ("test_parsing",):
    for f in sorted(os.listdir(os.path.join(here, "JSONTestSuite", d))):
        raw = open(os.path.join(here, "JSONTestSuite", d, f), "rb").read()
        try: text = raw.decode("utf-8")
        except UnicodeDecodeError: skipped[f[0]] += 1; continue
        cases.append((f[0], f, text))
outs = run_all([c[2] for c in cases], "attempt")
res = collections.Counter(); tot = collections.Counter(); design = collections.Counter(); fails = collections.defaultdict(list); i_results = []
for (kind, f, text), o in zip(cases, outs):
    tot[kind] += 1
    accepted = o.startswith("OK ")
    if kind == "y":
        good = accepted
        if accepted:
            try: good = same(json.loads(o[3:]), json.loads(text))
            except Exception: pass
        res["y"] += good
        if not good: fails["y"].append((f, text, o))
    elif kind == "n":
        if not accepted: res["n"] += 1
        else:
            ext = extension(f, text)
            if ext: design[ext] += 1
            else: fails["n"].append((f, text, o))
    else:
        i_results.append((f, "accepted" if accepted else ("refused" if o.startswith("ERR") else o[:30])))
        if not (accepted or o.startswith("ERR")): fails["i"].append((f, text, o))
print("JSONTestSuite test_parsing (%d skipped as not UTF-8: %s)" % (sum(skipped.values()), dict(skipped)))
print("  y  %d/%d parse with python's value" % (res["y"], tot["y"]))
print("  n  %d/%d refused, %d accepted by design, %d accepted otherwise" % (res["n"], tot["n"], sum(design.values()), len(fails["n"])))
for k, v in sorted(design.items()): print("       by design (JSON5 leniency): %-16s %d" % (k, v))
ia = sum(1 for r in i_results if r[1] == "accepted"); ir = sum(1 for r in i_results if r[1] == "refused")
print("  i  %d: %d accepted, %d refused, %d crashed or hung" % (len(i_results), ia, ir, len(i_results) - ia - ir))
rt = [c for c in cases if c[0] == "y"] + [("t", f, open(os.path.join(here, "JSONTestSuite/test_transform", f), "rb").read().decode("utf-8", "replace")) for f in sorted(os.listdir(os.path.join(here, "JSONTestSuite/test_transform")))]
routs = run_all([c[2] for c in rt], "round")
rok = collections.Counter(); rtot = collections.Counter()
for (kind, f, text), o in zip(rt, routs):
    g = "test_parsing y_" if kind == "y" else "test_transform"
    rtot[g] += 1; rok[g] += o == "SAME"
    if o != "SAME": fails["rt"].append((f, text, o))
for g in rtot: print("  round trip %-16s %d/%d" % (g, rok[g], rtot[g]))
if "--i" in sys.argv:
    for f, r in i_results: print("    %-60s %s" % (f, r))
if "--depth" in sys.argv:
    lo, hi = 1, 100000
    while lo < hi:
        mid = (lo + hi + 1) // 2
        o = run(["[" * mid + "]" * mid], "attempt", TIMEOUT)
        if o and o[0].startswith("OK"): lo = mid
        else: hi = mid - 1
    print("  deepest [ nesting parsed: %d" % lo)
if "--fails" in sys.argv:
    for f, text, o in fails[sys.argv[sys.argv.index("--fails") + 1]][:60]: print("\n%s\n  input %r\n  got   %r" % (f, text[:120], o[:160]))
