#!/usr/bin/env python3
# html5lib-tests tokenizer cases (test/html/html5lib-tokenizer) against lib/html; not part of smoke.
# usage: WARP_LIB=$PWD/lib test/html/tokenizer_test.py [--fails FILE|all]
# lib/html has no HTML tokenizer: it decodes character references (html_unescape). A case applies when its initial
# state is Data or RCDATA, its output is only Character tokens, and (in the Data state) its input holds no "<"; it
# passes when html_unescape(input) is the characters joined. Every other case is counted as not applicable, by reason.
# Parse errors are not compared (lib reports none).
import json, os, re, subprocess, sys, tempfile, collections
here = os.path.dirname(os.path.abspath(__file__)); base = os.path.join(here, "html5lib-tokenizer")
def unesc(s): return re.sub(r"\\u([0-9A-Fa-f]{4})", lambda m: chr(int(m.group(1), 16)), s)
def W(s):
    out = []
    for ch in s:
        o = ord(ch)
        if ch == "\\": out.append("\\\\")
        elif ch == '"': out.append('\\"')
        elif o < 32 or o == 127 or 0xD800 <= o < 0xE000 or 0x80 <= o < 0xA0: out.append('" + chr(%d) + "' % o)
        else: out.append(ch)
    return '"' + "".join(out) + '"'
cases, na = [], collections.Counter()
for f in sorted(x for x in os.listdir(base) if x.endswith(".test")):
    d = json.load(open(os.path.join(base, f), encoding="utf-8"))
    for t in d.get("tests", []):
        inp, out = t["input"], t["output"]
        if t.get("doubleEscaped"):
            inp = unesc(inp); out = [[unesc(x) if isinstance(x, str) else x for x in tok] for tok in out]
        for state in t.get("initialStates", ["Data state"]):
            g = f[:-5]
            if any(tok[0] != "Character" for tok in out): na[(g, "tags, comments or doctypes (no tokenizer)")] += 1; continue
            if state not in ("Data state", "RCDATA state"): na[(g, state + " (no character references)")] += 1; continue
            if "\r" in inp or "\x00" in inp: na[(g, "CR or NUL (input stream preprocessing, not unescaping)")] += 1; continue
            if "</" in inp: na[(g, "an end tag in RCDATA (no tokenizer)")] += 1; continue
            if state == "Data state" and "<" in inp: na[(g, "a < in text (no tokenizer)")] += 1; continue
            if any(0xD800 <= ord(c) < 0xE000 for c in inp + "".join(tok[1] for tok in out)): na[(g, "lone surrogates (no warp text)")] += 1; continue
            cases.append((g, t["description"], state, inp, "".join(tok[1] for tok in out)))
prog = "use html\n" + "".join('print(to_json_text(html_unescape(%s)))\nprint("@@")\n' % W(c[3]) for c in cases)
prog = prog.replace("use html\n", "use html\nuse json\nto_json_text(t: text): text := to_json(t)\n", 1)
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
outs = [o.strip("\n") for o in subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace").stdout.split("@@\n")]
tot, ok, fails = collections.Counter(), collections.Counter(), []
for k, (g, desc, state, inp, want) in enumerate(cases):
    o = outs[k] if k < len(outs) else "CRASH"
    try: good = json.loads(o) == want
    except ValueError: good = False
    tot[g] += 1; ok[g] += good
    if not good: fails.append((g, desc, state, inp, want, o))
print(f"tokenizer (character references): {sum(ok.values())}/{sum(tot.values())} applicable, {sum(na.values())} not applicable")
for g in sorted(set(tot) | {g for g, _ in na}): print(f"  {ok[g]:4}/{tot[g]:<4} {g}" + "".join(f"\n         n/a {n:4} {r}" for (gg, r), n in sorted(na.items()) if gg == g))
if "--fails" in sys.argv:
    w = sys.argv[sys.argv.index("--fails") + 1]
    for g, desc, state, inp, want, o in fails:
        if w in ("all", g): print(f"\n[{g}] {desc} ({state})\n  input {inp!r}\n  want  {want!r}\n  got   {o[:200]}")
