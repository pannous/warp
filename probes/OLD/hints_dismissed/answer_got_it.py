# Runs warp on a terminal (pty) and answers the first "got it?" with argv[1]: `python3 answer_got_it.py a warp prog.warp`
import os, pty, re, sys

answer, command = sys.argv[1], sys.argv[2:]
pid, terminal = pty.fork()
if pid == 0:
	os.execvp(command[0], command)
output, answered = b"", False
while True:
	try:
		chunk = os.read(terminal, 4096)
	except OSError:
		break
	if not chunk:
		break
	output += chunk
	if not answered and b"got it?" in output:
		os.write(terminal, (answer + "\n").encode())
		answered = True
os.waitpid(pid, 0)
print(re.sub(r"\x1b\[[0-9;]*m", "", output.decode(errors="replace")).replace("\r", ""))
