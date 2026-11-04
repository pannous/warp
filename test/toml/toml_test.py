#!/usr/bin/env python3
# toml-test (TOML 1.0 case list, vendored in test/toml/toml-test) against lib/toml's parse_toml; not part of smoke.
# usage: WARP_LIB=$PWD/lib test/toml/toml_test.py [--fails valid|invalid|GROUP] [--json FILE]
# valid: parse_toml's value, typed as toml-test's JSON ({"type", "value"}), must equal the case's .json;
# invalid: parse_toml must raise. Groups are the case's first folder (valid/string, invalid/table, ...).
# Cases that are not UTF-8 can't be a warp text: counted apart as skipped.
import re, json, os, subprocess, sys, tempfile, collections, math
here = os.path.dirname(os.path.abspath(__file__)); base = os.path.join(here, "toml-test")
files = [l.strip() for l in open(os.path.join(base, "files-toml-1.0.0")) if l.strip().endswith(".toml")]
def W(s):
    out = []
    for ch in s:
        o = ord(ch)
        if ch == "\\": out.append("\\\\")
        elif ch == '"': out.append('\\"')
        elif ch == "\n": out.append("\\n")
        elif ch == "\t": out.append("\\t")
        elif o < 32 or o == 127 or 0xD800 <= o < 0xE000: out.append('" + chr(%d) + "' % o)
        else: out.append(ch)
    return '"' + "".join(out) + '"'
PRELUDE = '''use toml
use json
typed(v: any): any := {
	if v is bool { return {"type": "bool", "value": if v then "true" else "false"} }
	if v is int { return {"type": "integer", "value": "" + v} }
	if v is float { return {"type": "float", "value": if v != v then "nan" else if v > 1e308 then "inf" else if v < -1e308 then "-inf" else "" + v} }
	if v is text { return {"type": "string", "value": v} }
	if v is list { return [typed(x) for x in v] }
	if v is map and #(v.keys) == 2 and v.has("toml") and v.has("value") { return {"type": "date:" + v["toml"], "value": v["value"]} }
	out = {}
	for k in v.keys { out[k] = typed(v[k]) }
	out
}
check(source: text): text := try { "OK " + to_json(typed(parse_toml(source))) } catch e { "ERR " + e.message.replace("\\n", " ") }
'''
def run(batch):
    prog = PRELUDE + "".join('print(check(%s))\nprint("@@")\n' % W(s) for _, s in batch)
    with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as f: f.write(prog)
    r = subprocess.run(["warp", f.name], capture_output=True, text=True, errors="replace", timeout=900)
    outs = [o.strip() for o in r.stdout.split("@@\n")][:len(batch)]
    return outs if len(outs) == len(batch) else None
def results(cases):
    res = {}
    for i in range(0, len(cases), 60):
        batch = cases[i:i + 60]
        outs = run(batch)
        if outs is None:  # a crash in the batch: one by one
            outs = []
            for c in batch:
                o = run([c]); outs.append(o[0] if o else "CRASH")
        for (name, _), o in zip(batch, outs): res[name] = o
    return res
KIND = {"offset-date-time": "datetime", "local-date-time": "datetime-local", "local-date": "date-local", "local-time": "time-local"}
def norm(x):
    if isinstance(x, list): return [norm(v) for v in x]
    if isinstance(x, dict) and set(x) == {"type", "value"} and isinstance(x["value"], str):
        t, v = x["type"], x["value"]
        if t.startswith("date:"): t = KIND.get(t[5:], t)
        if t == "float":
            f = float(v.replace("inf", "Infinity")) if v.lower() not in ("nan", "+nan", "-nan") else "nan"
            return (t, "nan" if f == "nan" or (isinstance(f, float) and math.isnan(f)) else f)
        if t == "integer": return (t, int(v))
        if t.startswith("date") or t.startswith("time"):  # as toml-test compares them: as times, .6 is .600
            v = re.sub(r"\.(\d+)", lambda m: "." + m.group(1).rstrip("0") if m.group(1).rstrip("0") else "", v.upper().replace(" ", "T"))
            return (t, v)
        return (t, v)
    if isinstance(x, dict): return {k: norm(v) for k, v in x.items()}
    return x
cases, skipped = [], collections.Counter()
for f in files:
    raw = open(os.path.join(base, f), "rb").read()
    try: cases.append((f, raw.decode("utf-8")))
    except UnicodeDecodeError: skipped[f.split("/")[0]] += 1
res = results(cases)
args = sys.argv[1:]
if "--json" in args: json.dump(res, open(args[args.index("--json") + 1], "w"))
tot, ok, fails = collections.Counter(), collections.Counter(), []
for f, _ in cases:
    group = "/".join(f.split("/")[:2]) if f.count("/") > 1 else f.split("/")[0]
    o = res[f]; kind = f.split("/")[0]
    if kind == "valid":
        want = json.load(open(os.path.join(base, f[:-5] + ".json"), encoding="utf-8"))
        good = o.startswith("OK ") and norm(json.loads(o[3:])) == norm(want)
    else: good = o.startswith("ERR ")
    tot[group] += 1; ok[group] += good
    if not good: fails.append((f, o))
for kind in ("valid", "invalid"):
    gs = sorted(g for g in tot if g.split("/")[0] == kind)
    print(f"{kind}: {sum(ok[g] for g in gs)}/{sum(tot[g] for g in gs)}" + (f" (+{skipped[kind]} skipped, not UTF-8)" if skipped[kind] else ""))
    for g in gs: print(f"  {ok[g]:4}/{tot[g]:<4} {g}")
if "--fails" in args:
    w = args[args.index("--fails") + 1]
    for f, o in fails:
        if f.startswith(w) or w == "all": print("\n" + f + "\n  " + o[:300])
