#!/usr/bin/env python3
"""Check that the real interactive console remembers an assigned variable."""
import os
import pty
import select
import sys
import time

WARP = sys.argv[1] if len(sys.argv) > 1 else "warp"
PROMPT = "🌀".encode()
RESULT = "» 7".encode()
TIMEOUT_SECONDS = 10


def read_until(fd, wanted):
	seen = b""
	deadline = time.time() + TIMEOUT_SECONDS
	while time.time() < deadline and wanted not in seen:
		if select.select([fd], [], [], 0.1)[0]:
			seen += os.read(fd, 4096)
	return seen


def enter(fd, source):
	os.write(fd, source.encode() + b"\r")
	return read_until(fd, PROMPT)


pid, fd = pty.fork()
if pid == 0:
	os.execvp(WARP, [WARP, "console"])
read_until(fd, PROMPT)
assert RESULT in enter(fd, "a=7"), "assignment returns its value"
answer = enter(fd, "a")
assert RESULT in answer, f"the next console line reads the assigned value: {answer!r}"
enter(fd, "quit")
_, status = os.waitpid(pid, 0)
assert os.waitstatus_to_exitcode(status) == 0, "console exits successfully"
print("ok: console remembers assigned variables")
