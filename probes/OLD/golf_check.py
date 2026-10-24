#!/usr/bin/env python3
"""Run every solution of samples/golf/ (<hole>.warp readable, <hole>.short.warp golfed) against <hole>.txt, the hole's
expected output (a quine's is its own source), called with the lines of <hole>.args as arguments, and print a table of pass/fail and char counts (code.golf counts chars and trims trailing whitespace)
Usage: probes/golf_check.py [hole …]"""
import subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOLF = ROOT / "samples" / "golf"
WARP = ROOT / "scratch" / "warp"
TIMEOUT_SECONDS = 120
VARIANTS = ["", ".short"]


def trimmed(text):
	return "\n".join(line.rstrip() for line in text.rstrip().split("\n"))


def chars(path, is_quine):
	"""a quine's final line break is part of its output, so it counts"""
	text = path.read_text()
	return len(text if is_quine else text.rstrip("\n"))


def outcome(solution, expected, arguments):
	if not solution.exists():
		return "—"
	is_quine = expected is None
	expected = solution.read_text() if is_quine else expected
	try:
		run = subprocess.run([WARP, "run", "--no-ask", "--no-hints", solution, *arguments], capture_output=True, text=True, timeout=TIMEOUT_SECONDS)
	except subprocess.TimeoutExpired:
		return "timeout"
	if (run.stdout == expected) if is_quine else (trimmed(run.stdout) == trimmed(expected)):
		return f"{chars(solution, is_quine)}"
	first_difference = next((f"line {n + 1}: {got!r} ≠ {want!r}" for n, (got, want) in
		enumerate(zip(trimmed(run.stdout).split("\n") + [""] * 999, trimmed(expected).split("\n"))) if got != want), "?")
	return f"FAIL {first_difference} {run.stderr.strip()[:200]}"


def main(holes):
	holes = holes or sorted({path.name.split(".")[0] for path in GOLF.glob("*.warp")})
	for hole in holes:
		expected = (GOLF / f"{hole}.txt").read_text() if (GOLF / f"{hole}.txt").exists() else None
		arguments = (GOLF / f"{hole}.args").read_text().splitlines() if (GOLF / f"{hole}.args").exists() else []
		print(f"{hole}: " + " | ".join(outcome(GOLF / f"{hole}{variant}.warp", expected, arguments) for variant in VARIANTS), flush=True)


if __name__ == "__main__":
	main(sys.argv[1:])
