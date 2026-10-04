#!/usr/bin/env python3
"""Run the `code` → `value` examples of wiki pages through `warp eval` and list those whose result differs.
Usage: probes/wiki_examples.py [wiki/Footguns.md …]   (default: every wiki page)"""
import glob
import re
import subprocess
import sys

WARP = "/Users/me/.cargo/shared-target.noindex/debug/warp"
EXAMPLE = re.compile(r"`([^`]+)`\s*→\s*`([^`]+)`")
TIMEOUT_SECONDS = 20
NEWLINE_MARK = "⏎"  # the wiki writes a line break inside inline code as ⏎


def result_of(code):
    try:
        run = subprocess.run([WARP, "eval", code], capture_output=True, text=True, timeout=TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        return "TIMEOUT"
    lines = [line for line in run.stdout.splitlines() if line.startswith("»")]
    return lines[-1][1:].strip() if lines else (run.stdout + run.stderr).strip().splitlines()[-1:] or ["?"]


BOOLEANS = {"true": "1", "false": "0"}


def normalized(text):
    text = str(text).strip().replace('"', "'").replace(" ", "")
    return BOOLEANS.get(text, text.strip("'"))


def matches(got, expected):
    """Equal after normalizing quotes, spaces and booleans; an expected text with … matches the parts around it"""
    got, expected = normalized(got), normalized(expected)
    parts = [part for part in expected.split("…") if part]
    return got == expected or ("…" in expected and all(part in got for part in parts))


def main():
    pages = sys.argv[1:] or sorted(glob.glob("wiki/*.md"))
    total = differing = 0
    for page in pages:
        for line_number, line in enumerate(open(page, encoding="utf-8"), 1):
            # Footguns.md shows other languages' results too: only its warp lines are warp's
            if page.endswith("Footguns.md") and ("Warp" not in line or "Warp before" in line):
                continue
            for code, expected in EXAMPLE.findall(line):
                total += 1
                got = result_of(code.replace(NEWLINE_MARK, "\n"))
                if not matches(got, expected):
                    differing += 1
                    print(f"{page}:{line_number}: {code}  expected {expected}  got {str(got)[:120]}")
    print(f"{differing} of {total} examples differ")


if __name__ == "__main__":
    main()
