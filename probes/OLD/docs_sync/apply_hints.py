#!/usr/bin/env python3
"""Rewrites a warp file in the form its hints prefer: `hint L:C: prefer A over B` replaces B by A at line L, column C.
usage: apply_hints.py <file.warp> [skip-pattern ...]   (hints whose text matches a skip pattern stay)"""
import re, subprocess, sys, os
WARP = os.environ.get("WARP", "/Users/me/dev/bin/warp")
HINT = re.compile(r"^hint (\d+):(\d+): prefer (.+) over (.+)$")

def hints(path):
    run = subprocess.run([WARP, "--no-ask", path], capture_output=True, text=True, timeout=60, stdin=subprocess.DEVNULL,
                         env={**os.environ, "WARP_NO_WINDOW": "1"})
    return [HINT.match(line).groups() for line in (run.stdout + run.stderr).splitlines() if HINT.match(line)]

def apply(path, skips):
    lines = open(path).read().split("\n")
    misses = []
    # right to left so earlier columns on a line stay valid
    for line, column, preferred, written in sorted(set(hints(path)), key=lambda h: (int(h[0]), -int(h[1]))):
        if any(re.search(skip, preferred + " " + written) for skip in skips):
            continue
        text = lines[int(line) - 1]
        start = int(column) - 1
        at = text.find(written, start) if text.find(written, start) == start else text.find(written)
        if at < 0:
            misses.append(f"{line}:{column} prefer {preferred} over {written}")
            continue
        lines[int(line) - 1] = text[:at] + preferred + text[at + len(written):]
    open(path, "w").write("\n".join(lines))
    for miss in misses:
        print(f"{path}: not applied: {miss}")

apply(sys.argv[1], sys.argv[2:])
