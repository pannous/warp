#!/usr/bin/env python3
"""The warp programs of the tests a survey found disagreeing (lines `module::file::test<TAB>message`), each run again
with the self-hosted analysis check (stage_check.sh's switches) by the checkout's own warp: which still disagree.
Usage: probes/self_hosting/analyzer/disagreement_codes.py <survey.txt> [--limit N] [--grep text]
Writes data/self_analyzer/still_disagreeing.txt (code<TAB>disagreement)."""
import os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

WARP = "scratch/warp"
JOBS = 4
OUT = "data/self_analyzer/still_disagreeing.txt"
SWITCHES = {"WARP_NO_WINDOW": "1", "WARP_HINTS": "0", "WARP_SELF_HOSTED": "1", "WARP_SELF_ANALYSIS": "1"}
# the first string argument of a test macro or eval call
CODE = re.compile(r'(?:is!|eval|fails_with|assert_run!?|run)\(\s*"((?:[^"\\]|\\.)*)"')


def option(name, default):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else default


def test_body(path, name):
    source = open(path, encoding="utf-8").read()
    start = source.find(f"fn {name}(")
    if start < 0:
        return ""
    end = source.find("\nfn ", start + 1)
    return source[start:end if end > 0 else len(source)]


RUST_ESCAPES = {"n": "\n", "t": "\t", '"': '"', "\\": "\\", "'": "'", "0": "\0", "r": "\r"}


def unescaped(raw):
    return re.sub(r"\\(.)", lambda m: RUST_ESCAPES.get(m[1], "\\" + m[1]), raw)


def codes(survey):
    seen = set()
    for line in open(survey, encoding="utf-8", errors="replace"):
        test = line.split("\t")[0].strip()
        parts = test.split("::")
        if len(parts) < 3:
            continue
        path = os.path.join("tests", *parts[:-1]) + ".rs"
        if not os.path.exists(path):
            continue
        for raw in CODE.findall(test_body(path, parts[-1])):
            code = unescaped(raw)
            if code not in seen:
                seen.add(code)
                yield code


def checked(code):
    try:
        run = subprocess.run([WARP, code], capture_output=True, text=True, timeout=60, env={**os.environ, **SWITCHES})
    except subprocess.TimeoutExpired:
        return code, ""
    found = re.search(r"disagrees with Rust's: ([^\"]*)", run.stdout + run.stderr)
    return code, found[1] if found else ""


def main():
    wanted = option("--grep", "")
    cases = [c for c in codes(sys.argv[1]) if wanted in c][: int(option("--limit", 10**9))]
    with ThreadPoolExecutor(JOBS) as pool:
        results = list(pool.map(checked, cases))
    differing = [(code, why) for code, why in results if why]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as out:
        for code, why in differing:
            out.write(code.replace("\n", "⏎") + "\t" + why + "\n")
    print(f"{len(differing)} of {len(cases)} programs disagree; {OUT}")


main()
