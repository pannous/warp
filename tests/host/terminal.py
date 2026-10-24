#!/usr/bin/env python3
"""Drive a command on a pseudo-terminal as a person would (tests/host/test_repl_terminal.sh): each argument after --
is typed as one line once the output has gone quiet; `^D` types Ctrl+D, `^[A` the up arrow. Prints all the command
wrote, exits with its status.
  terminal.py [--quiet seconds] -- <command…> -- <typed…>"""
import os, pty, select, sys, time

args = sys.argv[1:]
quiet = 1.0
if args[:1] == ["--quiet"]:
	quiet, args = float(args[1]), args[2:]
split = args.index("--", 1)
command, typed = args[1:split], args[split + 1:]
pid, fd = pty.fork()
if pid == 0:
	os.execvp(command[0], command)
said = b""


def read_until_quiet(limit=120.0):
	"""Read what the command writes until it says nothing for `quiet` seconds (or ends)"""
	global said
	start = time.time()
	while time.time() - start < limit:
		ready, _, _ = select.select([fd], [], [], quiet)
		if not ready:
			return True
		try:
			chunk = os.read(fd, 4096)
		except OSError:
			return False
		if not chunk:
			return False
		said += chunk
		sys.stdout.write(chunk.decode(errors="replace").replace("\r\n", "\n"))
		sys.stdout.flush()
	return True


read_until_quiet()
for line in typed:
	keys = line.replace("^[A", "\x1b[A").replace("^D", "\x04")
	os.write(fd, keys.encode() + (b"" if keys.endswith("\x04") or keys.endswith("\x1b[A") else b"\r"))
	if not read_until_quiet():
		break
read_until_quiet(5)
_, status = os.waitpid(pid, 0)
sys.exit(os.waitstatus_to_exitcode(status))
