#!/usr/bin/env python3
"""Runs every example of wiki/keyword.md that shows a result (`code` → `result`) and reports the ones whose output differs.

usage: probes/keyword_examples.py [warp binary] [keyword.md]   (defaults: scratch/warp, ~/dev/angles/warp/wiki/keyword.md)
"""
import re
import subprocess
import sys
from pathlib import Path

WARP = sys.argv[1] if len(sys.argv) > 1 else "scratch/warp"
PAGE = Path(sys.argv[2] if len(sys.argv) > 2 else "~/dev/angles/warp/wiki/keyword.md").expanduser()
EXAMPLE = re.compile(r"`([^`]+)` → `([^`]+)`")
RESULT_MARK = "» "


def result_of(code):
	output = subprocess.run([WARP, "--no-ask", code], capture_output=True, text=True, timeout=60).stdout
	results = [line[len(RESULT_MARK):].strip() for line in output.splitlines() if line.startswith(RESULT_MARK)]
	return results[-1] if results else output.strip()


def main():
	examples = EXAMPLE.findall(PAGE.read_text())
	failures = [(code, expected, actual) for code, expected in examples if (actual := result_of(code)) != expected]
	for code, expected, actual in failures:
		print(f"FAIL {code}\n  expected {expected}\n  got      {actual}")
	print(f"{len(examples) - len(failures)}/{len(examples)} examples give their result")
	sys.exit(1 if failures else 0)


if __name__ == "__main__":
	main()
