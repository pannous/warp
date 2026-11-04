#!/usr/bin/env python3
# WHATWG URL tests (web-platform-tests url/resources/urltestdata.json, vendored here) against lib/url; not part of smoke.
# usage: WARP_LIB=$PWD/lib test/url/url_test.py [--fails GROUP|all]
# lib/url is an RFC 3986 parser (parse_url, resolve_url): see README.md for the selection rule and the skipped
# categories. A case without a base runs url_text(parse_url(input)); with one, resolve_url(base, input). href and the
# parts are compared: protocol = scheme + ":", username = user, password, hostname = host ([…] for IPv6),
# port, host = hostname[:port], pathname = path, search = "?" + query, hash = "#" + fragment (both "" when empty).
# A failure case passes when lib raises.
import json, os, re, subprocess, sys, tempfile, collections
here = os.path.dirname(os.path.abspath(__file__))
data = [c for c in json.load(open(os.path.join(here, "urltestdata.json"), encoding="utf-8")) if isinstance(c, dict)]
SPECIAL = ("http", "https", "ws", "wss", "ftp")
def scheme_of(t):
    m = re.match(r"^([A-Za-z][A-Za-z0-9+.-]*):", t or ""); return m.group(1).lower() if m else None
def skip_reason(c):
    i, b = c["input"], c.get("base") or ""
    s = scheme_of(i) or scheme_of(b)
    if any(ord(ch) < 0x21 or ord(ch) == 0x7F for ch in i): return "whitespace and control characters (stripped or encoded)"
    if "\\" in i: return "backslash as a slash"
    if any(ord(ch) > 0x7E for ch in i + b): return "non-ASCII (IDNA, percent-encoding)"
    if s == "file" or scheme_of(i) == "file": return "file: quirks"
    if s not in SPECIAL: return "non-special schemes"
    h = c.get("hostname")
    if h is not None and re.fullmatch(r"[0-9a-fA-FxX.]+", h or "x") and re.search(r"(^|\.)(0x|0\d)|^\d+$|^[0-9.]+$", re.sub(r"^.*?//(?:[^@/]*@)?([^/:?#]*).*$", r"\1", i)) and not re.fullmatch(r"(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}", re.sub(r"^.*?//(?:[^@/]*@)?([^/:?#]*).*$", r"\1", i)):
        return "IPv4 number forms"
    return None
W = lambda s: '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'
PRELUDE = '''use url
use json
parts(href: text): text := {
	u = parse_url(href)
	host = if u.host == nil then "" else if u.host.contains(":") then "[" + u.host + "]" else u.host
	port = if u.port == nil then "" else "" + u.port
	to_json({"href": href, "protocol": if u.scheme == nil then "" else u.scheme + ":", "username": u.user ?? "", "password": u.password ?? "", "hostname": host, "port": port, "host": if port == "" then host else host + ":" + port, "pathname": u.path, "search": if u.query == nil or u.query == "" then "" else "?" + u.query, "hash": if u.fragment == nil or u.fragment == "" then "" else "#" + u.fragment})
}
run(input: text): text := try { "OK " + parts(url_text(parse_url(input))) } catch e { "ERR " + e.message }
run(base: text, input: text): text := try { "OK " + parts(resolve_url(base, input)) } catch e { "ERR " + e.message }
'''
# by design: lib/url keeps a URL as written (RFC 3986, url_text(parse_url(t)) is t) where WHATWG normalizes. A failing
# case is "by design" when one of WHATWG's normalizations, applied to lib's own parts, gives the expected ones.
DEFAULT_PORT = {"http": "80", "https": "443", "ws": "80", "wss": "443", "ftp": "21"}
def pct(t, extra):
    return "".join("%%%02X" % ord(ch) if ch in extra else ch for ch in t)
def remove_dots(path):
    out = []
    segs = path.split("/")
    for k, seg in enumerate(segs[1:], 1):
        last = k == len(segs) - 1
        if seg in (".", "%2e", "%2E"):
            if last: out.append("")
        elif seg in ("..", ".%2e", "%2e.", "%2e%2e", ".%2E", "%2E.", "%2E%2E"):
            if out: out.pop()
            if last: out.append("")
        else: out.append(seg)
    return "/" + "/".join(out) if path.startswith("/") else path
def ipv6_canon(h):
    import ipaddress
    try: return "[" + ipaddress.IPv6Address(h[1:-1]).compressed + "]" if h.startswith("[") else h
    except ValueError: return h
