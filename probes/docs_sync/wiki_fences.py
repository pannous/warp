#!/usr/bin/env python3
"""Runs every warp, wasp and untagged code fence of the wiki and records how each ends.

usage: wiki_fences.py [page.md …]   (default: every wiki/*.md)
Writes data/docs_sync/fences.tsv: page, fence line, status (ok / error / timeout), the first error line.
Planned fences (```warp planned) are skipped: they must name a card instead (listed as status planned).
"""
import os, re, subprocess, sys, pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
WIKI = REPO / "wiki"
WARP = os.environ.get("WARP", str(REPO / "scratch/docs_sync/warp"))
RUN_DIR = REPO / "scratch/docs_sync/run"
REPORT = REPO / "data/docs_sync/fences.tsv"
RUN_TAGS = {"warp", "wasp", ""}
TIMEOUT_SECONDS = 10
ENVIRONMENT = dict(os.environ, WARP_NO_WINDOW="1", WARP_CRASH_CARDS="0", WARP_HINTS="0",
                   WARP_SERVE_ADDRESS="127.0.0.2")
FENCE = re.compile(r"^```(\S*)\s*(\S*)")
ERROR = re.compile(r"Error|error|panicked|not a function|undefined")


def fences(page):
    """(line number, tag, extra word, code) of each fence"""
    lines = page.read_text().splitlines()
    found, start, tag, extra, body = [], None, "", "", []
    for number, line in enumerate(lines, 1):
        if re.match(r"^```.+```", line):  # inline code written with triple backticks
            continue
        match = FENCE.match(line)
        if start is None and match:
            start, tag, extra, body = number, match.group(1), match.group(2), []
        elif start is not None and line.startswith("```"):
            found.append((start, tag, extra, "\n".join(body)))
            start = None
        elif start is not None:
            body.append(line)
    return found


def run(code):
    program = RUN_DIR / "fence.warp"
    program.write_text(code + "\n")
    try:
        result = subprocess.run([WARP, "--no-ask", str(program)], cwd=RUN_DIR, env=ENVIRONMENT,
                                capture_output=True, text=True, timeout=TIMEOUT_SECONDS, stdin=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        return "timeout", ""
    output = (result.stdout + result.stderr).strip()
    (RUN_DIR / "last.txt").write_text(result.stdout)
    failure = next((line for line in output.splitlines() if ERROR.search(line)), "")
    # the exit status is the program's Int result, so only the output tells a failure
    return ("error" if failure else "ok"), failure[:200]


def show(places):
    """--show page.md:line …: print those fences with the output of a run"""
    for place in places:
        name, line = place.split(":")
        code = next(code for start, _, _, code in fences(WIKI / name) if start == int(line))
        program = RUN_DIR / "fence.warp"
        program.write_text(code + "\n")
        result = subprocess.run([WARP, "--no-ask", str(program)], cwd=RUN_DIR, env=ENVIRONMENT, capture_output=True,
                                text=True, timeout=TIMEOUT_SECONDS, stdin=subprocess.DEVNULL)
        print(f"===== {place}\n{code}\n----- run:\n{(result.stdout + result.stderr).strip()[:300]}\n")


VALUE_COMMENT = re.compile(r"^(?P<code>[^/#]*?\S)\s+(//|#)\s*(?P<value>[^ ].{0,24})$")
NOT_A_VALUE = re.compile(r"\b(error|warning|hint|same|like|not|is|the|a|an|ok|fine|works|e\.g\.|with|or|and)\b|:|…|\.\.\.")


def values(pages):
    """--values: each `code // value` line of a fence that runs, run after the fence's lines above it"""
    rows = []
    for page in pages:
        earlier = []  # the page's earlier fences: a case or class defined above
        for start, tag, extra, code in fences(page):
            if tag not in {"warp", "wasp"} or extra == "planned":
                continue
            lines = code.splitlines()
            for offset, line in enumerate(lines):
                match = VALUE_COMMENT.match(line)
                if not match or NOT_A_VALUE.search(match["value"]):
                    continue
                program = "\n".join(lines[:offset] + [match["code"]])
                status, failure = run("\n".join(earlier + [program]))
                if status == "error":
                    status, failure = run(program)
                output = (RUN_DIR / "last.txt").read_text().strip().splitlines()
                got = output[-1].removeprefix("» ").strip() if output else ""
                expected = match["value"].strip()
                if status == "error" or got.strip('"\'') != expected.strip('"\''):
                    rows.append(f"{page.name}\t{start + offset + 1}\t{match['code'].strip()}\t{expected}\t{failure or got}")
                    print(rows[-1], flush=True)
            if run(code)[0] == "ok":
                earlier.append(code)
    (REPORT.parent / "values.tsv").write_text("\n".join(rows) + "\n")


def main():
    if sys.argv[1:2] == ["--show"]:
        return show(sys.argv[2:])
    if sys.argv[1:2] == ["--values"]:
        return values([pathlib.Path(p) for p in sys.argv[2:]] or sorted(WIKI.glob("*.md")))
    RUN_DIR.mkdir(parents=True, exist_ok=True)
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    pages = [pathlib.Path(p) for p in sys.argv[1:]] or sorted(WIKI.glob("*.md"))
    rows = []
    for page in pages:
        earlier = []  # a fence may continue the page's earlier ones (a class defined above)
        for line, tag, extra, code in fences(page):
            if tag not in RUN_TAGS or not code.strip():
                continue
            status, failure = ("planned", "") if extra == "planned" else run(code)
            if status == "error" and earlier:
                continued, _ = run("\n".join(earlier + [code]))
                if continued == "ok":
                    status, failure = "ok", "(with the earlier fences)"
            if status == "ok":
                earlier.append(code)
            rows.append(f"{page.name}\t{line}\t{status}\t{failure}")
            print(rows[-1], flush=True)
    REPORT.write_text("\n".join(rows) + "\n")


main()
