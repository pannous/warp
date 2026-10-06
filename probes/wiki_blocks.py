#!/usr/bin/env python3
"""Run every fenced code block of the wiki pages (```, ```wasp, ```warp) as a program through `warp eval` and list the
blocks that end in an error, with the error: which documented features do not run yet.
Usage: [WARP=path/to/warp] probes/wiki_blocks.py [wiki/page.md …]   (default: every wiki page)"""
import glob
import os
import re
import subprocess
import sys

# a private copy (env WARP) keeps other worktrees' builds of the shared debug/warp out of the run
WARP = os.environ.get("WARP", "/Users/me/.cargo/shared-target.noindex/debug/warp")
FENCE = re.compile(r"^```(\w*)\s*$")
LANGUAGES = {"", "wasp", "warp", "angle"}
TIMEOUT_SECONDS = 20
# `warp eval` writes test.wasm into its working directory: run it in scratch/, never in the wiki checkout
SCRATCH = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "scratch")


def blocks(page):
    language, start, lines = None, 0, []
    for number, line in enumerate(open(page, encoding="utf-8"), 1):
        fence = FENCE.match(line.strip())
        if fence and language is None:
            language, start, lines = fence.group(1), number, []
        elif line.strip().startswith("```") and language is not None:
            if language in LANGUAGES and lines:
                yield start, "".join(lines)
            language = None
        elif language is not None:
            lines.append(line)


def outcome(code):
    try:
        run = subprocess.run([WARP, "eval", code], capture_output=True, text=True, timeout=TIMEOUT_SECONDS, cwd=SCRATCH)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    text = (run.stdout + run.stderr).strip()
    return text.splitlines()[-1] if text else ""


def main():
    os.makedirs(SCRATCH, exist_ok=True)
    pages = [os.path.abspath(page) for page in sys.argv[1:] or sorted(glob.glob("wiki/*.md"))]
    total = failing = 0
    for page in pages:
        for line, code in blocks(page):
            total += 1
            result = outcome(code)
            if "Error" in result or "panicked" in result or result == "TIMEOUT":
                failing += 1
                print(f"{page}:{line}: {result[:150]}")
    print(f"{failing} of {total} blocks end in an error")


if __name__ == "__main__":
    main()
