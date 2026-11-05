#!/usr/bin/env python3
# The official mustache spec (test/template/mustache-spec, mustache/spec) through lib/template's mustache(template,
# data, partials): counts by spec file, the six required ones and the optional ~ ones; --fails FILE lists that file's
# failures. A ~lambdas case's lambda is warp-less JSON (code for other languages), so those cases are n/a. Outside
# smoke; run from the repo root.
import json, os, subprocess, sys, tempfile, glob
here = os.path.dirname(os.path.abspath(__file__))
def W(s):
    out = []
    for ch in s:
        o = ord(ch)
        if ch == "\\": out.append("\\\\")
        elif ch == '"': out.append('\\"')
        elif o < 32 or o == 127: out.append('" + chr(%d) + "' % o)
        else: out.append(ch)
    return '"' + "".join(out) + '"'
def has_lambda(v):
    if isinstance(v, dict): return v.get("__tag__") == "code" or any(has_lambda(x) for x in v.values())
    if isinstance(v, list): return any(has_lambda(x) for x in v)
    return False
def line(t): return 'print(to_json(mustache(%s, parse_json(%s), parse_json(%s))))\nprint("@@")\n' % (W(t["template"]), W(json.dumps(t["data"])), W(json.dumps(t.get("partials", {}))))
def run(tests):
    prog = "use template\nuse json\n" + "".join(line(t) for t in tests)
    with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
    r = subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace")
    os.unlink(fh.name)
    return [o.strip("\n") for o in r.stdout.split("@@\n")], r.stderr
def got_of(o):
    try: return json.loads(o)
    except Exception: return None
results, fails = [], {}
for path in sorted(glob.glob(os.path.join(here, "mustache-spec", "*.json"))):
    name = os.path.basename(path)[:-5]
    tests = json.load(open(path, encoding="utf-8"))["tests"]
    na = [t for t in tests if has_lambda(t["data"])]
    run_tests = [t for t in tests if not has_lambda(t["data"])]
    outs, err = run(run_tests) if run_tests else ([], "")
    if len(outs) < len(run_tests) + 1 or any(got_of(o) is None for o in outs[:len(run_tests)]):
        outs = []
        for t in run_tests:
            o, e = run([t]); outs.append(o[0] if o and got_of(o[0]) is not None else "CRASH " + (e.strip().splitlines() or [""])[-1][:100])
    ok = 0; fails[name] = []
    for t, o in zip(run_tests, outs):
        g = got_of(o)
        if g == t["expected"]: ok += 1
        else: fails[name].append((t, g if g is not None else o))
    results.append((name, ok, len(run_tests), len(na)))
req = [r for r in results if not r[0].startswith("~")]
print("mustache spec: required %d/%d" % (sum(r[1] for r in req), sum(r[2] for r in req)))
for name, ok, total, na in results: print("  %4d/%-4d %s%s" % (ok, total, name, "  (%d n/a: lambdas in JSON)" % na if na else ""))
if "--fails" in sys.argv:
    for t, g in fails.get(sys.argv[sys.argv.index("--fails") + 1], [])[:40]:
        print("\n%s\n  template %r\n  want     %r\n  got      %r" % (t["name"], t["template"], t["expected"], g))
