#!/usr/bin/env python3
"""Runs every warp fence of web/playground/primer.md with scratch/warp and shows expected vs echoed value."""
import os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PRIMER = os.path.join(ROOT, "web/playground/primer.md")
WARP = os.path.join(ROOT, "scratch/warp")
FENCE = re.compile(r"```warp([^\n]*)\n(.*?)```", re.S)

def run(code):
    path = os.path.join(ROOT, "scratch/primer_snippet.warp")
    with open(path, "w") as f:
        f.write(code)
    env = dict(os.environ, WARP_NO_WINDOW="1")
    done = subprocess.run([WARP, path], capture_output=True, text=True, env=env, timeout=60)
    return (done.stdout + done.stderr).strip()

only = sys.argv[1] if len(sys.argv) > 1 else None
for number, (head, code) in enumerate(FENCE.findall(open(PRIMER).read()), 1):
    if "compiles" in head:  # servers serve and paint opens windows: tests/web/test_primer.rs compiles them
        continue
    if only and str(number) != only:
        continue
    print(f"--- #{number} {head.strip()}\n{code.strip()}\n>>> {run(code)}\n")
