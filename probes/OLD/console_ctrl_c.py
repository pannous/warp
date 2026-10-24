#!/usr/bin/env python3
# Card interactive-console: in `warp console` a ctrl-c twice in a row ends the console, once only clears the line.
# Usage: python3 probes/console_ctrl_c.py <warp binary>   (a pseudo-terminal: rustyline reads ctrl-c as a key there)
import os, pty, select, sys, time

WARP = sys.argv[1] if len(sys.argv) > 1 else "warp"
CTRL_C = b"\x03"
TIMEOUT_SECONDS = 10
PROMPT = "🌀".encode()


def read_until(fd, wanted, seconds=TIMEOUT_SECONDS):
	seen = b""
	deadline = time.time() + seconds
	while time.time() < deadline and wanted not in seen:
		if select.select([fd], [], [], 0.1)[0]:
			try:
				seen += os.read(fd, 4096)
			except OSError:
				break
	return seen


def typed(keys):
	"""The console's answer up to its next prompt: a key sent before that prompt would reach the terminal outside the
	line editor, where ctrl-c is a signal that ends the process"""
	os.write(fd, keys)
	return read_until(fd, PROMPT)


def exited(pid, seconds=TIMEOUT_SECONDS):
	"""The exit code once the console ended within `seconds`, else None; reads its output meanwhile"""
	deadline = time.time() + seconds
	while time.time() < deadline:
		finished, status = os.waitpid(pid, os.WNOHANG)
		if finished:
			return os.waitstatus_to_exitcode(status)
		read_until(fd, b"\0", 0.1)
	return None


pid, fd = pty.fork()
if pid == 0:
	os.execvp(WARP, [WARP, "console"])
read_until(fd, PROMPT)
assert b"3" in typed(b"1+2\r"), "the console evaluates"
assert b"again" in typed(CTRL_C), "one ctrl-c clears the line"
assert b"4" in typed(b"2+2\r"), "the console goes on after one ctrl-c"
typed(CTRL_C)
os.write(fd, CTRL_C)
code = exited(pid)
assert code == 0, f"two ctrl-c in a row end the console (exit {code})"
print("ok: one ctrl-c keeps the console, two end it")
