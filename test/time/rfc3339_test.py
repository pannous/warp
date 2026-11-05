#!/usr/bin/env python3
# test/time/rfc3339_cases.tsv (gen_rfc3339_cases.py) through lib/time's parse_date: the milliseconds, or a refusal;
# every accepted case also round trips, parse_date(Instant(ms).text()) == ms, its text the UTC RFC 3339 python writes
# (milliseconds without trailing zeros). Counts by group; --fails GROUP. Outside smoke; run from the repo root.
import os, subprocess, sys, tempfile, collections, datetime as dt
here = os.path.dirname(os.path.abspath(__file__))
cases = [l.rstrip("\n").split("\t") for l in open(os.path.join(here, "rfc3339_cases.tsv"))]
prog = "use time\n" + "".join('print((try { "" + parse_date("%s") } catch e { "ERR" }) + " " + (try { Instant(parse_date("%s")).text() } catch e { "ERR" }) + " " + (try { "" + parse_date(Instant(parse_date("%s")).text()) } catch e { "ERR" }))\n' % ((x.replace("\\", "\\\\").replace('"', '\\"'),) * 3) for g, x, w in cases)
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False) as fh: fh.write(prog)
out = subprocess.run(["warp", fh.name], capture_output=True, text=True).stdout.splitlines()
os.unlink(fh.name)
def utc_text(ms):
    t = dt.datetime(1970, 1, 1) + dt.timedelta(milliseconds=ms)
    f = ("%03d" % (ms % 1000)).rstrip("0")
    return t.strftime("%Y-%m-%dT%H:%M:%S").rjust(19, "0") + ("." + f if f else "") + "Z"
ok, tot, rok, rtot, fails = collections.Counter(), collections.Counter(), collections.Counter(), collections.Counter(), collections.defaultdict(list)
for (g, x, want), o in zip(cases, out + ["MISSING"] * (len(cases) - len(out))):
    tot[g] += 1
    got = o.split(" ")[0]
    if got == want: ok[g] += 1
    else: fails[g].append((x, want, o))
    if want != "ERR" and got == want:
        rtot[g] += 1
        parts = o.split(" ")
        if len(parts) == 3 and parts[1] == utc_text(int(want)) and parts[2] == want: rok[g] += 1
        else: fails[g].append((x, "round trip " + utc_text(int(want)), o))
print("rfc 3339: %d/%d parse, %d/%d round trip" % (sum(ok.values()), len(cases), sum(rok.values()), sum(rtot.values())))
for g in tot: print("  %3d/%-3d  round trip %3d/%-3d  %s" % (ok[g], tot[g], rok[g], rtot[g], g))
if "--fails" in sys.argv:
    for x, w, o in fails[sys.argv[sys.argv.index("--fails") + 1]]: print("  %r want %s got %s" % (x, w, o))
