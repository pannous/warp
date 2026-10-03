#!/usr/bin/env python3
"""PreToolUse hook: warp tests run only through tests/queue.sh (one machine-wide lock, notes/roles.md "Tester").

Blocks a Bash command with a segment that starts a test run directly: `cargo … test`, `cargo browser-test`, or a
compiled test binary (`…/deps/tests-<hash>`). `./test.sh` and `tests/queue.sh …` pass (both take the lock), and so
does `cargo test --no-run` (a build, no test runs). Exit 2 = block, the message goes back to the agent."""
import json
import re
import sys

SEGMENT_SEPARATORS = re.compile(r"&&|\|\||[;|\n]|\$\(|`")
DIRECT_TEST_RUN = re.compile(r"^(?:\S+=\S+\s+)*(?:timeout\s+\d+\s+|nice\s+(?:-n\s*\d+\s+)?|env\s+)*"
                             r"(?:cargo(?:\s+\+\S+)?(?:\s+--?\S+)*\s+(?:test|browser-test)\b|\S*/deps/tests-[0-9a-f]+\b)")
BUILD_ONLY = "--no-run"
FIX = ("warp tests go through the queue (one test binary at a time on this machine): "
       "tests/queue.sh -- <filter>  (or ./test.sh for the full suite, which only the Tester runs; see notes/roles.md)")


def direct_test_run(command: str) -> str | None:
    for segment in SEGMENT_SEPARATORS.split(command):
        segment = segment.strip().lstrip("(").strip()
        if DIRECT_TEST_RUN.match(segment) and BUILD_ONLY not in segment:
            return segment
    return None


def main() -> int:
    try:
        command = json.loads(sys.stdin.read()).get("tool_input", {}).get("command", "")
    except (ValueError, AttributeError):
        return 0
    offender = direct_test_run(command)
    if offender:
        print(f"BLOCKED: `{offender[:120]}` runs tests directly. {FIX}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
