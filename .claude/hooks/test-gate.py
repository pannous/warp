#!/usr/bin/env python3
"""PreToolUse hook: no cargo test runs on this Mac; the warp tests run with `warp test`.

Blocks a Bash command with a segment that starts a test run: `cargo … test`, `cargo browser-test`, a compiled test
binary (`…/deps/tests-<hash>`), `tests/queue.sh` or `./test.sh`. `cargo test --no-run` (a build) passes.
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
FIX = "The Rust harness has no cargo test crate on pure: run the warp tests with `warp test [folder…]`."


def direct_test_run(command: str) -> str | None:
    for segment in SEGMENT_SEPARATORS.split(command):
        segment = segment.strip().lstrip("(").strip()
        if DIRECT_TEST_RUN.match(segment) and BUILD_ONLY not in segment:
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
