#!/usr/bin/env python3
"""Alt+←/→ move a word back and on (card alt-arrow-navigation), and ↑/↓ step through the lines typed before (card
repl-alt-arrows): each in the escape sequences macOS Terminal (Option as Meta), iTerm and xterm send, on `warp repl` in
a pseudo terminal (test_repl_keys.py drives it). Run: python3 test/native/test_repl_alt_arrows_history.py [native warp]"""
import os, sys, tempfile
import test_repl_keys as console

ALT_LEFT_META, ALT_RIGHT_META = "\x1bb", "\x1bf"
ALT_LEFT, ALT_RIGHT = "\x1b[1;3D", "\x1b[1;3C"
ALT_LEFT_ESCAPED, ALT_RIGHT_ESCAPED = "\x1b\x1b[D", "\x1b\x1b[C"
UP, DOWN, UP_APPLICATION = "\x1b[A", "\x1b[B", "\x1bOA"
console.CASES = [
    ("0 + 1" + ALT_LEFT_META * 3 + "6" + ALT_RIGHT_META + "*1", "61"),
    ("1 + 1" + ALT_LEFT * 3 + "6" + ALT_RIGHT + "*1", "62"),
    ("2 + 1" + ALT_LEFT_ESCAPED * 3 + "6" + ALT_RIGHT_ESCAPED + "*1", "63"),
    ("70+1", "71"),
    (UP, "71"),
    (UP + UP + UP, "63"),
    (UP + UP + DOWN + "+1", "64"),
    (UP_APPLICATION + "+1", "65"),
]


def history_kept(main) -> int:
    """the console's lines stay in ~/.warp_history for the next console: a second console's ↑ shows the first's last line"""
    with tempfile.TemporaryDirectory() as folder:
        os.environ["HOME"] = folder
        failures = main()
        console.CASES = [(UP, "65")]
        return failures + main()


if __name__ == "__main__":
    sys.exit(1 if history_kept(console.main) else 0)
