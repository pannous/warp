#!/usr/bin/env python3
"""PreToolUse hook: Rust tests run in the cloud (user 2026-10-20), never on this Mac.

Blocks a Bash command with a segment that starts a test run: `cargo … test`, `cargo browser-test`, a compiled test
binary (`…/deps/tests-<hash>`), `tests/queue.sh` or `./test.sh`. `cargo test --no-run` (a build) passes, and so does
a segment marked MAC_ONLY_TESTS=1 (the tests in scripts/cloud-test-environmental.txt, which need the Mac).
Exit 2 = block, the message goes back to the agent.
Cloud sessions (CLAUDE_CODE_REMOTE=true) are exempt: their VM is not shared, so there is no machine to protect."""
import json
import os
import re
import sys

SEGMENT_SEPARATORS = re.compile(r"&&|\|\||[;|\n]|\$\(|`")
DIRECT_TEST_RUN = re.compile(r"^(?:\S+=\S+\s+)*(?:timeout\s+\d+\s+|nice\s+(?:-n\s*\d+\s+)?|env\s+)*"
                             r"(?:cargo(?:\s+\+\S+)?(?:\s+--?\S+)*\s+(?:test|browser-test)\b|\S*/deps/tests-[0-9a-f]+\b"
                             r"|\S*tests/queue\.sh\b|(?:\S*/)?test\.sh\b)")
BUILD_ONLY = "--no-run"
MAC_ONLY = "MAC_ONLY_TESTS=1"
FIX = ("Rust tests run in the cloud (user 2026-10-20): scripts/cloud-test.sh --card <card> <branch> <tip> <filters>. "
       "Only the Mac-only tests of scripts/cloud-test-environmental.txt run here: MAC_ONLY_TESTS=1 tests/queue.sh -- <filter>")


def direct_test_run(command: str) -> str | None:
    for segment in SEGMENT_SEPARATORS.split(command):
        segment = segment.strip().lstrip("(").strip()
        if DIRECT_TEST_RUN.match(segment) and BUILD_ONLY not in segment and MAC_ONLY not in segment:
            return segment
    return None


def main() -> int:
    if os.environ.get("CLAUDE_CODE_REMOTE") == "true":
        return 0
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
