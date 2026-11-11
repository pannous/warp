#!/usr/bin/env python3
"""The native console's line editing keys on a terminal (card repl-editing-keys): each line typed with the keys'
escape sequences as macOS Terminal, iTerm and xterm send them, run in `warp repl` on a pseudo terminal, its value
checked. Run: python3 test/native/test_repl_keys.py [native warp, default the warp on PATH]"""
import os, pty, re, select, shutil, sys, time

HOME, HOME_NUMBERED, HOME_APPLICATION = "\x1b[H", "\x1b[1~", "\x1bOH"
END, END_NUMBERED, END_APPLICATION = "\x1b[F", "\x1b[4~", "\x1bOF"
LEFT, DELETE = "\x1b[D", "\x1b[3~"
ALT_DELETE, ALT_D, ALT_BACKSPACE = "\x1b[3;3~", "\x1bd", "\x1b\x7f"
CTRL_A, CTRL_E, CTRL_D = "\x01", "\x05", "\x04"
# what is typed, the value the console shows for it
CASES = [
    ("50+1", "51"),
    ("+1" + HOME + "41" + END + "*1", "42"),
    ("+1" + HOME_NUMBERED + "42" + END_NUMBERED + "*1", "43"),
    ("+1" + HOME_APPLICATION + "43" + END_APPLICATION + "*1", "44"),
    ("+1" + CTRL_A + "44" + CTRL_E + "*1", "45"),
    ("4196" + LEFT + LEFT + LEFT + DELETE, "496"),
    ("foo bar 47" + HOME + ALT_DELETE + ALT_DELETE, "47"),
    ("foo 48" + HOME + ALT_D, "48"),
    ("49 foo bar" + ALT_BACKSPACE + ALT_BACKSPACE, "49"),
]
ESCAPES = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
TIMEOUT_SECONDS = 120


def console_output(warp: str) -> str:
    child, terminal = pty.fork()
    if child == 0:
        os.environ["WARP_WINDOW"] = "0"
        os.execvp(warp, [warp, "repl"])
    output = b""

    def read_for(seconds: float) -> None:
        nonlocal output
        end = time.time() + seconds
        while time.time() < end:
            ready, _, _ = select.select([terminal], [], [], 0.2)
            if ready:
                try:
                    output += os.read(terminal, 4096)
                except OSError:
                    return

    read_for(2)
    for typed, _ in CASES:
        before = output.count(b"\n")
        os.write(terminal, (typed + "\r").encode())
        end = time.time() + TIMEOUT_SECONDS
        while output.count(b"\n") < before + 2 and time.time() < end:
            read_for(0.5)
    os.write(terminal, CTRL_D.encode())
    read_for(2)
    os.waitpid(child, 0)
    return output.decode(errors="replace")


def main() -> int:
    warp = sys.argv[1] if len(sys.argv) > 1 else shutil.which("warp")
    shown = [ESCAPES.sub("", line).strip() for line in console_output(warp).replace("\r", "\n").split("\n")]
    failures = 0
    for typed, expected in CASES:
        if expected in shown:
            print(f"✓ {typed!r} shows {expected}")
            shown = shown[shown.index(expected) + 1:]
        else:
            print(f"✗ {typed!r}: expected {expected}")
            failures += 1
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
