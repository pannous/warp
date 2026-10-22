# Runs each is!("code", …) case of the given test files through the Rust lane and WARP_SELF_HOSTED=front
# (parser.warp of WARP_LIB) and prints the cases whose values differ (cards parser-star-params, parser-colon-forms)
import json, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

WARP = "/Users/me/dev/bin/warp"
CASE = re.compile(r'is!\("((?:[^"\\]|\\.)*)"')

def run(code, lane):
    env = dict(os.environ, WARP_NO_WINDOW="1", WARP_HINTS="0")
    if lane: env["WARP_SELF_HOSTED"] = lane
    out = subprocess.run([WARP, "--no-ask", code], env=env, capture_output=True, text=True, timeout=300)
    lines = [l for l in (out.stdout + out.stderr).splitlines() if l.strip()]
    return lines[-1] if lines else ""

def compared(code):
    return code, run(code, None), run(code, "front")

codes = [json.loads('"' + m + '"') for f in sys.argv[1:] for m in CASE.findall(open(f).read())]
with ThreadPoolExecutor(4) as pool:
    results = list(pool.map(compared, codes))
differ = [r for r in results if r[1] != r[2]]
for code, rust, front in differ: print(f"{code}\n  rust:  {rust}\n  front: {front}")
print(f"{len(differ)} of {len(results)} differ")
