#!/usr/bin/env python3
# card member-completion: `xs = [1 2]` then `xs.` Tab in `warp console` lists the list's members (run with the warp binary's path)
import os, pty, select, sys, time

def read_all(fd, seconds=3.0):
	out, end = b"", time.time() + seconds
	while time.time() < end:
		ready, _, _ = select.select([fd], [], [], 0.2)
		if ready:
			try:
				out += os.read(fd, 65536)
			except OSError:
				break
	return out.decode(errors="replace")

pid, fd = pty.fork()
if pid == 0:
	os.environ["WARP_NO_WINDOW"] = "1"
	os.execv(sys.argv[1], [sys.argv[1], "repl"])
read_all(fd, 2)
os.write(fd, b"xs = [1 2]\r")
read_all(fd, 3)
os.write(fd, b"xs.\t")
shown = read_all(fd, 3)
os.write(fd, b"\x15\x04")
print(shown)
missing = [name for name in ["count", "sum", "map", "filter", "unique"] if name not in shown]
print("MISSING", missing) if missing else print("OK")
sys.exit(1 if missing else 0)
