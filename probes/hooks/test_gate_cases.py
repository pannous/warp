#!/usr/bin/env python3
"""Outcome probe for .claude/hooks/test-gate.py: which Bash commands it blocks (exit 2) and which pass (exit 0)."""
import json
import os
import subprocess
import sys

HOOK = os.path.join(os.path.dirname(__file__), "..", "..", ".claude", "hooks", "test-gate.py")
RUST = "cargo" + " test"
CASES = [
    ("tests/queue.sh -- parser::", 2),
    ("./test.sh", 2),
    (f"{RUST} foo", 2),
    ("cd x && tests/queue.sh -- y", 2),
    ("MAC_ONLY_TESTS=1 tests/queue.sh -- sound::", 0),
    (f"{RUST} --no-run", 0),
    ("scripts/cloud-test.sh --card x b 1 parser::", 0),
    ("WARP_LIB=lib /Users/me/dev/bin/warp test samples/hello/hello.warp", 0),
    ("scripts/is_tests.py run pure-only", 0),
    ("git log -- tests/queue.sh", 0),
]


def verdict(command: str) -> int:
    payload = json.dumps({"tool_input": {"command": command}})
    environment = {key: value for key, value in os.environ.items() if key != "CLAUDE_CODE_REMOTE"}
    return subprocess.run([sys.executable, HOOK], input=payload, text=True, capture_output=True, env=environment).returncode


failures = [(command, expected, verdict(command)) for command, expected in CASES if verdict(command) != expected]
for command, expected, got in failures:
    print(f"✗ {command}: expected {expected}, got {got}")
print(f"{len(CASES) - len(failures)}/{len(CASES)} cases right")
sys.exit(1 if failures else 0)