STEPS = [
    ("default port dropped", lambda g: {**g, "port": "" if g["port"].lstrip("0") == DEFAULT_PORT.get(g["protocol"][:-1], "?") else g["port"]}),
    ("empty special path is /", lambda g: {**g, "pathname": g["pathname"] or "/"}),
    ("dot segments removed", lambda g: {**g, "pathname": remove_dots(g["pathname"])}),
    ("percent-encoding (WHATWG encode sets)", lambda g: {**g, "username": pct(g["username"], " \"#<>?`{}/:;=@[]^|"), "password": pct(g["password"], " \"#<>?`{}/:;=@[]^|"), "pathname": pct(g["pathname"], " \"#<>?`{}"), "search": g["search"][:1] + pct(g["search"][1:], " \"#<>'"), "hash": g["hash"][:1] + pct(g["hash"][1:], " \"<>`")}),
    ("host lowercased", lambda g: {**g, "hostname": g["hostname"].lower()}),
    ("IPv6 canonical form", lambda g: {**g, "hostname": ipv6_canon(g["hostname"])}),
    ("port with leading zeros", lambda g: {**g, "port": g["port"].lstrip("0") or ("0" if g["port"] else "")}),
]
PARTS = ["protocol", "username", "password", "hostname", "port", "pathname", "search", "hash"]
def by_design(c, got):
    g = {f: got[f] for f in PARTS}
    applied = []
    for name, step in STEPS:
        h = step(g)
        if h != g: applied.append(name); g = h
        if all(g[f] == c[f] for f in PARTS if f in c): return applied[-1] if applied else "href only"
    i = c["input"]
    if "%" in g["hostname"]: return "host percent-decoded, then IDNA"
    if re.match(r"^/{3,}", i) or re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:/{3,}", i): return "special scheme: /// read as an authority"
    if re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:(?!//)", i) and scheme_of(i) in SPECIAL: return "special scheme without // (WHATWG reads an authority or a relative URL)"
    return None
tot, ok, skipped, fails = collections.Counter(), collections.Counter(), collections.Counter(), []
design = collections.Counter()
run = []
for c in data:
    r = skip_reason(c)
    if r: skipped[r] += 1; continue
    run.append(c)
prog = PRELUDE + "".join('print(run(%s))\nprint("@@")\n' % (W(c["input"]) if not c.get("base") else W(c["base"]) + ", " + W(c["input"])) for c in run)
with tempfile.NamedTemporaryFile("w", suffix=".warp", delete=False, encoding="utf-8") as fh: fh.write(prog)
outs = [o.strip() for o in subprocess.run(["warp", fh.name], capture_output=True, text=True, errors="replace").stdout.split("@@\n")]
FIELDS = ["href", "protocol", "username", "password", "hostname", "port", "host", "pathname", "search", "hash"]
for k, c in enumerate(run):
    o = outs[k] if k < len(outs) else "CRASH"
    group = "failure" if c.get("failure") else ("with base" if c.get("base") else "no base")
    reason = None
    if c.get("failure"):
        good, why = o.startswith("ERR"), "no failure"
        if not good: reason = "failure case: parse_url does not validate (documented: it gives the parts)"
    else:
        if not o.startswith("OK "): good, why = False, "raised"
        else:
            got = json.loads(o[3:]); bad = [f for f in FIELDS if f in c and got[f] != c[f]]
            good, why = not bad, ",".join(bad[1:2] or bad[:1])
            if not good: reason = by_design(c, got)
    tot[group] += 1; ok[group] += good
    if reason: design[reason] += 1
    elif not good: fails.append((group, why, c, o))
print(f"url: {sum(ok.values())}/{sum(tot.values())} run ({sum(ok.values()) + sum(design.values())} counting by design), {sum(skipped.values())} skipped, of {len(data)}")
for g in ("no base", "with base", "failure"): print(f"  {ok[g]:4}/{tot[g]:<4} {g}")
print("skipped by category:")
for r, n in skipped.most_common(): print(f"  {n:4} {r}")
print(f"by design, {sum(design.values())} (WHATWG normalizes, lib/url keeps the text as written):")
for r, n in design.most_common(): print(f"  {n:4} {r}")
print(f"other failures, {len(fails)}, by first differing part:")
for w, n in collections.Counter(f"{g}: {why}" for g, why, _, _ in fails).most_common(): print(f"  {n:4} {w}")
if "--fails" in sys.argv:
    w = sys.argv[sys.argv.index("--fails") + 1]
    for g, why, c, o in fails:
        if w in ("all", g, why):
            print(f"\n[{g}: {why}] input {c['input']!r} base {c.get('base')!r}\n  want {c.get('href', 'failure')!r}\n  got  {o[:240]}")
