#!/usr/bin/env python3
# Card make-background: `play "song.mp3"` in a go block plays on after the file's end, by design; the run then stays
# until the music ended, so a ctrl-c (or Sublime's cancel: SIGTERM to the build's process group) stops warp and the
# player together, instead of leaving an orphaned player nothing can stop.
# Usage: python3 probes/music_outlives_file.py <warp binary>
# Silent: a stand-in `afplay` (sleep) first on PATH plays a generated silent WAV, so nothing is heard.
import os, signal, subprocess, sys, time, wave

WARP = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else "warp")
FOLDER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "scratch", "music_outlives_file")
PLAYER_SECONDS = 30
SECONDS_TO_STOP = 3
PROGRAM = 'go { play("silence.wav") }\nprint "main done"\n'


def prepare():
	os.makedirs(FOLDER, exist_ok=True)
	player = os.path.join(FOLDER, "afplay")
	with open(player, "w") as script:
		script.write(f"#!/bin/sh\nexec sleep {PLAYER_SECONDS}\n")
	os.chmod(player, 0o755)
	with wave.open(os.path.join(FOLDER, "silence.wav"), "wb") as silence:
		silence.setnchannels(1)
		silence.setsampwidth(2)
		silence.setframerate(8000)
		silence.writeframes(b"\0\0" * 8000)
	with open(os.path.join(FOLDER, "program.warp"), "w") as program:
		program.write(PROGRAM)


def group_members(group):
	listed = subprocess.run(["pgrep", "-g", str(group)], capture_output=True, text=True).stdout.split()
	return [int(pid) for pid in listed]


def started(stop_signal):
	"""The run stays after the file's end while the player plays, and `stop_signal` to its group ends both"""
	environment = {key: value for key, value in os.environ.items() if key not in ("WARP_NO_WINDOW", "CI")}
	environment["PATH"] = FOLDER + os.pathsep + environment["PATH"]
	run = subprocess.Popen([WARP, "run", "program.warp"], cwd=FOLDER, env=environment, stdin=subprocess.DEVNULL,
		stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
	try:
		assert b"main done" in run.stdout.readline(), "the file ran to its end"
		time.sleep(1)
		assert run.poll() is None, "the run stays while the music plays"
		os.killpg(run.pid, stop_signal)
		run.wait(SECONDS_TO_STOP)
		time.sleep(0.5)
		assert not group_members(run.pid), f"the player outlived {signal.Signals(stop_signal).name}"
	finally:
		for pid in group_members(run.pid):
			os.kill(pid, signal.SIGKILL)


prepare()
started(signal.SIGINT)
started(signal.SIGTERM)
print("ok: music outlives the file, ctrl-c and SIGTERM stop warp and the player")
