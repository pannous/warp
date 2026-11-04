#!/usr/bin/env python3
# Every named character reference of WHATWG's entities.json (test/html/entities.json) through lib/html's html_unescape:
# each name with its ; and, for the legacy names that may end without one, without it at the end of the text and
# before a non-name character. lib has one decoding (text context: no attribute context), so there is no attribute
# group. Prints counts by group; --fails GROUP lists that group's failures. Outside smoke; run from the repo root.
import json, os, subprocess, sys, tempfile, collections
root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
def W(s):
    out = []
    for ch in s:
        o = ord(ch)
        if ch == "\\": out.append("\\\\")
        elif ch == '"': out.append('\\"')
        elif o < 32 or o == 127 or 0x80 <= o < 0xA0 or 0x300 <= o < 0x370 or 0x200B <= o < 0x2070 or 0x20D0 <= o < 0x2100 or 0xFE00 <= o < 0xFE10:
            out.append('" + chr(%d) + "' % o)
        else: out.append(ch)
    return '"' + "".join(out) + '"'
data = json.load(open(os.path.join(root, "test/html/entities.json"), encoding="utf-8"))
cases = []
for name, v in sorted(data.items()):
    c = v["characters"]
    if name.endswith(";"): cases.append(("with ;", name, "x" + name + "y", "x" + c + "y"))
    else:
        cases.append(("legacy, no ; at the end", name, "x" + name, "x" + c))
        cases.append(("legacy, no ; before #", name, name + "#", c + "#"))
prog = "use html\nuse json\nto_json_text(t: text): text := to_json(t)\n" + "".join('print(to_json_text(html_unescape(%s)))\nprint("@@")\n' % W(c[2]) for c in cases)
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
outs = [o.strip("\n") for o in subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace").stdout.split("@@\n")]
ok, total, fails = collections.Counter(), collections.Counter(), collections.defaultdict(list)
for i, (g, name, inp, want) in enumerate(cases):
    total[g] += 1
    try: got = json.loads(outs[i])
    except Exception: got = "CRASH " + (outs[i] if i < len(outs) else "")[:80]
    if got == want: ok[g] += 1
    else: fails[g].append((name, inp, want, got))
print("entities (%d names): %d/%d" % (len(data), sum(ok.values()), sum(total.values())))
for g in total: print("  %5d/%-5d %s" % (ok[g], total[g], g))
if "--fails" in sys.argv:
    for name, inp, want, got in fails[sys.argv[sys.argv.index("--fails") + 1]][:40]:
        print("\n%s\n  input %r\n  want  %r\n  got   %r" % (name, inp, want, got))
