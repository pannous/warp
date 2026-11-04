#!/usr/bin/env python3
# csv-spectrum (test/csv/csv-spectrum) and RFC 4180 edge cases (test/csv/rfc4180) against lib/csv; not part of smoke.
# usage: WARP_LIB=$PWD/lib test/csv/csv_test.py [--fails spectrum|rfc4180|roundtrip|all]
# spectrum: csv_maps(text) must equal the case's json (header row to maps); rfc4180: csv_rows(text) must equal the
# rows, which come from python's csv.reader, except: a UTF-8 BOM before the first field is dropped (as Excel writes
# it), and a blank line is a record of one empty field, as RFC 4180's grammar reads it (python's reader gives []);
# roundtrip: csv_rows(csv_text(rows)) must give the rows back, for every case's rows.
import json, os, subprocess, sys, tempfile, collections
here = os.path.dirname(os.path.abspath(__file__))
W = lambda s: '"' + "".join({"\\": "\\\\", '"': '\\"', "\n": "\\n", "\r": "\\r", "\t": "\\t"}.get(c, c) if ord(c) >= 32 or c in "\n\r\t" else '" + chr(%d) + "' % ord(c) for c in s) + '"'
def V(v):
    if isinstance(v, list): return "[" + ", ".join(V(x) for x in v) + "]"
    if isinstance(v, dict): return "{" + ", ".join(W(k) + ": " + V(x) for k, x in v.items()) + "}"
    return W(v)
skipped = []
cases = []  # (group, name, warp expression giving JSON text, expected)
sp = os.path.join(here, "csv-spectrum")
for f in sorted(os.listdir(os.path.join(sp, "csvs"))):
    t = open(os.path.join(sp, "csvs", f), encoding="utf-8", newline="").read()
    want = json.load(open(os.path.join(sp, "json", f[:-4] + ".json"), encoding="utf-8"))
    # upstream's location_coordinates.json is no list and its phone number is not the csv's: skipped
    if not isinstance(want, list): skipped.append(f); continue
    cases.append(("spectrum", f, "csv_maps(%s)" % W(t), want))
    rows = [list(want[0].keys())] + [list(m.values()) for m in want] if want else []
    if rows: cases.append(("roundtrip", f, "csv_rows(csv_text(%s))" % V(rows), rows))
rf = os.path.join(here, "rfc4180")
for f in sorted(x for x in os.listdir(rf) if x.endswith(".csv")):
    t = open(os.path.join(rf, f), encoding="utf-8", newline="").read()
    want = json.load(open(os.path.join(rf, f[:-4] + ".json"), encoding="utf-8"))
    cases.append(("rfc4180", f, "csv_rows(%s)" % W(t), want))
    rows = [r for r in want if r]
    if rows: cases.append(("roundtrip", f, "csv_rows(csv_text(%s))" % V(rows), rows))
prog = "use csv\nuse json\n" + "".join('print(try { to_json(%s) } catch e { "ERR " + e.message })\nprint("@@")\n' % e for _, _, e, _ in cases)
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
outs = [o.strip("\n") for o in subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace").stdout.split("@@\n")]
tot, ok, fails = collections.Counter(), collections.Counter(), []
for i, (g, name, e, want) in enumerate(cases):
    o = outs[i] if i < len(outs) else "CRASH"
    try: good = json.loads(o) == want
    except ValueError: good = False
    tot[g] += 1; ok[g] += good
    if not good: fails.append((g, name, want, o))
print(f"csv: {sum(ok.values())}/{sum(tot.values())}" + (f" (skipped, broken upstream expectation: {', '.join(skipped)})" if skipped else ""))
for g in ("spectrum", "rfc4180", "roundtrip"): print(f"  {ok[g]:4}/{tot[g]:<4} {g}")
if "--fails" in sys.argv:
    w = sys.argv[sys.argv.index("--fails") + 1]
    for g, name, want, o in fails:
        if w in (g, "all"): print(f"\n{g} {name}\n  want {json.dumps(want, ensure_ascii=False)[:300]}\n  got  {o[:300]}")
